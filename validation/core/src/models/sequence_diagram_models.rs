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

use crate::ValidationResult;

/// Collection of sequence diagrams loaded from one or more FlatBuffer files.
pub struct SequenceDiagramInputs {
    pub diagrams: Vec<SequenceTree>,
}

/// One function-call interaction observed in a sequence diagram.
pub struct ObservedSequenceCall {
    pub caller: String,
    pub callee: String,
    pub method: String,
    pub source_file: String,
    pub source_line: u32,
}

impl SequenceDiagramInputs {
    /// Build a [`SequenceDiagramIndex`] from sequence diagram inputs.
    pub fn to_sequence_diagram_index(&self, result: &mut ValidationResult) -> SequenceDiagramIndex {
        SequenceDiagramIndex::from_diagrams(&self.diagrams, result)
    }
}

/// Indexed sequence-diagram data prepared for validators.
pub struct SequenceDiagramIndex {
    used_participants: BTreeSet<String>,
    observed_calls: Vec<ObservedSequenceCall>,
}

impl SequenceDiagramIndex {
    fn from_diagrams(diagrams: &[SequenceTree], result: &mut ValidationResult) -> Self {
        let mut used_participants = BTreeSet::new();
        let mut observed_calls = Vec::new();

        for diagram in diagrams {
            for node in &diagram.root_interactions {
                collect_sequence_data(node, &mut used_participants, &mut observed_calls, result);
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
    result: &mut ValidationResult,
) {
    match &node.event {
        Event::Interaction(interaction) => {
            validate_required_endpoints(
                result,
                RequiredEndpointsCheck {
                    connection_kind: "sequence function-call connection",
                    caller: interaction.caller.as_str(),
                    callee: interaction.callee.as_str(),
                    label_value: interaction.method.as_str(),
                    label_name: "Sequence function",
                    action:
                        "Provide both caller and callee for each sequence function-call connection",
                    source_file: node.source_location.file.as_ref(),
                    source_line: node.source_location.line,
                },
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
                source_file: node.source_location.file.to_string(),
                source_line: node.source_location.line,
            });
        }
        Event::Return(ret) => {
            validate_required_endpoints(
                result,
                RequiredEndpointsCheck {
                    connection_kind: "sequence return connection",
                    caller: ret.caller.as_str(),
                    callee: ret.callee.as_str(),
                    label_value: ret.return_content.as_str(),
                    label_name: "Return content",
                    action: "Provide both caller and callee for each sequence return connection",
                    source_file: node.source_location.file.as_ref(),
                    source_line: node.source_location.line,
                },
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
        collect_sequence_data(child, used_participants, observed_calls, result);
    }
}

struct RequiredEndpointsCheck<'a> {
    connection_kind: &'a str,
    caller: &'a str,
    callee: &'a str,
    label_value: &'a str,
    label_name: &'a str,
    action: &'a str,
    source_file: &'a str,
    source_line: u32,
}

fn validate_required_endpoints(result: &mut ValidationResult, check: RequiredEndpointsCheck<'_>) {
    let RequiredEndpointsCheck {
        connection_kind,
        caller,
        callee,
        label_value,
        label_name,
        action,
        source_file,
        source_line,
    } = check;

    if !caller.is_empty() && !callee.is_empty() {
        return;
    }

    let missing_endpoints = match (caller.is_empty(), callee.is_empty()) {
        (true, true) => "caller and callee",
        (true, false) => "caller",
        (false, true) => "callee",
        (false, false) => unreachable!(),
    };

    result.add_failure(format!(
        "Sequence validity failure: {connection_kind} is missing required endpoints:\n\
           Missing endpoints  : \"{missing_endpoints}\"\n\
           Caller unit        : \"{caller}\"\n\
           Callee unit        : \"{callee}\"\n\
           {label_name:<18}: \"{label_value}\"\n\
           Source file        : \"{source_file}\"\n\
           Source line        : \"{source_line}\"\n\
           Action             : {action}",
        source_file = source_file,
        source_line = source_line,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validators::fixtures::dummy_source_location;
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
            source_location: dummy_source_location(),
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
            source_location: dummy_source_location(),
            branches_node: Vec::new(),
        }
    }

    #[test]
    fn sequence_index_collects_calls_and_used_participants_recursively() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
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
            }],
        };

        let mut result = ValidationResult::default();
        let index = inputs.to_sequence_diagram_index(&mut result);

        assert!(result.is_empty());
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
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                root_interactions: vec![interaction("", "unit_2", "GetData()", Vec::new())],
            }],
        };

        let mut result = ValidationResult::default();
        let _index = inputs.to_sequence_diagram_index(&mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0]
            .contains("sequence function-call connection is missing required endpoints"));
        assert!(result.failures[0].contains("\"caller\""));
        assert!(result.failures[0].contains("\"unit_2\""));
    }

    #[test]
    fn sequence_index_reports_interaction_with_missing_callee() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                root_interactions: vec![interaction("unit_1", "", "GetData()", Vec::new())],
            }],
        };

        let mut result = ValidationResult::default();
        let _index = inputs.to_sequence_diagram_index(&mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0]
            .contains("sequence function-call connection is missing required endpoints"));
        assert!(result.failures[0].contains("\"callee\""));
        assert!(result.failures[0].contains("\"unit_1\""));
    }
}
