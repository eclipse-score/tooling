// *******************************************************************************
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// <https://www.apache.org/licenses/LICENSE-2.0>
//
// SPDX-License-Identifier: Apache-2.0
// *******************************************************************************
//! PlantUML Include Expander Module
//! ==============================
//! Implements PlantUML-compatible include semantics:
//! - `!include` / `!include_once` directives
//! - Subblock extraction
//! - Inline expansion
//! - Cycle detection
//!
//! # Performance
//! - Parsing: O(F) — each file parsed once.
//! - Expansion: O(I × S) — re-evaluated per include due to context dependence.
//! - include_once check: O(1) per include (HashSet lookup).
//!
//! # Notes
//! - `!include_once` is scoped per entry diagram; not global.
//! - ASTs are cached globally to avoid repeated parsing.
//! - Expanded include results are context-dependent; do not cache globally.
//! - Avoid optimizations that break context-dependent expansion.

use log::debug;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use thiserror::Error;

use crate::expanded::Expanded;
use crate::include_ast::{IncludeFile, IncludeKind, IncludeStmt, IncludeSuffix, SubBlock};
use crate::include_parser::{IncludeParseError, IncludeParserService};
use crate::utils::normalize_path;

// ----------------------
// Type Aliases
// ----------------------
type GlobalSubRegistry = HashMap<Rc<PathBuf>, FileSubRegistry>;
type AstCache = HashMap<Rc<PathBuf>, Rc<Vec<IncludeFile>>>;
type FileList = HashSet<Rc<PathBuf>>;

/// Stores all subblocks in a file, indexed by label or numeric index.
#[derive(Debug, Default, Clone)]
pub struct FileSubRegistry {
    by_label: HashMap<String, Vec<Rc<SubBlock>>>,
    by_index: HashMap<u32, Vec<Rc<SubBlock>>>,
}

impl FileSubRegistry {
    fn collect_from_file(&mut self, stmts: &[IncludeFile]) {
        for stmt in stmts {
            if let IncludeFile::SubBlock(sub) = stmt {
                let rc_sub = Rc::new(sub.clone());
                match &sub.name {
                    IncludeSuffix::Label(name) => {
                        self.by_label
                            .entry(name.clone())
                            .or_default()
                            .push(Rc::clone(&rc_sub));
                    }
                    IncludeSuffix::Index(idx) => {
                        self.by_index
                            .entry(*idx)
                            .or_default()
                            .push(Rc::clone(&rc_sub));
                    }
                }
            }
        }
    }

    fn lookup(&self, suffix: &IncludeSuffix) -> Option<Vec<Rc<SubBlock>>> {
        match suffix {
            IncludeSuffix::Label(name) => self.by_label.get(name).cloned(),
            IncludeSuffix::Index(idx) => self.by_index.get(idx).cloned(),
        }
    }
}

/// Expansion context for a single PlantUML diagram.
///
/// Tracks include stack for cycle detection and included_once files.
/// Each entry diagram owns its own context.
#[derive(Debug, Default, Clone)]
pub struct IncludeContext {
    include_stack: Vec<Rc<PathBuf>>,
    included_once: HashSet<Rc<PathBuf>>,
    included_default: HashSet<Rc<PathBuf>>,
}

impl IncludeContext {
    fn push_stack(&mut self, file: &Rc<PathBuf>) -> Result<(), IncludeExpandError> {
        if self.include_stack.contains(file) {
            let mut chain = self.include_stack.clone();
            chain.push(Rc::clone(file));
            return Err(IncludeExpandError::CycleInclude { chain });
        }
        self.include_stack.push(Rc::clone(file));
        Ok(())
    }

    fn pop_stack(&mut self) {
        self.include_stack.pop();
    }

    fn check_and_mark_include(
        &mut self,
        kind: &IncludeKind,
        current_file: &Rc<PathBuf>,
        target: &Rc<PathBuf>,
    ) -> Result<bool, IncludeExpandError> {
        match kind {
            IncludeKind::Include => {
                if self.included_default.contains(target) {
                    Ok(false)
                } else {
                    self.included_default.insert(Rc::clone(target));
                    Ok(true)
                }
            }
            IncludeKind::IncludeOnce => {
                if self.included_once.contains(target) {
                    Err(IncludeExpandError::IncludeOnceViolated {
                        file: Rc::clone(current_file),
                        conflict: Rc::clone(target),
                    })
                } else {
                    self.included_once.insert(Rc::clone(target));
                    self.included_default.insert(Rc::clone(target));
                    Ok(true)
                }
            }
            IncludeKind::IncludeMany => {
                self.included_default.insert(Rc::clone(target));
                Ok(true)
            }
        }
    }
}

