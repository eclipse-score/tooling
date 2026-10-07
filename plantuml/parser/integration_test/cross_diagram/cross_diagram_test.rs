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

//! Cross-diagram integration suite for `puml_cli`'s `--idmap-output-dir` output.
//!
//! Each seed case directory under this crate contains one or more `.puml`
//! files, an optional `case.yaml` describing how to invoke `puml_cli` and
//! which cross-file identifier assertions must hold, and an `output.json`
//! (or, for cases where every file is expected to fail, `output.yaml`)
//! golden capturing the idmap `defines`/`references` produced for every
//! file. This suite drives `test_framework::run_case` like the other
//! diagram-parser/resolver suites: [`PumlCliIdmapRunner`] shells out to
//! `puml_cli` per file (`DiagramProcessor`), and [`CrossDiagramChecker`]
//! adds the `links`/`distinct` cross-file id assertions on top of the
//! framework's default per-file checks (`ExpectationChecker::check_case`).
//! Files listed under `errors` must fail with the given error substrings and are left out
//! of the golden.
//!
//! Goldens document *current* behavior; they are updated by the changes that
//! implement the target design in `plantuml/parser/docs/element-identifiers.md`.

use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use test_framework::{
    run_case, DefaultExpectationChecker, DiagramProcessor, ErrorView, ExpectationChecker, Expected,
    ProjectedError,
};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct CaseConfig {
    /// Maps a `.puml` file name to the `--diagram-type` value to pass to
    /// `puml_cli`. Files not listed here are run without `--diagram-type`
    /// (letting `puml_cli` auto-detect the diagram type).
    diagram_types: HashMap<String, String>,
    /// Groups of `file.puml#Alias` references that must resolve to the same
    /// idmap id.
    links: Vec<Vec<String>>,
    /// Groups of `file.puml#Alias` references that must resolve to pairwise
    /// distinct idmap ids.
    distinct: Vec<Vec<String>>,
    /// Files that must fail, with substrings their error must contain. Such
    /// files produce no idmap output.
    errors: HashMap<String, Vec<String>>,
}

/// One or more idmap ids sharing a single alias, e.g. two same-named classes
/// nested in different scopes. A golden may write either a single scalar
/// (`Recorder: "logging.Recorder"`) or a list (`Recorder: ["a.R", "b.R"]`).
#[derive(Debug, PartialEq, Eq)]
struct IdSet(BTreeSet<String>);

impl<'de> Deserialize<'de> for IdSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            One(String),
            Many(Vec<String>),
        }

        Ok(match Repr::deserialize(deserializer)? {
            Repr::One(id) => IdSet(BTreeSet::from([id])),
            Repr::Many(ids) => IdSet(ids.into_iter().collect()),
        })
    }
}

