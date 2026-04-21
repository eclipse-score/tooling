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

//! PlantUML Linker
//!
//! Reads FlatBuffers `.fbs.bin` files produced by the PlantUML parser and
//! generates `plantuml_links.json` for the `clickable_plantuml` Sphinx extension.
//!
//! The tool correlates components across multiple diagrams: when a component
//! alias in diagram A matches a top-level component alias in diagram B, a
//! clickable link is created from A → B.

use std::collections::HashMap;
use std::fs;

use clap::Parser;

use component_fbs::component as fb_component;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(name = "linker")]
#[command(version = "1.0")]
#[command(
    about = "Generate plantuml_links.json from FlatBuffers diagram outputs",
    long_about = "Reads .fbs.bin files from the PlantUML parser and produces a \
                  plantuml_links.json file mapping component aliases to their \
                  detailed diagrams for the clickable_plantuml Sphinx extension."
)]
struct Args {
    /// FlatBuffers binary files to process (.fbs.bin)
    #[arg(long, num_args = 1..)]
    fbs_files: Vec<String>,

    /// Output JSON file path
    #[arg(long, default_value = "plantuml_links.json")]
    output: String,
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// A component extracted from a FlatBuffers diagram.
#[derive(Debug)]
struct DiagramComponent {
    alias: String,
    parent_id: Option<String>,
}

/// All components from a single diagram file.
#[derive(Debug)]
struct DiagramInfo {
    source_file: String,
    components: Vec<DiagramComponent>,
}

/// One entry in the output JSON `links` array.
#[derive(Debug, serde::Serialize)]
struct LinkEntry {
    source_file: String,
    source_id: String,
    target_file: String,
}

/// Root structure of the output JSON.
#[derive(Debug, serde::Serialize)]
struct LinksJson {
    links: Vec<LinkEntry>,
}

// ---------------------------------------------------------------------------
// FlatBuffers reading
// ---------------------------------------------------------------------------

fn read_diagram(path: &str) -> Result<DiagramInfo, String> {
    let data = fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;

    if data.is_empty() {
        return Err(format!("Empty file (placeholder): {path}"));
    }

    let graph = flatbuffers::root::<fb_component::ComponentGraph>(&data)
        .map_err(|e| format!("Failed to parse FlatBuffer {path}: {e}"))?;

    let source_file = graph
        .source_file()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Missing source_file in FlatBuffer: {path}"))?;

    let mut components = Vec::new();
    if let Some(entries) = graph.components() {
        for entry in entries.iter() {
            let Some(comp) = entry.value() else {
                continue;
            };
            let alias = comp.alias().or(comp.name()).unwrap_or_default().to_string();
            if alias.is_empty() {
                continue;
            }
            components.push(DiagramComponent {
                alias,
                parent_id: comp.parent_id().map(|s| s.to_string()),
            });
        }
    }

    Ok(DiagramInfo {
        source_file,
        components,
    })
}

// ---------------------------------------------------------------------------
// Link generation
// ---------------------------------------------------------------------------

/// Build links by matching component aliases across diagrams.
///
/// For each component alias in diagram A, if a top-level component (no parent)
/// with the same alias exists in diagram B, we create a link:
///   source_file = A,  source_id = alias,  target_file = B
///
/// A component is considered "top-level" if its `parent_id` is `None`.
fn generate_links(diagrams: &[DiagramInfo]) -> Vec<LinkEntry> {
    // Index: alias → list of diagrams where that alias is a top-level component
    let mut top_level_index: HashMap<String, Vec<&str>> = HashMap::new();
    for diagram in diagrams {
        for comp in &diagram.components {
            if comp.parent_id.is_none() {
                top_level_index
                    .entry(comp.alias.clone())
                    .or_default()
                    .push(&diagram.source_file);
            }
        }
    }

    let mut links = Vec::new();

    for diagram in diagrams {
        for comp in &diagram.components {
            if let Some(target_diagrams) = top_level_index.get(&comp.alias) {
                for &target_file in target_diagrams {
                    // Don't link a component to its own diagram.
                    if target_file == diagram.source_file {
                        continue;
                    }
                    links.push(LinkEntry {
                        source_file: diagram.source_file.clone(),
                        source_id: comp.alias.clone(),
                        target_file: target_file.to_string(),
                    });
                }
            }
        }
    }

    // Deduplicate: same (source_file, source_id, target_file) may appear
    // when a component is nested inside multiple parent scopes.
    links.sort_by(|a, b| {
        (&a.source_file, &a.source_id, &a.target_file).cmp(&(
            &b.source_file,
            &b.source_id,
            &b.target_file,
        ))
    });
    links.dedup_by(|a, b| {
        a.source_file == b.source_file
            && a.source_id == b.source_id
            && a.target_file == b.target_file
    });

    // PlantUML supports only one URL per alias — keep the first target
    // (alphabetically) for each (source_file, source_id) pair.
    links.dedup_by(|a, b| a.source_file == b.source_file && a.source_id == b.source_id);

    links
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.fbs_files.is_empty() {
        return Err("No .fbs.bin files provided. Use --fbs-files <file> ...".into());
    }

    let mut diagrams = Vec::new();
    for fbs_path in &args.fbs_files {
        match read_diagram(fbs_path) {
            Ok(diagram) => {
                eprintln!(
                    "Read {} components from {}",
                    diagram.components.len(),
                    diagram.source_file
                );
                diagrams.push(diagram);
            }
            Err(e) => {
                eprintln!("Warning: skipping {fbs_path}: {e}");
            }
        }
    }

    let links = generate_links(&diagrams);
    eprintln!("Generated {} link(s)", links.len());

    let output = LinksJson { links };
    let json = serde_json::to_string_pretty(&output)?;
    fs::write(&args.output, &json)?;
    eprintln!("Written to {}", args.output);

    Ok(())
}