/// An `!include` directive: where it stands and what it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeSite {
    /// File holding the directive.
    pub from: Rc<PathBuf>,
    /// 1-based line of the directive.
    pub line: u32,
    /// Path as written in the directive.
    pub target: String,
}

/// Include Expand errors.
#[derive(Debug, Error)]
pub enum IncludeExpandError {
    #[error("{}", describe_file_not_found(.file, .site.as_ref(), .hint.as_deref(), &|path| path.display().to_string()))]
    FileNotFound {
        file: Rc<PathBuf>,
        /// The directive that named `file`; `None` for a diagram given directly.
        site: Option<IncludeSite>,
        hint: Option<String>,
    },

    #[error("Failed to parse included file {file}: {error}")]
    ParseFailed {
        file: Rc<PathBuf>,
        #[source]
        error: IncludeParseError,
    },

    #[error("Include cycle detected: {chain:?}")]
    CycleInclude { chain: Vec<Rc<PathBuf>> },

    #[error("Diagram {file} include {conflict} more than once")]
    IncludeOnceViolated {
        file: Rc<PathBuf>,
        conflict: Rc<PathBuf>,
    },

    #[error("Sub block {suffix} not found in include file {file}")]
    UnknownSub { file: Rc<PathBuf>, suffix: String },
}

#[derive(Default)]
pub struct IncludeExpander {
    include_parser: IncludeParserService,
    // global AST cache to avoid re-parsing
    ast_cache: AstCache,
    // global registry of subblocks for all parsed files
    sub_registry: GlobalSubRegistry,
}

impl IncludeExpander {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Expands `file` to text. A file without include directives is returned
    /// verbatim when it is the only file in `file_list`.
    pub fn expand(
        &mut self,
        file: &Rc<PathBuf>,
        file_list: &FileList,
    ) -> Result<String, IncludeExpandError> {
        let verbatim_if_plain = file_list.len() == 1;
        Ok(self.expand_entry(file, file_list, verbatim_if_plain)?.text)
    }

    /// Expands `file` to text and records the source line of each of its
    /// lines. A file without include directives is returned verbatim, so
    /// `file_list` may hold include files besides `file`.
    pub fn expand_mapped(
        &mut self,
        file: &Rc<PathBuf>,
        file_list: &FileList,
    ) -> Result<Expanded, IncludeExpandError> {
        self.expand_entry(file, file_list, true)
    }

    fn expand_entry(
        &mut self,
        file: &Rc<PathBuf>,
        file_list: &FileList,
        verbatim_if_plain: bool,
    ) -> Result<Expanded, IncludeExpandError> {
        let mut ctx = IncludeContext::default();
        self.expand_file(file, &mut ctx, file_list, verbatim_if_plain)
    }

    /// Expands a single diagram file, recursively processing includes.
    ///
    /// # Arguments
    /// - `file`: path to the diagram
    /// - `ctx`: per-diagram `IncludeContext`
    /// - `file_list`: set of all available files
    /// - `verbatim_if_plain`: return a file without include directives as is
    ///
    /// # Returns
    /// - Fully expanded PlantUML text, with the source line of each line
    ///
    /// # Errors
    /// - `IncludeExpandError::FileNotFound`: file missing
    /// - `IncludeExpandError::ParseFailed`: read or parse errors
    /// - `IncludeExpandError::CycleInclude`: include cycle detected
    /// - `IncludeExpandError::IncludeOnceViolated`: `!include_once` violation
    /// - `IncludeExpandError::UnknownSub`: includesub with invalid suffix
    fn expand_file(
        &mut self,
        file: &Rc<PathBuf>,
        ctx: &mut IncludeContext,
        file_list: &FileList,
        verbatim_if_plain: bool,
    ) -> Result<Expanded, IncludeExpandError> {
        debug!("expand_file is {}", file.display());

        if !file_list.contains(file) {
            return Err(IncludeExpandError::FileNotFound {
                file: Rc::clone(file),
                site: None,
                hint: None,
            });
        }

        ctx.push_stack(file)?;

        let original_content =
            fs::read_to_string(file.as_ref()).map_err(|_| IncludeExpandError::FileNotFound {
                file: Rc::clone(file),
                site: None,
                hint: None,
            })?;

        if verbatim_if_plain && !has_include_or_subblock_directive(&original_content) {
            ctx.pop_stack();
            return Ok(Expanded::verbatim(file, &original_content));
        }

        let stmts_ast = self.load_ast(file)?;
        let mut expanded = Expanded::default();
        self.expand_stmts(&stmts_ast, file, ctx, file_list, &mut expanded)?;
        ctx.pop_stack();
        Ok(expanded)
    }

