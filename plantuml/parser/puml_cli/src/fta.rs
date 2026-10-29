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

//! `--fta-output-dir` mode: Fault-Tree-Analysis diagrams to `RootCause` TRLC.

use log::debug;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use puml_fta::{merge_root_causes, render_trlc_stub, FaultTree, FtaModel};
use puml_idmap::{write_idmap_to_file, IdMapModel};
use puml_parser::{DiagramParser, ProcedureParserService};
use puml_utils::LogLevel;

use crate::inputs::{collect_input_files, source_path_for_output};
use crate::sinks::{create_output_dir, IdmapOutputs};
use crate::Args;

/// FTA processing pipeline: parse the fault-tree macro calls straight from
/// each source diagram (the metamodel's ``!procedure`` definitions carry no
/// fault-tree topology themselves) and emit the generated ``fta_events.trlc``
/// stub. Diagrams are analysis-only here; they are staged/rendered as-authored
/// by the Bazel rule, not rewritten by this pipeline.
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

    let mut all_trees: Vec<FaultTree> = Vec::new();
    // Fault trees reference each diagram by basename; two inputs sharing a
    // basename (in different directories) would be indistinguishable downstream.
    let mut seen_basenames: HashSet<String> = HashSet::new();
    // Independent of the basename check above, which only fires on an exact
    // basename match: two inputs with the same file *stem* would overwrite
    // each other's `<stem>.idmap.json`.
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

        // Analysis only: the fault-tree topology comes entirely from the
        // `$FailureMode(...)` / `$RootCause(...)` / gate macro *calls*, which the
        // procedure parser reads straight from the source (the `!include
        // fta_metamodel.puml` line is inert text).  The diagram is rendered
        // as-authored by Sphinx/PlantUML, which finds the metamodel via the
        // toolchain's global `plantuml.include.path`.
        let source = fs::read_to_string(file.as_path())?;
        let parsed = ProcedureParserService.parse_file(file, &source, log_level)?;
        let model =
            FtaModel::from_procedure_file(&parsed).map_err(|e| format!("FTA {basename}: {e}"))?;
        all_trees.extend(model.fault_trees(basename)?);

        if let Some(outputs) = idmap_outputs.as_mut() {
            outputs.claim(file, "FTA input")?;
            let source_file = source_path_for_output(file);
            write_idmap_to_file(
                IdMapModel::Fta(&model),
                file,
                Some(&source_file),
                None,
                outputs.dir(),
            )?;
        }

        debug!("Processed FTA diagram: {}", file.display());
    }

    let root_causes = merge_root_causes(&all_trees)?;
    fs::write(
        out.join("fta_events.trlc"),
        render_trlc_stub(fta_package, &root_causes),
    )?;
    Ok(())
}
