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

//! `--fta-output-dir` mode: Fault-Tree-Analysis diagrams to `RootCause` TRLC
//! and a JSON index of the review evidence and findings.

use log::{debug, warn};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use puml_fta::{render_index, render_trlc_stub, resolve_root_causes, FtaModel};
use puml_idmap::{write_idmap_to_file, IdMapModel};
use puml_parser::{DiagramParser, ProcedureParserService};
use puml_utils::LogLevel;

use crate::inputs::{collect_input_files, source_path_for_output};
use crate::sinks::{create_output_dir, IdmapOutputs};
use crate::Args;

/// Fully qualified failure mode names, one per line; blank lines are ignored.
fn read_known_failure_modes(path: &str) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

/// Parse the fault-tree macro calls of every input diagram, attribute the root
/// causes to failure modes across all diagrams, and write `fta_events.trlc`
/// and `fta_index.json` to `output_dir`.  The diagrams themselves are not
/// rewritten.
pub fn run_fta(
    args: &Args,
    output_dir: &str,
    fta_package: &str,
    log_level: LogLevel,
) -> Result<(), Box<dyn std::error::Error>> {
    let out = create_output_dir(output_dir)?;

    let inputs = collect_input_files(&args.file, args.folders.as_deref())?;
    if inputs.is_empty() {
        return Err("No valid PUML files found.".into());
    }

    // Process inputs in a deterministic order for reproducible output.
    let mut sorted: Vec<Rc<PathBuf>> = inputs.into_iter().collect();
    sorted.sort();

    let known_failure_modes = args
        .failure_modes_file
        .as_deref()
        .map(read_known_failure_modes)
        .transpose()?;

    let mut models: Vec<(String, FtaModel)> = Vec::new();
    let mut source_paths: BTreeMap<String, String> = BTreeMap::new();
    // Findings and TRLC records name a diagram by basename, so two inputs with
    // the same basename in different directories would be indistinguishable.
    let mut seen_basenames: HashSet<String> = HashSet::new();
    // Two inputs with the same file stem would overwrite each other's
    // `<stem>.idmap.json`.
    let mut idmap_outputs = args
        .idmap_output_dir
        .as_deref()
        .map(|dir| create_output_dir(dir).map(IdmapOutputs::new))
        .transpose()?;

    for file in &sorted {
        let basename = file.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            format!(
                "input path has no valid UTF-8 file name: {}",
                file.display()
            )
        })?;
        // The metamodel carries only `!procedure` definitions, no fault tree.
        if basename == "fta_metamodel.puml" {
            continue;
        }
        if !seen_basenames.insert(basename.to_string()) {
            return Err(format!(
                "duplicate FTA diagram basename {:?}: inputs in different directories \
                 would be indistinguishable in {}",
                basename, output_dir,
            )
            .into());
        }

        // The topology comes from the macro calls in the source; the
        // `!include fta_metamodel.puml` line is inert text.
        let source = fs::read_to_string(file.as_path())?;
        let parsed = ProcedureParserService.parse_file(file, &source, log_level)?;
        let model =
            FtaModel::from_procedure_file(&parsed).map_err(|e| format!("FTA {basename}: {e}"))?;
        let source_file = source_path_for_output(file);

        if let Some(outputs) = idmap_outputs.as_mut() {
            outputs.claim(file, "FTA input")?;
            write_idmap_to_file(
                IdMapModel::Fta {
                    model: &model,
                    package: fta_package,
                },
                file,
                Some(&source_file),
                None,
                outputs.dir(),
            )?;
        }

        source_paths.insert(basename.to_string(), source_file);
        models.push((basename.to_string(), model));
        debug!("Processed FTA diagram: {}", file.display());
    }

    let diagrams: Vec<(&str, &FtaModel)> = models
        .iter()
        .map(|(name, model)| (name.as_str(), model))
        .collect();
    let resolution = resolve_root_causes(&diagrams, known_failure_modes.as_ref())?;
    for finding in &resolution.findings {
        warn!(
            "FTA {}:{}: {}: {}",
            finding.diagram,
            finding.line,
            finding.kind.as_str(),
            finding.message
        );
    }
    fs::write(
        out.join("fta_events.trlc"),
        render_trlc_stub(fta_package, &resolution.root_causes),
    )?;
    fs::write(
        out.join("fta_index.json"),
        render_index(&resolution, &source_paths),
    )?;
    Ok(())
}
