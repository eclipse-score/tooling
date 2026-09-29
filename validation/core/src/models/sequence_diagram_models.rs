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

//! Models for sequence-diagram FlatBuffer inputs used by design verification.

use std::collections::BTreeMap;

use sequence_logic::{Block, Interaction, Node, SequenceTree, SourceLocation};

use crate::validators::shared::{display_name_from_source_path, normalize};
use crate::{ErrorBuilder, ErrorCategory, ValidationResult};

/// Collection of sequence diagrams loaded from one or more FlatBuffer files.
pub struct SequenceDiagramInputs {
    pub diagrams: Vec<SequenceTree>,
}

const EXTERNAL_ENDPOINT_NAME: &str = "ExternalEndpoint";

pub fn is_external_endpoint(participant: &str) -> bool {
    participant == EXTERNAL_ENDPOINT_NAME
}

/// One function-call interaction observed in a sequence diagram.
pub struct ObservedSequenceCall {
    pub caller: String,
    pub callee: String,
    pub method: String,
    pub source_location: SourceLocation,
}

/// Validation-only participant metadata keyed by the participant reference name
/// used in sequence interactions.
pub struct SequenceParticipantInfo {
    pub reference_name: String,
    pub source_location: SourceLocation,
}

impl SequenceParticipantInfo {
    // TODO: Remove this normalization once class diagram identifiers also use
    // `::` namespaces directly instead of `.`.
    pub fn normalize_qualified_name(reference: &str) -> String {
        normalize(reference)
    }
}

impl SequenceDiagramInputs {
    /// Build a [`SequenceDiagramIndex`] from sequence diagram inputs.
    pub fn to_sequence_diagram_index(&self, result: &mut ValidationResult) -> SequenceDiagramIndex {
        SequenceDiagramIndex::from_diagrams(&self.diagrams, result)
    }
}

/// Indexed sequence-diagram data prepared for validators.
pub struct SequenceDiagramIndex {
    participants: BTreeMap<String, SequenceParticipantInfo>,
    observed_calls: Vec<ObservedSequenceCall>,
}

impl SequenceDiagramIndex {
    fn from_diagrams(diagrams: &[SequenceTree], result: &mut ValidationResult) -> Self {
        let mut observed_calls = Vec::new();
        let mut participants: BTreeMap<String, SequenceParticipantInfo> = BTreeMap::new();

        for diagram in diagrams {
            for participant in &diagram.participants {
                let reference_name = participant
                    .alias
                    .as_deref()
                    .unwrap_or(&participant.display_name)
                    .to_string();

                let participant_id = normalize(&participant.uid);

                if participant_id.is_empty() {
                    result.add_failure(missing_participant_uid_error(participant, &reference_name));
                    continue;
                }

                if let Some(existing) = participants.get(&participant_id) {
                    if existing.reference_name != reference_name {
                        result.add_failure(conflicting_participant_uid_error(
                            &participant_id,
                            existing,
                            participant,
                            &reference_name,
                        ));
                    }
                } else if let Some((existing_uid, existing)) =
                    participants.iter().find(|(existing_uid, existing)| {
                        *existing_uid != &participant_id
                            && existing.reference_name == reference_name
                    })
                {
                    let (existing_source_file, _) = existing.source_location.display();
                    let (source_file, _) = participant.source_location.display();
                    result.add_failure(duplicate_participant_reference_error(
                        &reference_name,
                        existing_uid,
                        existing,
                        &participant_id,
                        participant,
                        &existing_source_file,
                        &source_file,
                    ));
                }

                // Keep the first declaration location when a participant is
                // declared in more than one input diagram.
                participants
                    .entry(participant_id)
                    .or_insert_with(|| SequenceParticipantInfo {
                        reference_name,
                        source_location: participant.source_location.clone(),
                    });
            }

            collect_block_data(&diagram.root, &mut observed_calls, result);
        }

        Self {
            participants,
            observed_calls,
        }
    }

    pub fn participants(&self) -> &BTreeMap<String, SequenceParticipantInfo> {
        &self.participants
    }

