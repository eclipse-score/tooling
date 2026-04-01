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
use std::path::PathBuf;
use std::rc::Rc;
use thiserror::Error;

use crate::include_ast::{IncludeFile, IncludeKind, IncludeStmt, IncludeSuffix, SubBlock};
use crate::include_parser::{IncludeParseError, IncludeParserService};
use crate::utils::{normalize_path, strip_start_end};

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

/// Include Expand errors.
#[derive(Debug, Error)]
pub enum IncludeExpandError {
    #[error("Diagram {file} not found.")]
    FileNotFound { file: Rc<PathBuf> },

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

    pub fn expand(
        &mut self,
        file: &Rc<PathBuf>,
        file_list: &FileList,
    ) -> Result<String, IncludeExpandError> {
        let mut ctx = IncludeContext::default();
        self.expand_file(file, &mut ctx, file_list)
    }

    /// Expands a single diagram file, recursively processing includes.
    ///
    /// # Arguments
    /// - `file`: path to the entry diagram
    /// - `ctx`: per-diagram `IncludeContext`
    /// - `file_list`: set of all available files
    ///
    /// # Returns
    /// - Fully expanded PlantUML text as `String`
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
    ) -> Result<String, IncludeExpandError> {
        debug!("expand_file is {}", file.display());

        if !file_list.contains(file) {
            return Err(IncludeExpandError::FileNotFound {
                file: Rc::clone(file),
            });
        }

        ctx.push_stack(file)?;

        let stmts_ast = self.load_ast(file)?;
        let expanded_ast = self.expand_stmts(&stmts_ast, file, ctx, file_list)?;
        let rendered = render_stmts(expanded_ast);

        ctx.pop_stack();
        Ok(rendered)
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

    /// Recursively expands a sequence of AST statements.
    ///
    /// # Returns
    /// - Fully expanded AST statements
    ///
    /// # Errors
    /// - Propagates errors from include expansion
    fn expand_stmts(
        &mut self,
        stmts: &[IncludeFile],
        current_file: &Rc<PathBuf>,
        ctx: &mut IncludeContext,
        file_list: &FileList,
    ) -> Result<Vec<IncludeFile>, IncludeExpandError> {
        let mut result = Vec::new();

        for stmt in stmts {
            match stmt {
                IncludeFile::Text(text) => {
                    result.push(IncludeFile::Text(text.clone()));
                }
                IncludeFile::Include(inc) => {
                    let expanded = self.expand_include(inc, current_file, ctx, file_list)?;
                    result.push(IncludeFile::Text(expanded));
                }
                IncludeFile::SubBlock(sub) => {
                    let expanded_content =
                        self.expand_stmts(&sub.content, current_file, ctx, file_list)?;
                    result.push(IncludeFile::SubBlock(SubBlock {
                        name: sub.name.clone(),
                        content: expanded_content,
                    }));
                }
            }
        }

        Ok(result)
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
    ) -> Result<String, IncludeExpandError> {
        let base_dir = current_file
            .parent()
            .ok_or_else(|| IncludeExpandError::FileNotFound {
                file: Rc::clone(current_file),
            })?;

        match inc {
            IncludeStmt::Include { kind, path } => {
                let path = Rc::new(normalize_path(&base_dir.join(path)));

                if !ctx.check_and_mark_include(kind, current_file, &path)? {
                    return Ok(String::new());
                }

                let full_text = self.expand_file(&path, ctx, file_list)?;
                Ok(strip_start_end(&full_text))
            }

            IncludeStmt::IncludeSub { path, suffix } => {
                let path = Rc::new(normalize_path(&base_dir.join(path)));

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

                let mut result = String::new();
                for sub in subs {
                    let full_sub_text = self.expand_stmts(&sub.content, &path, ctx, file_list)?;
                    result.push_str(&render_stmts(full_sub_text));
                }

                Ok(result)
            }
        }
    }
}

/// Renders AST statements into PlantUML text.
///
/// # Arguments
/// - `stmts`: AST nodes to render
///
/// # Returns
/// - `String` containing PlantUML text
pub fn render_stmts(stmts: Vec<IncludeFile>) -> String {
    let mut out = String::new();

    for stmt in stmts {
        stmt.render(&mut out);
    }

    out
}
