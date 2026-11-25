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

//! Per-diagram link facts: the elements a diagram elaborates (defines) and
//! the elements it only mentions (references).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;

use class_diagram::ClassDiagram;
use class_fbs::class_metamodel as fb_class;
use component_fbs::component as fb_component;
use sequence_fbs::sequence_metamodel as fb_sequence;
use sequence_logic::SequenceTree;

use crate::readers::{ClassDiagramReader, Reader, SequenceDiagramReader};

/// An element of a diagram.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactEntry {
    /// PlantUML code of the element in its own diagram (the `url of` key).
    pub alias: String,
    /// Fully qualified id, matched across diagrams.
    pub id: String,
}

/// Defines and references of one diagram, each sorted by id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagramFacts {
    /// Source path of the diagram file.
    pub source: String,
    /// Elements elaborated here.
    pub defines: Vec<FactEntry>,
    /// Elements only mentioned here.
    pub references: Vec<FactEntry>,
    /// Ids in `defines` that stem from a namespace wrapper.
    pub namespace_defines: BTreeSet<String>,
}

/// A component diagram element.
#[derive(Clone, Debug, Default)]
pub struct ComponentElement {
    pub id: String,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub parent_id: Option<String>,
}

fn id_leaf(id: &str) -> &str {
    id.rsplit('.').next().unwrap_or(id)
}