impl Serialize for IdSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Mirrors the flexible Deserialize impl above: a single id is
        // written as a bare scalar (matching how goldens are hand-written),
        // multiple ids as a list.
        match self.0.len() {
            1 => self.0.iter().next().unwrap().serialize(serializer),
            _ => self.0.iter().collect::<Vec<_>>().serialize(serializer),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
struct IdMapSections {
    defines: BTreeMap<String, IdSet>,
    references: BTreeMap<String, IdSet>,
}

/// Resolves a path under `TEST_SRCDIR`/`TEST_WORKSPACE`, mirroring
/// `validation/core/integration_test`'s `case_file_path` helper.
fn case_file_path(relative_path: &str) -> PathBuf {
    let test_srcdir = std::env::var("TEST_SRCDIR").expect("TEST_SRCDIR is not set");
    let workspace = std::env::var("TEST_WORKSPACE").expect("TEST_WORKSPACE is not set");

    PathBuf::from(test_srcdir)
        .join(workspace)
        .join(relative_path)
}

fn puml_cli_path() -> PathBuf {
    case_file_path("plantuml/parser/puml_cli/puml_cli")
}

fn load_case_config(dir: &Path) -> CaseConfig {
    let path = dir.join("case.yaml");
    if !path.exists() {
        return CaseConfig::default();
    }

    let content = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_yaml::from_str(&content)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn puml_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to list {}: {error}", dir.display()))
        .filter_map(|entry| {
            let path = entry
                .unwrap_or_else(|error| {
                    panic!("failed to read entry in {}: {error}", dir.display())
                })
                .path();
            (path.extension().is_some_and(|ext| ext == "puml")).then_some(path)
        })
        .collect();
    files.sort();
    files
}

fn test_tmp_dir(case_name: &str, file_stem: &str) -> PathBuf {
    let test_tmpdir = std::env::var("TEST_TMPDIR").expect("TEST_TMPDIR is not set");
    let dir = PathBuf::from(test_tmpdir)
        .join("cross_diagram")
        .join(case_name)
        .join(file_stem);
    fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", dir.display()));
    dir
}

/// The error `puml_cli` reports on stderr for a single file, with the
/// `"Resolve error in {file}: "` (or equivalent) prefix stripped, since the
/// input file is already known to the caller.
#[derive(Debug)]
struct PumlCliError {
    file: PathBuf,
    message: String,
}

impl ErrorView for PumlCliError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        let file = self.file.strip_prefix(base_dir).unwrap_or(&self.file);
        ProjectedError::new("ResolveError")
            .with_field("file", file.to_string_lossy().into_owned())
            .with_field("message", self.message.clone())
    }
}

/// Runs `puml_cli --idmap-output-dir` for a single file and parses its
/// output (or failure) into this suite's `DiagramProcessor` types.
fn run_file(
    case_name: &str,
    path: &Path,
    diagram_type: Option<&str>,
) -> Result<IdMapSections, PumlCliError> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_else(|| panic!("{case_name}: non-utf8 file name {}", path.display()));
    let out_dir = test_tmp_dir(case_name, stem);

    let mut cmd = Command::new(puml_cli_path());
    cmd.arg("--file").arg(path);
    if let Some(diagram_type) = diagram_type {
        cmd.arg("--diagram-type").arg(diagram_type);
    }
    cmd.arg("--idmap-output-dir").arg(&out_dir);

    let output = cmd
        .output()
        .unwrap_or_else(|error| panic!("failed to execute puml_cli for {case_name}: {error}"));

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let prefix = format!("Resolve error in {}: ", path.display());
        let message = stderr
            .strip_prefix(prefix.as_str())
            .unwrap_or(&stderr)
            .to_string();
        return Err(PumlCliError {
            file: path.to_path_buf(),
            message,
        });
    }

    let idmap_path = out_dir.join(format!("{stem}.idmap.json"));
    let idmap_content = fs::read_to_string(&idmap_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", idmap_path.display()));

    #[derive(Deserialize)]
    struct RawIdMapEntry {
        alias: String,
        id: String,
    }
    #[derive(Deserialize)]
    struct RawIdMapFile {
        defines: Vec<RawIdMapEntry>,
        references: Vec<RawIdMapEntry>,
    }

    let raw: RawIdMapFile = serde_json::from_str(&idmap_content)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", idmap_path.display()));
    let to_map = |entries: Vec<RawIdMapEntry>| {
        let mut grouped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for entry in entries {
            grouped.entry(entry.alias).or_default().insert(entry.id);
        }
        grouped
            .into_iter()
            .map(|(alias, ids)| (alias, IdSet(ids)))
            .collect::<BTreeMap<_, _>>()
    };

    Ok(IdMapSections {
        defines: to_map(raw.defines),
        references: to_map(raw.references),
    })
}

/// Runs `puml_cli` for every file in a case, driven by `test_framework`.
struct PumlCliIdmapRunner;

