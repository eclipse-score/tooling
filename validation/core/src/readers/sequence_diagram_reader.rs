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
    Block, Branch, BranchCase, EarlyExit, Interaction, LifecycleAction, Loop, Node, Parallel,
    ParallelBranch, ParticipantLifecycle, ParticipantType, Reference, SequenceParticipant,
    SequenceTree,
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

            // See the analogous check in class_diagram_reader.rs: skip a buffer that
            // doesn't match this schema instead of letting the verifier fail on
            // misaligned data.
            if !fb_sequence::sequence_diagram_buffer_has_identifier(&data) {
                log::warn!("{path}: not a sequence-diagram, skipping validation");
                continue;
            }

            let diagram = flatbuffers::root::<fb_sequence::SequenceDiagram>(&data)
                .map_err(|e| format!("Failed to parse sequence FlatBuffer {path}: {e}"))?;

            let root = read_block(diagram.root(), &format!("{path}:root"))
                .map_err(|e| format!("Failed to parse sequence root block: {e}"))?;

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
                root,
            });
        }

        Ok(SequenceDiagramInputs { diagrams })
    }
}

fn read_block(block: fb_sequence::Block<'_>, block_path: &str) -> Result<Block, String> {
    let mut items = Vec::new();
    if let Some(values) = block.items() {
        items.reserve(values.len());
        for (index, item) in values.iter().enumerate() {
            items.push(read_node_item(
                item,
                &format!("{block_path}.items[{index}]"),
            )?);
        }
    }

    Ok(Block { items })
}

fn read_node_item(item: fb_sequence::NodeItem<'_>, node_path: &str) -> Result<Node, String> {
    match item.node_type() {
        fb_sequence::Node::Interaction => {
            let interaction = item.node_as_interaction().ok_or_else(|| {
                format!("{node_path}: node_type is Interaction, but payload is missing")
            })?;
            Ok(Node::Interaction(Interaction {
                sender: interaction.sender().map(|sender| sender.to_string().into()),
                receiver: interaction
                    .receiver()
                    .map(|receiver| receiver.to_string().into()),
                message: interaction.message().map(|s| s.to_string()),
                source_location: to_source_location(
                    interaction.source_location().file(),
                    interaction.source_location().line(),
                ),
            }))
        }
        fb_sequence::Node::Branch => {
            let branch = item.node_as_branch().ok_or_else(|| {
                format!("{node_path}: node_type is Branch, but payload is missing")
            })?;
            Ok(Node::Branch(read_branch(branch, node_path)?))
        }
        fb_sequence::Node::Loop => {
            let loop_node = item
                .node_as_loop()
                .ok_or_else(|| format!("{node_path}: node_type is Loop, but payload is missing"))?;
            Ok(Node::Loop(Loop {
                condition: loop_node.condition().map(|s| s.to_string()),
                block: read_block(loop_node.block(), &format!("{node_path}.block"))?,
                source_location: to_source_location(
                    loop_node.source_location().file(),
                    loop_node.source_location().line(),
                ),
            }))
        }
        fb_sequence::Node::Parallel => {
            let parallel = item.node_as_parallel().ok_or_else(|| {
                format!("{node_path}: node_type is Parallel, but payload is missing")
            })?;
            Ok(Node::Parallel(read_parallel(parallel, node_path)?))
        }
        fb_sequence::Node::EarlyExit => {
            let early_exit = item.node_as_early_exit().ok_or_else(|| {
                format!("{node_path}: node_type is EarlyExit, but payload is missing")
            })?;
            Ok(Node::EarlyExit(EarlyExit {
                reason: early_exit.reason().map(|s| s.to_string()),
                block: read_block(early_exit.block(), &format!("{node_path}.block"))?,
                source_location: to_source_location(
                    early_exit.source_location().file(),
                    early_exit.source_location().line(),
                ),
            }))
        }
        fb_sequence::Node::ParticipantLifecycle => {
            let lifecycle = item.node_as_participant_lifecycle().ok_or_else(|| {
                format!("{node_path}: node_type is ParticipantLifecycle, but payload is missing")
            })?;
            Ok(Node::Lifecycle(ParticipantLifecycle {
                participant: lifecycle.participant().to_string().into(),
                action: map_lifecycle_action(lifecycle.action())
                    .map_err(|err| format!("{node_path}: {err}"))?,
                source_location: to_source_location(
                    lifecycle.source_location().file(),
                    lifecycle.source_location().line(),
                ),
            }))
        }
        fb_sequence::Node::Reference => {
            let reference = item.node_as_reference().ok_or_else(|| {
                format!("{node_path}: node_type is Reference, but payload is missing")
            })?;
            Ok(Node::Reference(read_reference(reference)))
        }
        fb_sequence::Node::NONE => Err(format!("{node_path}: node_type is NONE")),
        other => Err(format!("{node_path}: unsupported node_type {other:?}")),
    }
}

fn read_branch(branch: fb_sequence::Branch<'_>, branch_path: &str) -> Result<Branch, String> {
    let mut cases = Vec::new();
    if let Some(values) = branch.cases() {
        cases.reserve(values.len());
        for (index, case) in values.iter().enumerate() {
            cases.push(BranchCase {
                condition: case.condition().map(|s| s.to_string()),
                block: read_block(case.block(), &format!("{branch_path}.cases[{index}].block"))?,
                source_location: to_source_location(
                    case.source_location().file(),
                    case.source_location().line(),
                ),
            });
        }
    }

    Ok(Branch { cases })
}

fn read_parallel(
    parallel: fb_sequence::Parallel<'_>,
    parallel_path: &str,
) -> Result<Parallel, String> {
    let mut branches = Vec::new();
    if let Some(values) = parallel.branches() {
        branches.reserve(values.len());
        for (index, branch) in values.iter().enumerate() {
            branches.push(ParallelBranch {
                label: branch.label().map(|s| s.to_string()),
                block: read_block(
                    branch.block(),
                    &format!("{parallel_path}.branches[{index}].block"),
                )?,
                source_location: to_source_location(
                    branch.source_location().file(),
                    branch.source_location().line(),
                ),
            });
        }
    }

    Ok(Parallel { branches })
}

fn read_reference(reference: fb_sequence::Reference<'_>) -> Reference {
    let participants = reference
        .participants()
        .map(|values| {
            values
                .iter()
                .map(|value| value.to_string().into())
                .collect()
        })
        .unwrap_or_default();

    Reference {
        participants,
        text: reference.text().map(|s| s.to_string()),
        source_location: to_source_location(
            reference.source_location().file(),
            reference.source_location().line(),
        ),
    }
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

fn map_lifecycle_action(value: fb_sequence::LifecycleAction) -> Result<LifecycleAction, String> {
    match value {
        fb_sequence::LifecycleAction::Create => Ok(LifecycleAction::Create),
        fb_sequence::LifecycleAction::Activate => Ok(LifecycleAction::Activate),
        fb_sequence::LifecycleAction::Deactivate => Ok(LifecycleAction::Deactivate),
        fb_sequence::LifecycleAction::Destroy => Ok(LifecycleAction::Destroy),
        other => Err(format!("unsupported lifecycle action {other:?}")),
    }
}
