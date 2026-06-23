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

//! Reader for component-level PlantUML FlatBuffer exports used by architecture
//! validation.

use std::fs;

use component_fbs::component as fb_component;

use crate::models::{
    ComponentDiagramElementType, ComponentDiagramInput, ComponentDiagramInputs,
    ComponentDiagramRelation,
};
use crate::readers::Reader;

pub struct ComponentDiagramReader;

fn map_element_type(value: fb_component::ComponentType) -> Option<ComponentDiagramElementType> {
    match value {
        fb_component::ComponentType::Component => Some(ComponentDiagramElementType::Component),
        fb_component::ComponentType::Package => Some(ComponentDiagramElementType::Package),
        fb_component::ComponentType::Interface => Some(ComponentDiagramElementType::Interface),
        _ => None,
    }
}

fn read_relations(
    component: &fb_component::LogicComponent<'_>,
    context: &str,
) -> Result<Vec<ComponentDiagramRelation>, String> {
    component
        .relations()
        .map(|relations| {
            relations
                .iter()
                .map(|relation| {
                    let target = relation
                        .target()
                        .ok_or_else(|| format!("Component relation missing target in {context}"))?;

                    Ok(ComponentDiagramRelation {
                        target: target.to_string(),
                        annotation: relation.annotation().map(|value| value.to_string()),
                        relation_type: relation
                            .relation_type()
                            .variant_name()
                            .map(|value| value.to_string()),
                        source_role: relation
                            .source_role()
                            .variant_name()
                            .map(|value| value.to_string()),
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()
        .map(|relations| relations.unwrap_or_default())
}

impl ComponentDiagramReader {
    /// Read all `Component` and `Package` entities from the given FlatBuffers
    /// binary files.
    ///
    /// Files that carry a known non-component file identifier (e.g. `CLSD` for
    /// class diagrams or `SEQD` for sequence diagrams) are silently skipped so
    /// that callers can pass all architectural-design FlatBuffers without
    /// pre-filtering by diagram type.
    pub fn read(paths: &[String]) -> Result<ComponentDiagramInputs, String> {
        let mut out = Vec::new();

        for path in paths {
            let data = fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;

            // FlatBuffers stores the file identifier at bytes 4-7 when one is
            // present.  Component diagrams are written without an identifier
            // (builder.finish(root, None)), so bytes 4-7 are regular data.
            // Class diagrams ("CLSD") and sequence diagrams ("SEQD") carry an
            // explicit identifier.  Skip such files here; they are not
            // component diagrams and must not be parsed with the component schema.
            if data.len() >= 8 {
                let file_id = &data[4..8];
                if file_id == b"CLSD" || file_id == b"SEQD" {
                    continue;
                }
            }

            let graph = flatbuffers::root::<fb_component::ComponentGraph>(&data)
                .map_err(|e| format!("Failed to parse FlatBuffer {path}: {e}"))?;

            if let Some(entries) = graph.components() {
                for entry in entries.iter() {
                    if let Some(comp) = entry.value() {
                        if let Some(element_type) = map_element_type(comp.comp_type()) {
                            let context =
                                format!("{path}:component:{}", comp.id().unwrap_or_default());
                            out.push(ComponentDiagramInput {
                                id: comp.id().unwrap_or_default().to_string(),
                                alias: comp.alias().map(|s| s.to_string()),
                                parent_id: comp.parent_id().map(|s| s.to_string()),
                                element_type,
                                stereotype: comp.stereotype().map(|s| s.to_string()),
                                relations: read_relations(&comp, &context)?,
                            });
                        }
                    } else {
                        return Err(format!(
                            "FlatBuffer entry with key {:?} has null value in {path} (corrupted or truncated file)",
                            entry.key()
                        ));
                    }
                }
            }
        }

        Ok(ComponentDiagramInputs { entities: out })
    }
}

impl Reader for ComponentDiagramReader {
    type Input = [String];
    type Raw = ComponentDiagramInputs;
    type Error = String;

    fn read(input: &Self::Input) -> Result<Self::Raw, Self::Error> {
        ComponentDiagramReader::read(input)
    }
}