    /// Loads AST from cache or parses file if not present.
    ///
    /// # Returns
    /// - Parsed AST of the file.
    ///
    /// # Errors
    /// - `IncludeExpandError::ParseFailed`: read or parse errors
    fn load_ast(&mut self, file: &Rc<PathBuf>) -> Result<Rc<Vec<IncludeFile>>, IncludeExpandError> {
        if let Some(ast) = self.ast_cache.get(file) {
            return Ok(Rc::clone(ast));
        }

        let ast =
            self.include_parser
                .parse_file(file)
                .map_err(|e| IncludeExpandError::ParseFailed {
                    file: Rc::clone(file),
                    error: e,
                })?;

        self.sub_registry
            .entry(Rc::clone(file))
            .or_default()
            .collect_from_file(&ast);

        let rc_ast = Rc::new(ast);
        self.ast_cache.insert(Rc::clone(file), Rc::clone(&rc_ast));

        Ok(rc_ast)
    }

    /// Recursively expands a sequence of AST statements into `out`.
    ///
    /// # Errors
    /// - Propagates errors from include expansion
    fn expand_stmts(
        &mut self,
        stmts: &[IncludeFile],
        current_file: &Rc<PathBuf>,
        ctx: &mut IncludeContext,
        file_list: &FileList,
        out: &mut Expanded,
    ) -> Result<(), IncludeExpandError> {
        for stmt in stmts {
            match stmt {
                IncludeFile::Text { text, line } => out.push_line(text, current_file, *line),
                IncludeFile::Include(inc) => {
                    let expanded = self.expand_include(inc, current_file, ctx, file_list)?;
                    out.append(expanded);
                }
                IncludeFile::SubBlock(sub) => {
                    self.expand_stmts(&sub.content, current_file, ctx, file_list, out)?;
                }
            }
        }

        Ok(())
    }

    /// Expands a single `IncludeStmt` inline.
    ///
    /// # Returns
    /// - Expanded text of included file or subblock
    ///
    /// # Errors
    /// - Propagates errors from file expansion or `!include_once` check
    fn expand_include(
        &mut self,
        inc: &IncludeStmt,
        current_file: &Rc<PathBuf>,
        ctx: &mut IncludeContext,
        file_list: &FileList,
    ) -> Result<Expanded, IncludeExpandError> {
        let base_dir = current_file
            .parent()
            .ok_or_else(|| IncludeExpandError::FileNotFound {
                file: Rc::clone(current_file),
                site: None,
                hint: None,
            })?;

        match inc {
            IncludeStmt::Include { kind, path, line } => {
                let site = IncludeSite {
                    from: Rc::clone(current_file),
                    line: *line,
                    target: path.clone(),
                };
                let path = resolve_target(&site, base_dir, file_list)?;

                if !ctx.check_and_mark_include(kind, current_file, &path)? {
                    return Ok(Expanded::default());
                }

                let full = self.expand_file(&path, ctx, file_list, false)?;
                Ok(full.without_start_end())
            }

            IncludeStmt::IncludeSub { path, suffix, line } => {
                let site = IncludeSite {
                    from: Rc::clone(current_file),
                    line: *line,
                    target: path.clone(),
                };
                let path = resolve_target(&site, base_dir, file_list)?;

                let _ = self.load_ast(&path)?;
                let subs = self
                    .sub_registry
                    .get(&path)
                    .and_then(|r| r.lookup(suffix))
                    .ok_or_else(|| IncludeExpandError::UnknownSub {
                        suffix: match &suffix {
                            IncludeSuffix::Label(name) => name.clone(),
                            IncludeSuffix::Index(idx) => idx.to_string(),
                        },
                        file: Rc::clone(&path),
                    })?;

                let mut result = Expanded::default();
                for sub in subs {
                    self.expand_stmts(&sub.content, &path, ctx, file_list, &mut result)?;
                }

                Ok(result)
            }
        }
    }
}

/// The file an include directive names: its path is relative to the
/// directory of the including file, and the file must be in `file_list`.
fn resolve_target(
    site: &IncludeSite,
    base_dir: &Path,
    file_list: &FileList,
) -> Result<Rc<PathBuf>, IncludeExpandError> {
    let target = Rc::new(normalize_path(&base_dir.join(&site.target)));
    if file_list.contains(&target) {
        return Ok(target);
    }

    Err(IncludeExpandError::FileNotFound {
        hint: Some(not_found_hint(&site.target, base_dir, file_list)),
        file: target,
        site: Some(site.clone()),
    })
}