    pub fn declared_participants(&self) -> impl Iterator<Item = &str> {
        self.participants.keys().map(String::as_str)
    }
    pub fn observed_calls(&self) -> &[ObservedSequenceCall] {
        &self.observed_calls
    }
}

fn conflicting_participant_uid_error(
    participant_id: &str,
    existing: &SequenceParticipantInfo,
    participant: &sequence_logic::SequenceParticipant,
    reference_name: &str,
) -> String {
    let (existing_source_file, existing_source_line) = existing.source_location.display();
    let (source_file, source_line) = participant.source_location.display();

    ErrorBuilder::new(ErrorCategory::Design)
        .title(format!(
            "sequence participant uid \"{participant_id}\" is declared with conflicting reference names"
        ))
        .field("participant uid", format!("\"{participant_id}\""))
        .field(
            "first participant reference name",
            format!("\"{}\"", existing.reference_name),
        )
        .field(
            "duplicate participant reference name",
            format!("\"{reference_name}\""),
        )
        .field(
            "first sequence source file",
            format!("\"{existing_source_file}\""),
        )
        .field("first sequence source line", existing_source_line.to_string())
        .field("duplicate sequence source file", format!("\"{source_file}\""))
        .field("duplicate sequence source line", source_line.to_string())
        .fix("use one reference name consistently for the same sequence participant uid")
        .build()
}

fn duplicate_participant_reference_error(
    reference_name: &str,
    existing_uid: &str,
    existing: &SequenceParticipantInfo,
    participant_uid: &str,
    participant: &sequence_logic::SequenceParticipant,
    existing_source_file: &str,
    source_file: &str,
) -> String {
    let (_, existing_source_line) = existing.source_location.display();
    let (_, source_line) = participant.source_location.display();
    let existing_uid = display_name_from_source_path(existing_uid, existing_source_file, 2);
    let participant_uid = display_name_from_source_path(participant_uid, source_file, 2);

    ErrorBuilder::new(ErrorCategory::Design)
        .title(format!(
            "sequence participant reference name \"{reference_name}\" refers to multiple uids"
        ))
        .field(
            "participant reference name",
            format!("\"{reference_name}\""),
        )
        .field("first participant uid", format!("\"{existing_uid}\""))
        .field(
            "duplicate participant uid",
            format!("\"{participant_uid}\""),
        )
        .field(
            "first sequence source file",
            format!("\"{existing_source_file}\""),
        )
        .field(
            "first sequence source line",
            existing_source_line.to_string(),
        )
        .field(
            "duplicate sequence source file",
            format!("\"{source_file}\""),
        )
        .field("duplicate sequence source line", source_line.to_string())
        .fix("rename one participant alias or use a unique participant reference name")
        .build()
}

fn missing_participant_uid_error(
    participant: &sequence_logic::SequenceParticipant,
    reference_name: &str,
) -> String {
    let (source_file, source_line) = participant.source_location.display();

    ErrorBuilder::new(ErrorCategory::Design)
        .title(format!(
            "sequence participant \"{reference_name}\" is missing the uid required for validation"
        ))
        .field("participant reference name", format!("\"{reference_name}\""))
        .field(
            "participant display name",
            format!("\"{}\"", participant.display_name),
        )
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .fix(
            "set the participant uid to the canonical component or class identifier used by validation",
        )
        .build()
}

fn collect_block_data(
    block: &Block,
    observed_calls: &mut Vec<ObservedSequenceCall>,
    result: &mut ValidationResult,
) {
    for node in &block.items {
        collect_sequence_data(node, observed_calls, result);
    }
}

fn collect_sequence_data(
    node: &Node,
    observed_calls: &mut Vec<ObservedSequenceCall>,
    result: &mut ValidationResult,
) {
    match node {
        Node::Interaction(interaction) => {
            observed_calls.push(observe_interaction(interaction));
        }
        Node::Branch(branch) => {
            for case in &branch.cases {
                collect_block_data(&case.block, observed_calls, result);
            }
        }
        Node::Loop(loop_node) => {
            collect_block_data(&loop_node.block, observed_calls, result);
        }
        Node::Parallel(parallel) => {
            for branch in &parallel.branches {
                collect_block_data(&branch.block, observed_calls, result);
            }
        }
        Node::EarlyExit(early_exit) => {
            collect_block_data(&early_exit.block, observed_calls, result);
        }
        Node::Lifecycle(_) | Node::Reference(_) => {}
    }
}