impl DiagramProcessor for PumlCliIdmapRunner {
    type Output = IdMapSections;
    type Error = PumlCliError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, IdMapSections>, PumlCliError> {
        let dir = files
            .iter()
            .next()
            .and_then(|file| file.parent())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let case_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("cross_diagram_case")
            .to_string();
        let config = load_case_config(&dir);

        let mut results = HashMap::new();
        for path in files {
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let diagram_type = config.diagram_types.get(&file_name).map(String::as_str);
            let outcome = run_file(&case_name, path, diagram_type);

            match (outcome, config.errors.get(&file_name)) {
                (Ok(idmap), None) => {
                    results.insert(Rc::clone(path), idmap);
                }
                (Err(error), None) => return Err(error),
                (Err(error), Some(substrings)) => {
                    for substring in substrings {
                        assert!(
                            error.message.contains(substring.as_str()),
                            "{case_name}: error for {file_name} must contain {substring:?}, got {:?}",
                            error.message
                        );
                    }
                }
                (Ok(_), Some(_)) => {
                    panic!("{case_name}: {file_name} is listed in `errors` but succeeded")
                }
            }
        }
        Ok(results)
    }
}

/// Resolves a `"file.puml#Alias"` reference to its idmap id. `alias` must
/// name exactly one define-or-reference entry, and that entry must resolve
/// to exactly one id, or this panics.
fn resolve_ref(
    resolved: &HashMap<String, &IdMapSections>,
    reference: &str,
    case_name: &str,
) -> String {
    let (file_name, alias) = reference.split_once('#').unwrap_or_else(|| {
        panic!("{case_name}: malformed reference {reference:?}, expected file.puml#Alias")
    });

    let sections = resolved.get(file_name).unwrap_or_else(|| {
        panic!("{case_name}: reference {reference:?} names a file with no successful idmap output")
    });

    let ids = match (sections.defines.get(alias), sections.references.get(alias)) {
        (Some(_), Some(_)) => {
            panic!("{case_name}: alias {alias:?} in {file_name} is both a define and a reference")
        }
        (Some(ids), None) | (None, Some(ids)) => ids,
        (None, None) => {
            panic!("{case_name}: alias {alias:?} not found in {file_name}'s idmap")
        }
    };

    assert_eq!(
        ids.0.len(),
        1,
        "{case_name}: alias {alias:?} in {file_name} resolves to more than one id: {:?}",
        ids.0
    );
    ids.0.iter().next().unwrap().clone()
}

/// Catches typos in `case.yaml`: file names that don't exist in this case,
/// `errors` entries that assert nothing, `links`/`distinct` groups too small
/// to assert anything, and references to files that produce no idmap.
fn validate_case_config(case_name: &str, file_names: &BTreeSet<String>, config: &CaseConfig) {
    let configured_files = config.diagram_types.keys().chain(config.errors.keys());
    for file_name in configured_files {
        assert!(
            file_names.contains(file_name),
            "{case_name}: case.yaml names {file_name:?}, which is not a .puml file in this case"
        );
    }

    for (file_name, substrings) in &config.errors {
        assert!(
            !substrings.is_empty(),
            "{case_name}: `errors` entry for {file_name:?} needs at least one error substring"
        );
    }

    for group in config.links.iter().chain(config.distinct.iter()) {
        assert!(
            group.len() >= 2,
            "{case_name}: a links/distinct group needs at least 2 entries, got {group:?}"
        );
        for reference in group {
            let (file_name, _alias) = reference.split_once('#').unwrap_or_else(|| {
                panic!("{case_name}: malformed reference {reference:?}, expected file.puml#Alias")
            });
            assert!(
                file_names.contains(file_name),
                "{case_name}: reference {reference:?} names a file that is not in this case"
            );
            assert!(
                !config.errors.contains_key(file_name),
                "{case_name}: reference {reference:?} names a file listed in `errors`, which produces no idmap"
            );
        }
    }
}

/// Adds the `links`/`distinct` cross-file id assertions on top of the
/// framework's default per-file `check_ok`/`check_err`.
struct CrossDiagramChecker;

impl ExpectationChecker<PumlCliError, IdMapSections> for CrossDiagramChecker {
    fn check_ok(&self, actual: &IdMapSections, expected: &Expected<IdMapSections>) {
        ExpectationChecker::<PumlCliError, IdMapSections>::check_ok(
            &DefaultExpectationChecker,
            actual,
            expected,
        );
    }