/// How to fix an include path that names no available file.
fn not_found_hint(target: &str, base_dir: &Path, file_list: &FileList) -> String {
    let written = Path::new(target);
    if written.is_absolute() {
        return "include paths are relative to the including file's directory".to_string();
    }

    match file_list.iter().filter(|f| f.ends_with(written)).min() {
        Some(candidate) => format!(
            "include paths are relative to the including file's directory; did you mean \"{}\"?",
            relative_path(base_dir, candidate).display()
        ),
        None => "declare the file as an include file (`--include-file`; a `.iuml` file in a \
                 view of architectural_design in rules_score)"
            .to_string(),
    }
}

/// `to` as a path relative to the directory `from`.
fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from: Vec<Component> = from.components().collect();
    let to: Vec<Component> = to.components().collect();
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();

    let mut relative = PathBuf::new();
    for _ in shared..from.len() {
        relative.push("..");
    }
    for component in &to[shared..] {
        relative.push(component.as_os_str());
    }
    relative
}

/// The message of an include that names no available file; `name_of` names a
/// file in it.
fn describe_file_not_found(
    file: &Path,
    site: Option<&IncludeSite>,
    hint: Option<&str>,
    name_of: &dyn Fn(&Path) -> String,
) -> String {
    let Some(site) = site else {
        return format!("Diagram {} not found.", name_of(file));
    };

    let from = name_of(&site.from);
    let target = match Path::new(&from).parent() {
        Some(dir) if !Path::new(&site.target).is_absolute() => {
            normalize_path(&dir.join(&site.target))
                .display()
                .to_string()
        }
        _ => name_of(file),
    };
    let mut message = format!(
        "{from}:{}: !include \"{}\" resolves to {target}, which is not an available file",
        site.line, site.target,
    );
    if let Some(hint) = hint {
        message.push_str("\n  hint: ");
        message.push_str(hint);
    }
    message
}

impl IncludeExpandError {
    /// The message, with `name_of` naming the files of a missing include.
    pub fn describe(&self, name_of: &dyn Fn(&Path) -> String) -> String {
        match self {
            Self::FileNotFound { file, site, hint } => {
                describe_file_not_found(file, site.as_ref(), hint.as_deref(), name_of)
            }
            other => other.to_string(),
        }
    }
}

