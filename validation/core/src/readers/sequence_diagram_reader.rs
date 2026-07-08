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
    Condition, ConditionType, Event, Interaction, ParticipantType, Return, SequenceNode,
    SequenceParticipant, SequenceTree,
};

use crate::models::SequenceDiagramInputs;
use crate::readers::{to_source_location, Reader};

pub struct SequenceDiagramReader;

fn map_participant_type(value: fb_sequence::ParticipantType) -> Result<ParticipantType, String> {
    match value {
        fb_sequence::ParticipantType::Participant => Ok(ParticipantType::Participant),
        fb_sequence::ParticipantType::Actor => Ok(ParticipantType::Actor),
        fb_sequence::ParticipantType::Boundary => Ok(ParticipantType::Boundary),
        fb_sequence::ParticipantType::Control => Ok(ParticipantType::Control),
        fb_sequence::ParticipantType::Entity => Ok(ParticipantType::Entity),
        fb_sequence::ParticipantType::Queue => Ok(ParticipantType::Queue),
        fb_sequence::ParticipantType::Database => Ok(ParticipantType::Database),
        fb_sequence::ParticipantType::Collections => Ok(ParticipantType::Collections),
        other => Err(format!("unsupported participant type {:?}", other)),
    }
}

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

            let participants = if let Some(values) = diagram.participants() {
                let mut parsed_participants = Vec::with_capacity(values.len());
                for (index, participant) in values.iter().enumerate() {
                    parsed_participants.push(read_participant(
                        participant,
                        &format!("{path}:participants[{index}]"),
                    )?);
                }
                parsed_participants
            } else {
                Vec::new()
            };

            diagrams.push(SequenceTree {
                name: diagram.name().map(|s| s.to_string()),
                participants,
                root_interactions,
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
        source_location: to_source_location(
            node.source_location().file(),
            node.source_location().line(),
        ),
        branches_node,
    })
}

fn read_participant(
    participant: fb_sequence::SequenceParticipant<'_>,
    participant_path: &str,
) -> Result<SequenceParticipant, String> {
    Ok(SequenceParticipant {
        display_name: participant.display_name().to_string(),
        alias: participant.alias().map(|s| s.to_string()),
        participant_type: map_participant_type(participant.participant_type())
            .map_err(|err| format!("{participant_path}: {err}"))?,
        source_location: to_source_location(
            participant.source_location().file(),
            participant.source_location().line(),
        ),
        stereotype: participant.stereotype().map(|s| s.to_string()),
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