    fn check_err(&self, err: &PumlCliError, expected: &YamlValue, base_dir: &Path) {
        ExpectationChecker::<PumlCliError, IdMapSections>::check_err(
            &DefaultExpectationChecker,
            err,
            expected,
            base_dir,
        );
    }

    fn check_case(&self, outputs: &HashMap<Rc<PathBuf>, IdMapSections>, dir: &Path) {
        let case_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("cross_diagram_case");
        let config = load_case_config(dir);

        let file_names: BTreeSet<String> = puml_files(dir)
            .iter()
            .map(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_else(|| panic!("{case_name}: non-utf8 file name {}", path.display()))
                    .to_string()
            })
            .collect();
        validate_case_config(case_name, &file_names, &config);

        let by_name: HashMap<String, &IdMapSections> = outputs
            .iter()
            .map(|(path, sections)| {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_else(|| panic!("{case_name}: non-utf8 file name {}", path.display()))
                    .to_string();
                (file_name, sections)
            })
            .collect();

        for group in &config.links {
            let ids: Vec<String> = group
                .iter()
                .map(|reference| resolve_ref(&by_name, reference, case_name))
                .collect();
            for pair in ids.windows(2) {
                assert_eq!(
                    pair[0], pair[1],
                    "{case_name}: expected {group:?} to all resolve to the same id, got {ids:?}"
                );
            }
        }

        for group in &config.distinct {
            let ids: Vec<String> = group
                .iter()
                .map(|reference| resolve_ref(&by_name, reference, case_name))
                .collect();
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    assert_ne!(
                        ids[i], ids[j],
                        "{case_name}: expected {:?} and {:?} to resolve to distinct ids, both got {:?}",
                        group[i], group[j], ids[i]
                    );
                }
            }
        }
    }
}

/// Lists the case directories that actually exist on disk, so
/// [`cross_diagram_cases!`] can be checked against reality instead of silently
/// never running a case whose directory has no matching `#[test]` fn. A
/// directory counts as a case only if it directly contains a `.puml` file,
/// which excludes unrelated sibling directories the test runner may place
/// under this crate at runtime (e.g. its own working directory).
fn discover_case_names() -> BTreeSet<String> {
    let base = case_file_path("plantuml/parser/integration_test/cross_diagram");
    fs::read_dir(&base)
        .unwrap_or_else(|error| panic!("failed to list {}: {error}", base.display()))
        .filter_map(|entry| {
            let entry = entry.unwrap_or_else(|error| {
                panic!("failed to read entry in {}: {error}", base.display())
            });
            let path = entry.path();
            (path.is_dir() && !puml_files(&path).is_empty())
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect()
}

fn run_cross_diagram_case(case_name: &str) {
    run_case(
        "integration_test/cross_diagram",
        case_name,
        PumlCliIdmapRunner,
        CrossDiagramChecker,
    );
}

/// Declares one `#[test]` per case name, plus a guard test asserting that the
/// list matches the case directories on disk in both directions.
macro_rules! cross_diagram_cases {
    ($($name:ident),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                run_cross_diagram_case(stringify!($name));
            }
        )+

        #[test]
        fn all_case_directories_are_registered() {
            let registered: BTreeSet<String> =
                [$(stringify!($name)),+].into_iter().map(str::to_string).collect();
            let on_disk = discover_case_names();
            assert_eq!(
                registered, on_disk,
                "case directories under cross_diagram/ must exactly match the #[test] fns \
                 declared via cross_diagram_cases!()"
            );
        }
    };
}

cross_diagram_cases!(
    component_nesting,
    class_name_wins,
    alias_is_local_key,
    namespace_and_package,
    sequence_forms,
    prose_names,
    linking_three_diagrams,
    qualified_reference,
    qualified_in_nested_scope,
    doc_4_trap_prose_label,
    doc_6_external_endpoint,
    errors_participants,
    errors_listed_file,
    unit_to_class_link,
    separator_equivalence,
);
