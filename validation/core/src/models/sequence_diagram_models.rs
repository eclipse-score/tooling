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

use std::collections::BTreeSet;

use sequence_logic::{Event, SequenceNode, SequenceTree};

use super::Errors;

/// One parsed sequence diagram from a FlatBuffer file.
pub struct SequenceDiagramInput {
    pub tree: SequenceTree,
    #[allow(dead_code)]
    pub source_files: Vec<String>,
    #[allow(dead_code)]
    pub version: Option<String>,
}

/// Collection of sequence diagrams loaded from one or more FlatBuffer files.
pub struct SequenceDiagramInputs {
    pub diagrams: Vec<SequenceDiagramInput>,
}

/// One function-call interaction observed in a sequence diagram.
pub struct ObservedSequenceCall {
    pub caller: String,
    pub callee: String,
    pub method: String,
}

impl SequenceDiagramInputs {
    /// Build a [`SequenceDiagramIndex`] from sequence diagram inputs.
    pub fn to_sequence_diagram_index(&self, errors: &mut Errors) -> SequenceDiagramIndex {
        SequenceDiagramIndex::from_diagrams(&self.diagrams, errors)
    }
}

/// Indexed sequence-diagram data prepared for validators.
pub struct SequenceDiagramIndex {
    used_participants: BTreeSet<String>,
    observed_calls: Vec<ObservedSequenceCall>,
}

impl SequenceDiagramIndex {
    fn from_diagrams(diagrams: &[SequenceDiagramInput], errors: &mut Errors) -> Self {
        let mut used_participants = BTreeSet::new();
        let mut observed_calls = Vec::new();

        for diagram in diagrams {
            for node in &diagram.tree.root_interactions {
                collect_sequence_data(node, &mut used_participants, &mut observed_calls, errors);
            }
        }

        Self {
            used_participants,
            observed_calls,
        }
    }

    pub fn used_participants(&self) -> &BTreeSet<String> {
        &self.used_participants
    }

    pub fn observed_calls(&self) -> &[ObservedSequenceCall] {
        &self.observed_calls
    }
}

fn collect_sequence_data(
    node: &SequenceNode,
    used_participants: &mut BTreeSet<String>,
    observed_calls: &mut Vec<ObservedSequenceCall>,
    errors: &mut Errors,
) {
    match &node.event {
        Event::Interaction(interaction) => {
            validate_required_endpoints(
                errors,
                "sequence function-call connection",
                interaction.caller.as_str(),
                interaction.callee.as_str(),
                interaction.method.as_str(),
                "Sequence function",
                "Provide both caller and callee for each sequence function-call connection",
            );

            if !interaction.caller.is_empty() {
                used_participants.insert(interaction.caller.clone());
            }
            if !interaction.callee.is_empty() {
                used_participants.insert(interaction.callee.clone());
            }

            observed_calls.push(ObservedSequenceCall {
                caller: interaction.caller.clone(),
                callee: interaction.callee.clone(),
                method: interaction.method.clone(),
            });
        }
        Event::Return(ret) => {
            validate_required_endpoints(
                errors,
                "sequence return connection",
                ret.caller.as_str(),
                ret.callee.as_str(),
                ret.return_content.as_str(),
                "Return content",
                "Provide both caller and callee for each sequence return connection",
            );

            if !ret.caller.is_empty() {
                used_participants.insert(ret.caller.clone());
            }
            if !ret.callee.is_empty() {
                used_participants.insert(ret.callee.clone());
            }
        }
        Event::Condition(_) => {}
    }

    for child in &node.branches_node {
        collect_sequence_data(child, used_participants, observed_calls, errors);
    }
}

