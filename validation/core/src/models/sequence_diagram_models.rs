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

use crate::ValidationResult;

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

impl SequenceDiagramInputs {
    /// Build a [`SequenceDiagramIndex`] from sequence diagram inputs.
    pub fn to_sequence_diagram_index(&self, result: &mut ValidationResult) -> SequenceDiagramIndex {
        SequenceDiagramIndex::from_diagrams(&self.diagrams, result)
    }
}

/// Indexed sequence-diagram data prepared for validators.
pub struct SequenceDiagramIndex {
    participants: BTreeMap<String, SourceLocation>,
    observed_calls: Vec<ObservedSequenceCall>,
}

impl SequenceDiagramIndex {
    fn from_diagrams(diagrams: &[SequenceTree], result: &mut ValidationResult) -> Self {
        let mut observed_calls = Vec::new();
        let mut participants = BTreeMap::new();

        for diagram in diagrams {
            for participant in &diagram.participants {
                let reference_name = participant
                    .alias
                    .as_deref()
                    .unwrap_or(&participant.display_name)
                    .to_string();

                // Keep the first declaration location when a participant is
                // declared in more than one input diagram.
                participants
                    .entry(reference_name)
                    .or_insert_with(|| participant.source_location.clone());
            }

            collect_block_data(&diagram.root, &mut observed_calls, result);
        }

        Self {
            participants,
            observed_calls,
        }
    }

    pub fn participants(&self) -> &BTreeMap<String, SourceLocation> {
        &self.participants
    }

    pub fn observed_calls(&self) -> &[ObservedSequenceCall] {
        &self.observed_calls
    }
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
    use sequence_logic::{Branch, BranchCase, Interaction};

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
}
