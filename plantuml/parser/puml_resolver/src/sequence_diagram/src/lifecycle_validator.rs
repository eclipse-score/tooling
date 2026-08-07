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

use std::collections::HashSet;

use crate::error::SequenceResolverError;
use sequence_logic::{Block, LifecycleAction, Node, ParticipantId, SourceLocation};

pub(crate) fn validate_lifecycle_consistency(root: &Block) -> Result<(), SequenceResolverError> {
    let mut destroyed = HashSet::new();
    validate_block(root, &mut destroyed)
}

fn validate_block(
    block: &Block,
    destroyed: &mut HashSet<ParticipantId>,
) -> Result<(), SequenceResolverError> {
    for node in &block.items {
        match node {
            Node::Interaction(interaction) => {
                if let Some(sender) = &interaction.sender {
                    ensure_not_destroyed(sender, &interaction.source_location, destroyed)?;
                }
                if let Some(receiver) = &interaction.receiver {
                    ensure_not_destroyed(receiver, &interaction.source_location, destroyed)?;
                }
            }
            Node::Reference(reference) => {
                for participant in &reference.participants {
                    ensure_not_destroyed(participant, &reference.source_location, destroyed)?;
                }
            }
            Node::Lifecycle(lifecycle) => match lifecycle.action {
                LifecycleAction::Create => {
                    destroyed.remove(&lifecycle.participant);
                }
                LifecycleAction::Destroy => {
                    destroyed.insert(lifecycle.participant.clone());
                }
                LifecycleAction::Activate | LifecycleAction::Deactivate => {
                    ensure_not_destroyed(
                        &lifecycle.participant,
                        &lifecycle.source_location,
                        destroyed,
                    )?;
                }
            },
            Node::Branch(branch) => {
                for case in &branch.cases {
                    validate_child_block(&case.block, destroyed)?;
                }
            }
            Node::Loop(loop_node) => validate_child_block(&loop_node.block, destroyed)?,
            Node::Parallel(parallel) => {
                for branch in &parallel.branches {
                    validate_child_block(&branch.block, destroyed)?;
                }
            }
            Node::EarlyExit(early_exit) => validate_child_block(&early_exit.block, destroyed)?,
        }
    }

    Ok(())
}

fn validate_child_block(
    block: &Block,
    destroyed: &HashSet<ParticipantId>,
) -> Result<(), SequenceResolverError> {
    let mut scoped_destroyed = destroyed.clone();
    validate_block(block, &mut scoped_destroyed)
}

fn ensure_not_destroyed(
    participant: &str,
    source_location: &SourceLocation,
    destroyed: &HashSet<ParticipantId>,
) -> Result<(), SequenceResolverError> {
    if destroyed.contains(participant) {
        return Err(SequenceResolverError::DestroyedParticipantUse {
            participant: participant.to_string(),
            source_location: source_location.clone(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod lifecycle_validator_tests {
    use super::*;
    use sequence_logic::{Block, Interaction, ParticipantLifecycle};

    fn dummy_source_location() -> SourceLocation {
        SourceLocation::new("test.puml", 0)
    }

    fn interaction(sender: &str, receiver: &str) -> Node {
        Node::Interaction(Interaction {
            sender: Some(sender.to_string().into()),
            receiver: Some(receiver.to_string().into()),
            message: Some("message".to_string()),
            source_location: dummy_source_location(),
        })
    }

    fn lifecycle(participant: &str, action: LifecycleAction) -> Node {
        Node::Lifecycle(ParticipantLifecycle {
            participant: participant.to_string().into(),
            action,
            source_location: dummy_source_location(),
        })
    }

    #[test]
    fn test_destroyed_participant_cannot_be_used_later() {
        let block = Block {
            items: vec![
                interaction("A", "B"),
                lifecycle("B", LifecycleAction::Destroy),
                interaction("A", "B"),
            ],
        };

        let err = validate_lifecycle_consistency(&block)
            .expect_err("destroyed participant use must fail");
        assert_eq!(
            err,
            SequenceResolverError::DestroyedParticipantUse {
                participant: "B".to_string(),
                source_location: dummy_source_location(),
            }
        );
    }

    #[test]
    fn test_create_restores_destroyed_participant() {
        let block = Block {
            items: vec![
                interaction("A", "B"),
                lifecycle("B", LifecycleAction::Destroy),
                lifecycle("B", LifecycleAction::Create),
                interaction("A", "B"),
            ],
        };

        validate_lifecycle_consistency(&block).expect("create makes participant usable again");
    }
}
