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

//! Reader for sequence-diagram FlatBuffer exports used by design verification.

use std::fs;

use sequence_fbs::sequence_metamodel as fb_sequence;
use sequence_logic::{
    Condition, ConditionType, Event, Interaction, Return, SequenceNode, SequenceTree,
};

use crate::models::{SequenceDiagramInput, SequenceDiagramInputs};
use crate::readers::Reader;

pub struct SequenceDiagramReader;

impl Reader for SequenceDiagramReader {
    type Input = [String];
    type Raw = SequenceDiagramInputs;
    type Error = String;

    fn read(input: &Self::Input) -> Result<Self::Raw, Self::Error> {
        let mut diagrams = Vec::new();

        for path in input {
            let data = fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;

            let diagram = flatbuffers::root::<fb_sequence::SequenceDiagram>(&data)
                .map_err(|e| format!("Failed to parse sequence FlatBuffer {path}: {e}"))?;

            let root_interactions = if let Some(nodes) = diagram.root_interactions() {
                let mut parsed_nodes = Vec::with_capacity(nodes.len());
                for (index, node) in nodes.iter().enumerate() {
                    parsed_nodes.push(
                        read_node(node, &format!("{path}:root[{index}]"))
                            .map_err(|e| format!("Failed to parse sequence node: {e}"))?,
                    );
                }
                parsed_nodes
            } else {
                Vec::new()
            };

            let source_files = diagram
                .source_files()
                .map(|values| values.iter().map(|f| f.to_string()).collect::<Vec<_>>())
                .unwrap_or_default();

            diagrams.push(SequenceDiagramInput {
                tree: SequenceTree {
                    name: diagram.name().map(|s| s.to_string()),
                    root_interactions,
                },
                source_files,
                version: diagram.version().map(|s| s.to_string()),
            });
        }

        Ok(SequenceDiagramInputs { diagrams })
    }
}

fn read_node(node: fb_sequence::SequenceNode<'_>, node_path: &str) -> Result<SequenceNode, String> {
    let event = match node.event_type() {
        fb_sequence::Event::Interaction => {
            let interaction = node.event_as_interaction().ok_or_else(|| {
                format!(
                    "{node_path}: event_type is Interaction, but interaction payload is missing"
                )
            })?;
            Event::Interaction(Interaction {
                caller: interaction.caller().to_string(),
                callee: interaction.callee().to_string(),
                method: interaction
                    .method()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            })
        }
        fb_sequence::Event::Return => {
            let ret = node.event_as_return().ok_or_else(|| {
                format!("{node_path}: event_type is Return, but return payload is missing")
            })?;
            Event::Return(Return {
                caller: ret.caller().to_string(),
                callee: ret.callee().to_string(),
                return_content: ret
                    .return_content()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            })
        }
        fb_sequence::Event::Condition => {
            let condition = node.event_as_condition().ok_or_else(|| {
                format!("{node_path}: event_type is Condition, but condition payload is missing")
            })?;
            Event::Condition(Condition {
                condition_type: map_condition_type(condition.condition_type(), node_path)?,
                condition_value: condition
                    .condition_value()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            })
        }
        fb_sequence::Event::NONE => {
            return Err(format!("{node_path}: event_type is NONE"));
        }
        _ => {
            return Err(format!(
                "{node_path}: unsupported event_type {:?}",
                node.event_type()
            ));
        }
    };

    let branches_node = if let Some(children) = node.branches_node() {
        let mut parsed_children = Vec::with_capacity(children.len());
        for (index, child) in children.iter().enumerate() {
            parsed_children.push(read_node(child, &format!("{node_path}.branches[{index}]"))?);
        }
        parsed_children
    } else {
        Vec::new()
    };

    Ok(SequenceNode {
        event,
        branches_node,
    })
}

fn map_condition_type(
    value: fb_sequence::ConditionType,
    node_path: &str,
) -> Result<ConditionType, String> {
    match value {
        fb_sequence::ConditionType::Opt => Ok(ConditionType::Opt),
        fb_sequence::ConditionType::Alt => Ok(ConditionType::Alt),
        fb_sequence::ConditionType::Loop => Ok(ConditionType::Loop),
        fb_sequence::ConditionType::Par => Ok(ConditionType::Par),
        fb_sequence::ConditionType::Par2 => Ok(ConditionType::Par2),
        fb_sequence::ConditionType::Break => Ok(ConditionType::Break),
        fb_sequence::ConditionType::Critical => Ok(ConditionType::Critical),
        fb_sequence::ConditionType::Else => Ok(ConditionType::Else),
        fb_sequence::ConditionType::Also => Ok(ConditionType::Also),
        fb_sequence::ConditionType::End => Ok(ConditionType::End),
        fb_sequence::ConditionType::Group => Ok(ConditionType::Group),
        _ => Err(format!(
            "{node_path}: unsupported condition_type {:?}",
            value
        )),
    }
}