fn validate_required_endpoints(
    errors: &mut Errors,
    connection_kind: &str,
    caller: &str,
    callee: &str,
    label_value: &str,
    label_name: &str,
    action: &str,
) {
    if !caller.is_empty() && !callee.is_empty() {
        return;
    }

    let missing_endpoints = match (caller.is_empty(), callee.is_empty()) {
        (true, true) => "caller and callee",
        (true, false) => "caller",
        (false, true) => "callee",
        (false, false) => unreachable!(),
    };

    errors.push(format!(
        "Sequence validity violation: {connection_kind} is missing required endpoints:\n\
           Missing endpoints  : \"{missing_endpoints}\"\n\
           Caller unit        : \"{caller}\"\n\
           Callee unit        : \"{callee}\"\n\
           {label_name:<18}: \"{label_value}\"\n\
           Action             : {action}",
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequence_logic::{Interaction, Return};

    fn interaction(
        caller: &str,
        callee: &str,
        method: &str,
        branches_node: Vec<SequenceNode>,
    ) -> SequenceNode {
        SequenceNode {
            event: Event::Interaction(Interaction {
                caller: caller.to_string(),
                callee: callee.to_string(),
                method: method.to_string(),
            }),
            branches_node,
        }
    }

    fn ret(caller: &str, callee: &str) -> SequenceNode {
        SequenceNode {
            event: Event::Return(Return {
                caller: caller.to_string(),
                callee: callee.to_string(),
                return_content: String::new(),
            }),
            branches_node: Vec::new(),
        }
    }

    #[test]
    fn sequence_index_collects_calls_and_used_participants_recursively() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceDiagramInput {
                tree: SequenceTree {
                    name: Some("seq".to_string()),
                    root_interactions: vec![interaction(
                        "unit_1",
                        "unit_2",
                        "GetData()",
                        vec![
                            ret("unit_1", "unit_2"),
                            interaction("unit_2", "unit_3", "Forward()", Vec::new()),
                        ],
                    )],
                },
                source_files: Vec::new(),
                version: None,
            }],
        };

        let mut errors = Errors::default();
        let index = inputs.to_sequence_diagram_index(&mut errors);

        assert!(errors.is_empty());
        assert_eq!(
            index.used_participants(),
            &BTreeSet::from([
                "unit_1".to_string(),
                "unit_2".to_string(),
                "unit_3".to_string(),
            ])
        );
        assert_eq!(index.observed_calls().len(), 2);
        assert_eq!(index.observed_calls()[0].caller, "unit_1");
        assert_eq!(index.observed_calls()[0].callee, "unit_2");
        assert_eq!(index.observed_calls()[0].method, "GetData()");
        assert_eq!(index.observed_calls()[1].caller, "unit_2");
        assert_eq!(index.observed_calls()[1].callee, "unit_3");
        assert_eq!(index.observed_calls()[1].method, "Forward()");
    }

    #[test]
    fn sequence_index_reports_interaction_with_missing_required_endpoints() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceDiagramInput {
                tree: SequenceTree {
                    name: Some("seq".to_string()),
                    root_interactions: vec![interaction("", "unit_2", "GetData()", Vec::new())],
                },
                source_files: Vec::new(),
                version: None,
            }],
        };

        let mut errors = Errors::default();
        let _index = inputs.to_sequence_diagram_index(&mut errors);

        assert_eq!(errors.messages.len(), 1);
        assert!(errors.messages[0]
            .contains("sequence function-call connection is missing required endpoints"));
        assert!(errors.messages[0].contains("\"caller\""));
        assert!(errors.messages[0].contains("\"unit_2\""));
    }

    #[test]
    fn sequence_index_reports_interaction_with_missing_callee() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceDiagramInput {
                tree: SequenceTree {
                    name: Some("seq".to_string()),
                    root_interactions: vec![interaction("unit_1", "", "GetData()", Vec::new())],
                },
                source_files: Vec::new(),
                version: None,
            }],
        };

        let mut errors = Errors::default();
        let _index = inputs.to_sequence_diagram_index(&mut errors);

        assert_eq!(errors.messages.len(), 1);
        assert!(errors.messages[0]
            .contains("sequence function-call connection is missing required endpoints"));
        assert!(errors.messages[0].contains("\"callee\""));
        assert!(errors.messages[0].contains("\"unit_1\""));
    }
}
