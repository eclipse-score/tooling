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

use std::collections::{HashMap, HashSet};
use std::fs;

use clap::{Parser, ValueEnum};
use env_logger::Builder;

use class_fbs::class_metamodel as fb_class;
use component_fbs::component as fb_component;
use sequence_fbs::sequence_metamodel as fb_sequence;

// ---------------------------------------------------------------------------
// Log level
// ---------------------------------------------------------------------------

/// CLI-visible log level (mirrors the parser's convention).
#[derive(Copy, Clone, ValueEnum, Debug)]
enum CliLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl CliLogLevel {
    fn to_level_filter(self) -> log::LevelFilter {
        match self {
            CliLogLevel::Error => log::LevelFilter::Error,
            CliLogLevel::Warn => log::LevelFilter::Warn,
            CliLogLevel::Info => log::LevelFilter::Info,
            CliLogLevel::Debug => log::LevelFilter::Debug,
            CliLogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

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

    /// Log level: error, warn, info, debug, trace
    #[arg(long, value_enum, default_value = "warn")]
    log_level: CliLogLevel,
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// A component extracted from a FlatBuffers diagram.
#[derive(Debug)]
struct DiagramComponent {
    alias: String,
    /// Fully qualified identifier (e.g. `package.Component`), registered as
    /// an additional key in the top-level index so other diagrams can match
    /// by FQN.
    id: Option<String>,
    parent_id: Option<String>,
}

/// All components from a single diagram file.
#[derive(Debug)]
struct DiagramInfo {
    source_file: String,
    /// Optional diagram name (from `@startuml <name>` or class diagram name).
    /// When present, this name is registered as an additional top-level alias
    /// so other diagrams can link to this diagram by its title.
    diagram_name: Option<String>,
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

/// File identifier bytes written by the component serializer ("COMP" at bytes 4–7).
/// Class ("CLSD") and sequence ("SEQD") diagrams carry a different identifier and
/// are intentionally skipped — they do not participate in component-level linking.
const COMPONENT_FILE_ID: &[u8; 4] = b"COMP";

fn parse_component_diagram(path: &str, data: &[u8]) -> Result<DiagramInfo, String> {
    if data.is_empty() {
        return Err(format!("Empty file (placeholder): {path}"));
    }

    // Reject non-component FlatBuffers (class / sequence diagrams) early so we
    // never mis-parse their binary layout as a ComponentGraph.
    if data.len() < 8 || &data[4..8] != COMPONENT_FILE_ID {
        let found = if data.len() >= 8 {
            String::from_utf8_lossy(&data[4..8]).into_owned()
        } else {
            "<too short>".to_string()
        };
        return Err(format!(
            "Not a component diagram FlatBuffer (expected id 'COMP', found '{found}'): {path}"
        ));
    }

    let graph = flatbuffers::root::<fb_component::ComponentGraph>(data)
        .map_err(|e| format!("Failed to parse FlatBuffer {path}: {e}"))?;

    let source_file = graph
        .source_file()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Missing source_file in FlatBuffer: {path}"))?;

    let mut components = Vec::new();
    let mut seen_relation_targets: HashSet<String> = HashSet::new();
    if let Some(entries) = graph.components() {
        for entry in entries.iter() {
            let Some(comp) = entry.value() else {
                continue;
            };
            let alias = comp.alias().or(comp.name()).unwrap_or_default().to_string();
            if alias.is_empty() {
                continue;
            }
            let id = comp
                .id()
                .filter(|s| !s.is_empty() && *s != alias)
                .map(|s| s.to_string());
            let parent_id = comp.parent_id().map(|s| s.to_string());

            // When the component has both an alias and a distinct name,
            // register the name as an additional linkable element.
            if comp.alias().is_some() {
                if let Some(name) = comp.name().filter(|n| !n.is_empty() && *n != alias) {
                    components.push(DiagramComponent {
                        alias: name.to_string(),
                        id: None,
                        parent_id: parent_id.clone(),
                    });
                }
            }

            components.push(DiagramComponent {
                alias,
                id,
                parent_id,
            });

            // Extract relation targets so components referenced via
            // dependency arrows (e.g. `[A] --> [B]`) become linkable.
            if let Some(relations) = comp.relations() {
                for rel in relations.iter() {
                    if let Some(target) = rel.target() {
                        if !target.is_empty() && seen_relation_targets.insert(target.to_string()) {
                            components.push(DiagramComponent {
                                alias: target.to_string(),
                                id: None,
                                parent_id: None,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(DiagramInfo {
        source_file,
        diagram_name: None,
        components,
    })
}

/// File identifier for class diagram FlatBuffers ("CLSD").
const CLASS_FILE_ID: &[u8; 4] = b"CLSD";

fn parse_class_diagram(path: &str, data: &[u8]) -> Result<DiagramInfo, String> {
    if data.is_empty() {
        return Err(format!("Empty file (placeholder): {path}"));
    }

    if data.len() < 8 || &data[4..8] != CLASS_FILE_ID {
        return Err(format!("Not a class diagram FlatBuffer: {path}"));
    }

    let diagram = flatbuffers::root::<fb_class::ClassDiagram>(data)
        .map_err(|e| format!("Failed to parse class FlatBuffer {path}: {e}"))?;

    // source_files is a vector; take the first non-empty entry.
    let source_file = diagram
        .source_files()
        .and_then(|v| v.iter().find(|s| !s.is_empty()).map(|s| s.to_string()))
        .ok_or_else(|| format!("Missing source_files in class FlatBuffer: {path}"))?;

    // The diagram name (from @startuml <name>) can serve as a link target.
    // ClassDiagram.name is required (non-Option &str).
    let raw_name = diagram.name();
    let diagram_name = if !raw_name.is_empty() && raw_name != source_file {
        Some(raw_name.to_string())
    } else {
        None
    };

    let mut components = Vec::new();
    if let Some(entities) = diagram.entities() {
        for entity in entities.iter() {
            let name = entity.name().to_string();
            if name.is_empty() {
                continue;
            }
            let id_str = entity.id();
            let id = if !id_str.is_empty() && id_str != name {
                Some(id_str.to_string())
            } else {
                None
            };
            components.push(DiagramComponent {
                alias: name,
                id,
                parent_id: entity.enclosing_namespace_id().map(|s| s.to_string()),
            });

            // Extract relationship targets defined on this entity so that
            // referenced classes (inheritance, composition, etc.) become linkable.
            if let Some(rels) = entity.relationships() {
                for rel in rels.iter() {
                    let target = rel.target();
                    if !target.is_empty() {
                        components.push(DiagramComponent {
                            alias: target.to_string(),
                            id: None,
                            parent_id: None,
                        });
                    }
                }
            }
        }
    }

    // Also extract targets from diagram-level relationships (outside entities).
    if let Some(rels) = diagram.relationships() {
        for rel in rels.iter() {
            let target = rel.target();
            if !target.is_empty() {
                components.push(DiagramComponent {
                    alias: target.to_string(),
                    id: None,
                    parent_id: None,
                });
            }
            let source = rel.source();
            if !source.is_empty() {
                components.push(DiagramComponent {
                    alias: source.to_string(),
                    id: None,
                    parent_id: None,
                });
            }
        }
    }

    Ok(DiagramInfo {
        source_file,
        diagram_name,
        components,
    })
}

/// File identifier for sequence diagram FlatBuffers ("SEQD").
const SEQUENCE_FILE_ID: &[u8; 4] = b"SEQD";

fn parse_sequence_diagram(path: &str, data: &[u8]) -> Result<DiagramInfo, String> {
    if data.is_empty() {
        return Err(format!("Empty file (placeholder): {path}"));
    }

    if data.len() < 8 || &data[4..8] != SEQUENCE_FILE_ID {
        return Err(format!("Not a sequence diagram FlatBuffer: {path}"));
    }

    let diagram = flatbuffers::root::<fb_sequence::SequenceDiagram>(data)
        .map_err(|e| format!("Failed to parse sequence FlatBuffer {path}: {e}"))?;

    let source_file = diagram
        .source_files()
        .and_then(|v| v.iter().find(|s| !s.is_empty()).map(|s| s.to_string()))
        .ok_or_else(|| format!("Missing source_files in sequence FlatBuffer: {path}"))?;

    let diagram_name = diagram
        .name()
        .filter(|n| !n.is_empty() && *n != source_file)
        .map(|n| n.to_string());

    // Extract unique participants from interactions as top-level "components"
    // so they can be linked to their defining diagrams (component or class).
    let mut participants: HashSet<String> = HashSet::new();
    if let Some(nodes) = diagram.root_interactions() {
        collect_sequence_participants(&nodes, &mut participants);
    }

    let components = participants
        .into_iter()
        .map(|name| DiagramComponent {
            alias: name,
            id: None,
            parent_id: None,
        })
        .collect();

    Ok(DiagramInfo {
        source_file,
        diagram_name,
        components,
    })
}

/// Recursively collect caller/callee names from sequence diagram nodes.
fn collect_sequence_participants(
    nodes: &flatbuffers::Vector<flatbuffers::ForwardsUOffset<fb_sequence::SequenceNode>>,
    participants: &mut HashSet<String>,
) {
    for node in nodes.iter() {
        match node.event_type() {
            fb_sequence::Event::Interaction => {
                if let Some(interaction) = node.event_as_interaction() {
                    let caller = interaction.caller();
                    if !caller.is_empty() && !participants.contains(caller) {
                        participants.insert(caller.to_string());
                    }
                    let callee = interaction.callee();
                    if !callee.is_empty() && !participants.contains(callee) {
                        participants.insert(callee.to_string());
                    }
                }
            }
            fb_sequence::Event::Return => {
                if let Some(ret) = node.event_as_return() {
                    let caller = ret.caller();
                    if !caller.is_empty() && !participants.contains(caller) {
                        participants.insert(caller.to_string());
                    }
                    let callee = ret.callee();
                    if !callee.is_empty() && !participants.contains(callee) {
                        participants.insert(callee.to_string());
                    }
                }
            }
            _ => {}
        }
        // Recurse into child nodes
        if let Some(branches) = node.branches_node() {
            collect_sequence_participants(&branches, participants);
        }
    }
}

/// Attempt to read a FlatBuffer file as a component, class, or sequence
/// diagram and return the extracted [`DiagramInfo`].
fn read_any_diagram(path: &str) -> Result<DiagramInfo, String> {
    let data = fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
    if data.is_empty() {
        return Err(format!("Empty file (placeholder): {path}"));
    }
    let id = if data.len() >= 8 {
        &data[4..8]
    } else {
        &[] as &[u8]
    };
    match id {
        b"COMP" => parse_component_diagram(path, &data),
        b"CLSD" => parse_class_diagram(path, &data),
        b"SEQD" => parse_sequence_diagram(path, &data),
        other => {
            let s = String::from_utf8_lossy(other);
            Err(format!("Unsupported FlatBuffer type '{s}': {path}"))
        }
    }
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
/// Additionally, diagrams with a `diagram_name` register that name as a
/// virtual top-level alias, enabling components to link to diagrams by title.
fn generate_links(diagrams: &[DiagramInfo]) -> Vec<LinkEntry> {
    // Index: alias → list of diagrams where that alias is a top-level component.
    // Uses borrowed &str keys since `diagrams` outlives the index.
    let mut top_level_index: HashMap<&str, Vec<&str>> = HashMap::new();
    for diagram in diagrams {
        for comp in &diagram.components {
            if comp.parent_id.is_none() {
                top_level_index
                    .entry(comp.alias.as_str())
                    .or_default()
                    .push(&diagram.source_file);
                // Also register the FQN so other diagrams can match by
                // fully qualified identifier.
                if let Some(id) = &comp.id {
                    top_level_index
                        .entry(id.as_str())
                        .or_default()
                        .push(&diagram.source_file);
                }
            }
        }
        // Register the diagram name as a virtual top-level alias so that
        // components in other diagrams can link to this diagram by title.
        if let Some(name) = &diagram.diagram_name {
            top_level_index
                .entry(name.as_str())
                .or_default()
                .push(&diagram.source_file);
        }
    }

    let mut links = Vec::new();

    for diagram in diagrams {
        for comp in &diagram.components {
            if let Some(target_diagrams) = top_level_index.get(comp.alias.as_str()) {
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
    Builder::new()
        .filter_level(args.log_level.to_level_filter())
        .init();

    if args.fbs_files.is_empty() {
        return Err("No .fbs.bin files provided. Use --fbs-files <file> ...".into());
    }

    let mut diagrams = Vec::new();
    for fbs_path in &args.fbs_files {
        match read_any_diagram(fbs_path) {
            Ok(diagram) => {
                log::info!(
                    "Read {} components from {}",
                    diagram.components.len(),
                    diagram.source_file
                );
                diagrams.push(diagram);
            }
            Err(e) => {
                log::warn!("Skipping {}: {}", fbs_path, e);
            }
        }
    }

    let links = generate_links(&diagrams);
    log::info!("Generated {} link(s)", links.len());

    let output = LinksJson { links };
    let json = serde_json::to_string_pretty(&output)?;
    fs::write(&args.output, &json)?;
    log::debug!("Written to {}", args.output);

    Ok(())
}