fn has_include_or_subblock_directive(content: &str) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("!include")
            || trimmed.starts_with("!includesub")
            || trimmed.starts_with("!startsub")
            || trimmed.starts_with("!endsub")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct Workspace {
        root: PathBuf,
    }

    impl Workspace {
        fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let root = std::env::temp_dir().join(format!(
                "puml_include_{}_{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn write(&self, name: &str, content: &str) -> Rc<PathBuf> {
            let path = self.root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, content).unwrap();
            Rc::new(path)
        }
    }

    impl Drop for Workspace {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).ok();
        }
    }

    fn files(paths: &[&Rc<PathBuf>]) -> FileList {
        paths.iter().map(|p| Rc::clone(p)).collect()
    }

    fn lines_of(expanded: &Expanded) -> Vec<(String, u32)> {
        expanded
            .origins
            .iter()
            .map(|o| (o.file.file_name().unwrap().to_string_lossy().into(), o.line))
            .collect()
    }

    #[test]
    fn included_lines_keep_their_own_file_and_line() {
        let ws = Workspace::new();
        let fragment = ws.write(
            "sub/frag.puml",
            "' header\n\nB -> C : first\nC -> B : second\n",
        );
        let main = ws.write(
            "main.puml",
            "@startuml\n' note\n\n!include sub/frag.puml\nA -> B : last\n@enduml\n",
        );

        let expanded = IncludeExpander::new()
            .expand_mapped(&main, &files(&[&main, &fragment]))
            .unwrap();

        assert_eq!(
            expanded.text,
            "@startuml\nB -> C : first\nC -> B : second\nA -> B : last\n@enduml\n"
        );
        assert_eq!(
            lines_of(&expanded),
            [
                ("main.puml".into(), 1),
                ("frag.puml".into(), 3),
                ("frag.puml".into(), 4),
                ("main.puml".into(), 5),
                ("main.puml".into(), 6),
            ]
        );
    }

    #[test]
    fn nested_include_is_relative_to_the_including_fragment() {
        let ws = Workspace::new();
        let inner = ws.write("sub/deep/inner.puml", "X -> Y\n");
        let fragment = ws.write("sub/frag.puml", "!include deep/inner.puml\n");
        let main = ws.write("main.puml", "@startuml\n!include sub/frag.puml\n@enduml\n");

        let expanded = IncludeExpander::new()
            .expand_mapped(&main, &files(&[&main, &fragment, &inner]))
            .unwrap();

        assert_eq!(expanded.text, "@startuml\nX -> Y\n@enduml\n");
        assert_eq!(lines_of(&expanded)[1], ("inner.puml".into(), 1));
    }

    #[test]
    fn file_without_directives_is_returned_verbatim() {
        let ws = Workspace::new();
        let fragment = ws.write("frag.puml", "X -> Y\n");
        let main = ws.write("main.puml", "@startuml\n' note\n\nA -> B\n@enduml\n");

        let expanded = IncludeExpander::new()
            .expand_mapped(&main, &files(&[&main, &fragment]))
            .unwrap();

        assert_eq!(expanded.text, "@startuml\n' note\n\nA -> B\n@enduml\n");
        assert_eq!(expanded.origins.len(), 5);
    }

    #[test]
    fn workspace_relative_path_is_reported_with_the_relative_one() {
        let ws = Workspace::new();
        let fragment = ws.write("pkg/dyn/sub/frag.puml", "X -> Y\n");
        let main = ws.write(
            "pkg/dyn/main.puml",
            "@startuml\n!include pkg/dyn/sub/frag.puml\n@enduml\n",
        );

        let error = IncludeExpander::new()
            .expand_mapped(&main, &files(&[&main, &fragment]))
            .unwrap_err();

        let message = error.to_string();
        assert!(
            message.contains("main.puml:2: !include \"pkg/dyn/sub/frag.puml\""),
            "{message}"
        );
        assert!(
            message.contains("did you mean \"sub/frag.puml\"?"),
            "{message}"
        );
    }

    #[test]
    fn undeclared_file_is_reported_with_the_way_to_declare_it() {
        let ws = Workspace::new();
        ws.write("frag.puml", "X -> Y\n");
        let main = ws.write("main.puml", "@startuml\n!include frag.puml\n@enduml\n");

        let error = IncludeExpander::new()
            .expand_mapped(&main, &files(&[&main]))
            .unwrap_err();

        let message = error.to_string();
        assert!(message.contains("main.puml:2"), "{message}");
        assert!(message.contains("--include-file"), "{message}");
    }

    #[test]
    fn missing_include_is_described_with_the_given_names() {
        let ws = Workspace::new();
        let main = ws.write("main.puml", "@startuml\n!include sub/frag.puml\n@enduml\n");

        let error = IncludeExpander::new()
            .expand_mapped(&main, &files(&[&main]))
            .unwrap_err();

        let main_path = main.as_path().to_path_buf();
        let message = error.describe(&|path| {
            if path == main_path {
                "pkg/main.puml".to_string()
            } else {
                path.display().to_string()
            }
        });
        assert!(
            message.starts_with(
                "pkg/main.puml:2: !include \"sub/frag.puml\" resolves to pkg/sub/frag.puml,"
            ),
            "{message}"
        );
    }

    #[test]
    fn absolute_path_that_is_not_available_is_rejected_with_a_hint() {
        let ws = Workspace::new();
        let main = ws.write("main.puml", "@startuml\n!include /abs/frag.puml\n@enduml\n");

        let error = IncludeExpander::new()
            .expand_mapped(&main, &files(&[&main]))
            .unwrap_err();

        assert!(error.to_string().contains("relative to the including file"));
    }

    #[test]
    fn includesub_is_limited_to_available_files() {
        let ws = Workspace::new();
        ws.write("lib.puml", "!startsub Part\nX -> Y\n!endsub\n");
        let main = ws.write(
            "main.puml",
            "@startuml\n!includesub lib.puml!Part\n@enduml\n",
        );

        let error = IncludeExpander::new()
            .expand_mapped(&main, &files(&[&main]))
            .unwrap_err();

        assert!(matches!(
            error,
            IncludeExpandError::FileNotFound { site: Some(_), .. }
        ));
    }

    #[test]
    fn relative_path_walks_up_from_the_base_directory() {
        assert_eq!(
            relative_path(Path::new("/a/b/c"), Path::new("/a/b/x/y.puml")),
            Path::new("../x/y.puml")
        );
        assert_eq!(
            relative_path(Path::new("/a/b"), Path::new("/a/b/sub/y.puml")),
            Path::new("sub/y.puml")
        );
    }
}