fn sorted(mut entries: Vec<FactEntry>) -> Vec<FactEntry> {
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

/// One facts entry per source. The same diagram may come with several
/// architectural designs; differing facts for one source are an error.
fn unique_by_source(facts: Vec<DiagramFacts>) -> Result<Vec<DiagramFacts>, String> {
    let mut unique: Vec<DiagramFacts> = Vec::with_capacity(facts.len());
    for fact in facts {
        match unique.iter().find(|known| known.source == fact.source) {
            Some(known) if *known == fact => {}
            Some(_) => {
                return Err(format!(
                    "Diagram {} is given more than once with different content",
                    fact.source
                ))
            }
            None => unique.push(fact),
        }
    }
    Ok(unique)
}

impl DiagramFacts {
    /// Facts of component, class and sequence diagram FlatBuffers. Other
    /// diagram kinds have no facts. The FBS `source` names a diagram; without
    /// it, the path does.
    pub fn read_files(paths: &[String]) -> Result<Vec<DiagramFacts>, String> {
        let mut out = Vec::new();
        for path in paths {
            let data = fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
            // Placeholder files of unsupported diagram kinds are empty.
            if data.len() < 8 {
                log::debug!("{path}: no FlatBuffer");
            } else if fb_component::component_graph_buffer_has_identifier(&data) {
                out.push(read_component(path, &data)?);
            } else if fb_class::class_diagram_buffer_has_identifier(&data) {
                for diagram in <ClassDiagramReader as Reader>::read(std::slice::from_ref(path))? {
                    out.push(Self::from_class(&diagram, path));
                }
            } else if fb_sequence::sequence_diagram_buffer_has_identifier(&data) {
                for diagram in
                    <SequenceDiagramReader as Reader>::read(std::slice::from_ref(path))?.diagrams
                {
                    out.push(Self::from_sequence(&diagram, path));
                }
            } else {
                log::debug!("{path}: diagram kind without links");
            }
        }
        unique_by_source(out)
    }

    /// An element is a define when it has children or the `@startuml` name
    /// is its alias or name; else it is a reference.
    pub fn from_components(
        elements: &[ComponentElement],
        source: &str,
        diagram_name: Option<&str>,
    ) -> Self {
        let has_children: HashSet<&str> = elements
            .iter()
            .filter_map(|e| e.parent_id.as_deref())
            .collect();

        let mut defines = Vec::new();
        let mut references = Vec::new();
        for element in elements {
            let entry = FactEntry {
                alias: element
                    .alias
                    .as_deref()
                    .or(element.name.as_deref())
                    .unwrap_or(&element.id)
                    .to_string(),
                id: element.id.clone(),
            };
            let named = diagram_name.is_some_and(|name| {
                element.alias.as_deref() == Some(name) || element.name.as_deref() == Some(name)
            });
            if named || has_children.contains(element.id.as_str()) {
                defines.push(entry);
            } else {
                references.push(entry);
            }
        }

        Self {
            source: source.to_string(),
            defines: sorted(defines),
            references: sorted(references),
            namespace_defines: BTreeSet::new(),
        }
    }

    /// An entity is a define when it has members or the `@startuml` name is
    /// its name or last id segment; else it is a reference. A namespace with
    /// a defined child is a define too (a namespace define). Relation
    /// endpoints that are no define and no namespace are references.
    pub fn from_class(diagram: &ClassDiagram, fallback_source: &str) -> Self {
        let source = diagram.source.as_deref().unwrap_or(fallback_source);
        let diagram_name = (!diagram.name.is_empty()).then_some(diagram.name.as_str());

        let mut defines = Vec::new();
        let mut references = Vec::new();
        let mut define_ids: HashSet<String> = HashSet::new();
        let mut reference_ids: HashSet<String> = HashSet::new();
        let name_by_id: HashMap<&str, &str> = diagram
            .entities
            .iter()
            .map(|e| (e.id.as_str(), e.name.as_str()))
            .collect();
        let namespace_ids: HashSet<&str> = diagram
            .entities
            .iter()
            .filter_map(|e| e.enclosing_namespace_id.as_deref())
            .collect();
        let mut namespaces_with_define: BTreeSet<&str> = BTreeSet::new();

        for entity in &diagram.entities {
            let has_members = !entity.methods.is_empty() || !entity.variables.is_empty();
            let named =
                diagram_name.is_some_and(|name| name == entity.name || name == id_leaf(&entity.id));
            let entry = FactEntry {
                alias: entity.name.clone(),
                id: entity.id.clone(),
            };
            if has_members || named {
                define_ids.insert(entity.id.clone());
                defines.push(entry);
                if let Some(namespace) = entity.enclosing_namespace_id.as_deref() {
                    namespaces_with_define.insert(namespace);
                }
            } else if reference_ids.insert(entity.id.clone()) {
                references.push(entry);
            }
        }

        // A namespace has no entity of its own; a unit drawn only as
        // `namespace unit_1 { class Foo {...} }` must still be linkable.
        let mut namespace_defines = BTreeSet::new();
        for namespace in namespaces_with_define {
            if define_ids.contains(namespace) || reference_ids.contains(namespace) {
                continue;
            }
            define_ids.insert(namespace.to_string());
            namespace_defines.insert(namespace.to_string());
            defines.push(FactEntry {
                alias: id_leaf(namespace).to_string(),
                id: namespace.to_string(),
            });
        }

        for relationship in diagram.entities.iter().flat_map(|e| &e.relationships) {
            for endpoint in [&relationship.source, &relationship.target] {
                if namespace_ids.contains(endpoint.as_str())
                    || define_ids.contains(endpoint)
                    || !reference_ids.insert(endpoint.clone())
                {
                    continue;
                }
                let alias = match name_by_id.get(endpoint.as_str()) {
                    Some(name) => name.to_string(),
                    None => {
                        log::warn!(
                            "class diagram relationship endpoint {endpoint:?} does not match any \
                             known entity or namespace in {source}; using its raw id"
                        );
                        endpoint.clone()
                    }
                };
                references.push(FactEntry {
                    alias,
                    id: endpoint.clone(),
                });
            }
        }

        Self {
            source: source.to_string(),
            defines: sorted(defines),
            references: sorted(references),
            namespace_defines,
        }
    }

    /// Every participant is a reference by its uid.
    pub fn from_sequence(diagram: &SequenceTree, fallback_source: &str) -> Self {
        let references = diagram
            .participants
            .iter()
            .map(|participant| FactEntry {
                alias: participant
                    .alias
                    .clone()
                    .unwrap_or_else(|| participant.display_name.clone()),
                id: participant.uid.clone(),
            })
            .collect();

        Self {
            source: diagram
                .source
                .clone()
                .unwrap_or_else(|| fallback_source.to_string()),
            defines: Vec::new(),
            references: sorted(references),
            namespace_defines: BTreeSet::new(),
        }
    }
}

fn read_component(path: &str, data: &[u8]) -> Result<DiagramFacts, String> {
    let graph = flatbuffers::root::<fb_component::ComponentGraph>(data)
        .map_err(|e| format!("Failed to parse FlatBuffer {path}: {e}"))?;
    let mut elements = Vec::new();
    for entry in graph.components().iter().flat_map(|entries| entries.iter()) {
        let component = entry.value().ok_or_else(|| {
            format!(
                "FlatBuffer entry with key {:?} has null value in {path}",
                entry.key()
            )
        })?;
        elements.push(ComponentElement {
            id: component.id().unwrap_or_default().to_string(),
            name: component.name().map(str::to_string),
            alias: component.alias().map(str::to_string),
            parent_id: component.parent_id().map(str::to_string),
        });
    }
    Ok(DiagramFacts::from_components(
        &elements,
        graph.source().unwrap_or(path),
        graph.name(),
    ))
}

#[cfg(test)]
#[path = "test/facts_test.rs"]
mod tests;