fn observe_interaction(interaction: &Interaction) -> ObservedSequenceCall {
    let observed_call = ObservedSequenceCall {
        caller: interaction
            .sender
            .as_deref()
            .unwrap_or(EXTERNAL_ENDPOINT_NAME)
            .to_string(),
        callee: interaction
            .receiver
            .as_deref()
            .unwrap_or(EXTERNAL_ENDPOINT_NAME)
            .to_string(),
        method: interaction.message.clone().unwrap_or_default(),
        source_location: interaction.source_location.clone(),
    };

    observed_call
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::fixtures::dummy_source_location;
    use sequence_logic::{Branch, BranchCase, Interaction, ParticipantType, SequenceParticipant};

    fn interaction(caller: Option<&str>, callee: Option<&str>, method: &str) -> Node {
        Node::Interaction(Interaction {
            sender: caller.map(|caller| caller.to_string().into()),
            receiver: callee.map(|callee| callee.to_string().into()),
            message: Some(method.to_string()),
            source_location: dummy_source_location(),
        })
    }

    fn branch(items: Vec<Node>) -> Node {
        Node::Branch(Branch {
            cases: vec![BranchCase {
                condition: Some("case".to_string()),
                block: Block { items },
                source_location: dummy_source_location(),
            }],
        })
    }

    #[test]
    fn sequence_index_collects_nested_calls_recursively() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: Vec::new(),
                root: Block {
                    items: vec![
                        interaction(Some("unit_1"), Some("unit_2"), "GetData()"),
                        branch(vec![interaction(
                            Some("unit_2"),
                            Some("unit_3"),
                            "Forward()",
                        )]),
                    ],
                },
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert!(result.is_empty());
        assert_eq!(index.observed_calls().len(), 2);
        assert_eq!(index.observed_calls()[0].caller, "unit_1");
        assert_eq!(index.observed_calls()[0].callee, "unit_2");
        assert_eq!(index.observed_calls()[0].method, "GetData()");
        assert_eq!(index.observed_calls()[1].caller, "unit_2");
        assert_eq!(index.observed_calls()[1].callee, "unit_3");
        assert_eq!(index.observed_calls()[1].method, "Forward()");
    }

    #[test]
    fn sequence_index_maps_missing_caller_to_external_endpoint() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: Vec::new(),
                root: Block {
                    items: vec![interaction(None, Some("unit_2"), "GetData()")],
                },
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert!(result.is_empty());
        assert_eq!(index.observed_calls()[0].caller, EXTERNAL_ENDPOINT_NAME);
        assert_eq!(index.observed_calls()[0].callee, "unit_2");
    }

    #[test]
    fn sequence_index_maps_missing_callee_to_external_endpoint() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: Vec::new(),
                root: Block {
                    items: vec![interaction(Some("unit_1"), None, "GetData()")],
                },
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert!(result.is_empty());
        assert_eq!(index.observed_calls()[0].caller, "unit_1");
        assert_eq!(index.observed_calls()[0].callee, EXTERNAL_ENDPOINT_NAME);
    }

    #[test]
    fn sequence_index_reports_missing_participant_uid_without_fallback_indexing() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: vec![SequenceParticipant {
                    display_name: ":Process/nara::com user".to_string(),
                    alias: Some("help".to_string()),
                    uid: String::new(),
                    participant_type: ParticipantType::Participant,
                    source_location: dummy_source_location(),
                    stereotype: None,
                }],
                root: Block { items: Vec::new() },
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert_eq!(index.participants().len(), 0);
        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("missing the uid required for validation"));
        assert!(result.failures[0].contains("\"help\""));
    }
}
