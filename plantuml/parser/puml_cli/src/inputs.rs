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

//! Input files and the per-file / per-run flags that describe them.

use std::borrow::Cow;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use puml_utils::is_trlc_identifier;

/// Collects the input files, deduplicated: every `files` entry in the given
/// order, then the files of `folder` sorted. All of them are resolved as one
/// group per diagram kind.
pub fn collect_input_files(
    files: &[String],
    folder: Option<&str>,
) -> Result<Vec<Rc<PathBuf>>, Box<dyn std::error::Error>> {
    let mut ordered: Vec<Rc<PathBuf>> = Vec::new();
    let mut seen: HashSet<Rc<PathBuf>> = HashSet::new();

    for file_path in files {
        let abs_path = Rc::new(resolve_puml_file(Path::new(file_path))?);
        if !seen.insert(Rc::clone(&abs_path)) {
            return Err(format!("Duplicate --file entry: {file_path}").into());
        }
        ordered.push(abs_path);
    }

    if let Some(folder_path) = folder {
        let mut folder_files: HashSet<Rc<PathBuf>> = HashSet::new();
        collect_puml_files_from_folder(Path::new(folder_path), &mut folder_files)?;
        let mut folder_files: Vec<Rc<PathBuf>> = folder_files.into_iter().collect();
        folder_files.sort();
        for path in folder_files {
            if seen.insert(Rc::clone(&path)) {
                ordered.push(path);
            }
        }
    }

    Ok(ordered)
}

fn resolve_path(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    // When running with 'bazel run', use BUILD_WORKSPACE_DIRECTORY
    let base_dir = match std::env::var("BUILD_WORKSPACE_DIRECTORY") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => std::env::current_dir()?,
    };

    Ok(base_dir.join(path))
}

/// Rejects a per-file flag given neither zero times nor once per input file.
pub fn require_one_per_file_or_none(
    values: &[String],
    file_count: usize,
    flag_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !values.is_empty() && values.len() != file_count {
        return Err(format!(
            "--{flag_name} must be given exactly once per input file (in the same \
             order as --file) or not at all; got {file_count} input file(s), \
             {} --{flag_name}",
            values.len(),
        )
        .into());
    }
    Ok(())
}

/// Validate a user-supplied `--source-name` value.
///
/// Rejects empty strings and absolute paths: an absolute path would leak the
/// local machine's directory layout into build outputs (breaking build
/// reproducibility) and would defeat the whole purpose of the flag, which is
/// to supply a *stable, workspace-relative* key for cross-diagram matching.
pub fn validate_source_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if name.is_empty() {
        return Err("--source-name must not be empty".into());
    }
    if Path::new(name).is_absolute() {
        return Err(format!(
            "--source-name must be a workspace-relative path, got an absolute path: {name}"
        )
        .into());
    }
    Ok(())
}

/// Validate a user-supplied `--output-stem` value.
///
/// Unlike `--source-name` (a full relative path embedded *inside* outputs),
/// this becomes the output *filename* stem itself, so it must be a single
/// path component: rejects empty strings and anything containing a `/` or
/// resolving to `.`/`..`.
pub fn validate_output_stem(stem: &str) -> Result<(), Box<dyn std::error::Error>> {
    if stem.is_empty() {
        return Err("--output-stem must not be empty".into());
    }
    let mut components = Path::new(stem).components();
    let is_single_normal_component =
        matches!(components.next(), Some(std::path::Component::Normal(_)))
            && components.next().is_none();
    if !is_single_normal_component {
        return Err(format!(
            "--output-stem must be a single path component (no '/', '.', or '..'), got: {stem}"
        )
        .into());
    }
    Ok(())
}

/// Validate a `--*-package` value: the generated TRLC file is only valid for a
/// plain TRLC identifier.
pub fn validate_trlc_package(flag: &str, package: &str) -> Result<(), Box<dyn std::error::Error>> {
    if is_trlc_identifier(package) {
        Ok(())
    } else {
        Err(format!(
            "--{flag} must be a TRLC identifier (an ASCII letter followed by letters, digits \
             and '_'), got: {package:?}"
        )
        .into())
    }
}

pub fn source_path_for_output(path: &Path) -> String {
    let workspace_root = std::env::var("BUILD_WORKSPACE_DIRECTORY")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok());

    // Prefer a stable, workspace-relative path so the embedded `source` is
    // reproducible across machines and portable outside Bazel.
    if let Some(root) = workspace_root {
        if let Ok(rel) = path.strip_prefix(&root) {
            return rel.to_string_lossy().into_owned();
        }
    }

    // Already relative: keep it verbatim (still portable).
    if path.is_relative() {
        return path.to_string_lossy().into_owned();
    }

    // Absolute path outside the workspace: never embed the machine-specific
    // absolute path — it leaks local directory layout and breaks build
    // reproducibility. Fall back to the file name only.
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Compute the corrected, stable `source_file` string (embedded *inside*
/// outputs) and the `Rc<PathBuf>` that parsers use to stamp `SourceLocation`s,
/// for one input `path`.
///
/// With `--source-name`, both derive from that name so every `SourceLocation`
/// carries the corrected path too. Without it, `source_file` is
/// `source_path_for_output(path)` and `SourceLocation`s keep using `path`
/// itself.
pub fn output_source_path(source_name: Option<&str>, path: &Rc<PathBuf>) -> (String, Rc<PathBuf>) {
    match source_name {
        Some(name) => {
            let source_file = name.to_string();
            let source_path = Rc::new(PathBuf::from(&source_file));
            (source_file, source_path)
        }
        None => (source_path_for_output(path), Rc::clone(path)),
    }
}

/// Compute the path whose file stem drives every output *filename* for one
/// input `path` (the `write_*_to_file` helpers all derive their output name
/// from a passed-in path's file stem alone).
///
/// `output_stem` (`--output-stem`) overrides the stem outright; otherwise
/// `path` itself is used unchanged.
pub fn output_naming_path<'a>(output_stem: Option<&str>, path: &'a Path) -> Cow<'a, Path> {
    match output_stem {
        Some(stem) => Cow::Owned(path.with_file_name(stem)),
        None => Cow::Borrowed(path),
    }
}

/// Resolve a `--file`/folder-entry path to an absolute path, checking it is
/// an existing `.puml` file.
fn resolve_puml_file(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let abs_path = resolve_path(path)?;

    if !abs_path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()).into());
    }
    if abs_path.extension().and_then(|ext| ext.to_str()) != Some("puml") {
        return Err(format!("File is not a .puml file: {}", path.display()).into());
    }
    Ok(abs_path)
}

fn collect_puml_files_from_folder(
    dir: &Path,
    file_list: &mut HashSet<Rc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let abs_dir = resolve_path(dir)?;

    if !abs_dir.is_dir() {
        return Err(format!("Path is not a directory: {}", dir.display()).into());
    }
    collect_puml_files(&abs_dir, file_list)?;
    Ok(())
}

fn collect_puml_files(
    dir: &Path,
    file_list: &mut HashSet<Rc<PathBuf>>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_puml_files(&path, file_list)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("puml") {
            file_list.insert(Rc::new(path.to_path_buf()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_must_be_a_trlc_identifier() {
        assert!(validate_trlc_package("api-package", "Pkg_api").is_ok());

        let error = validate_trlc_package("api-package", "_pkg").unwrap_err();
        assert!(error
            .to_string()
            .starts_with("--api-package must be a TRLC identifier"));
        assert!(validate_trlc_package("api-package", "").is_err());
        assert!(validate_trlc_package("api-package", "a.b").is_err());
        assert!(validate_trlc_package("api-package", "1a").is_err());
    }
}
