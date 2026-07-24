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

use std::collections::{BTreeMap, BTreeSet};

use sequence_logic::{Event, SequenceNode, SequenceParticipant, SequenceTree};
use source_location::SourceLocation;

use crate::{ErrorBuilder, ErrorCategory, ValidationResult};

/// Collection of sequence diagrams loaded from one or more FlatBuffer files.
pub struct SequenceDiagramInputs {
    pub diagrams: Vec<SequenceTree>,
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
    used_participants: BTreeSet<String>,
    observed_calls: Vec<ObservedSequenceCall>,
    participant_sources: BTreeMap<String, SourceLocation>,
}

impl SequenceDiagramIndex {
    fn from_diagrams(diagrams: &[SequenceTree], result: &mut ValidationResult) -> Self {
        let mut used_participants = BTreeSet::new();
        let mut observed_calls = Vec::new();
        let mut participant_sources = BTreeMap::new();

        for diagram in diagrams {
            collect_participant_sources(&diagram.participants, &mut participant_sources);
            for node in &diagram.root_interactions {
                collect_sequence_data(
                    node,
                    &mut used_participants,
                    &mut observed_calls,
                    &mut participant_sources,
                    result,
                );
            }
        }

        Self {
            used_participants,
            observed_calls,
            participant_sources,
        }
    }

    pub fn used_participants(&self) -> &BTreeSet<String> {
        &self.used_participants
    }

    pub fn observed_calls(&self) -> &[ObservedSequenceCall] {
        &self.observed_calls
    }

    pub fn participant_source(&self, participant: &str) -> Option<&SourceLocation> {
        self.participant_sources.get(participant)
    }
}

fn collect_participant_sources(
    participants: &[SequenceParticipant],
    participant_sources: &mut BTreeMap<String, SourceLocation>,
) {
    for participant in participants {
        participant_sources
            .entry(participant_name(participant))
            .or_insert_with(|| participant.source_location.clone());
    }
}

fn collect_sequence_data(
    node: &SequenceNode,
    used_participants: &mut BTreeSet<String>,
    observed_calls: &mut Vec<ObservedSequenceCall>,
    participant_sources: &mut BTreeMap<String, SourceLocation>,
    result: &mut ValidationResult,
) {
    match &node.event {
        Event::Interaction(interaction) => {
            let (source_file, source_line) = node.source_location.display();
            validate_required_endpoints(
                result,
                RequiredEndpointsCheck {
                    item_kind: "sequence function",
                    caller: interaction.caller.as_str(),
                    callee: interaction.callee.as_str(),
                    label_value: interaction.method.as_str(),
                    label_name: "method",
                    source_file: source_file.as_str(),
                    source_line,
                },
            );

            record_participant_usage_and_source(
                interaction.caller.as_str(),
                &node.source_location,
                used_participants,
                participant_sources,
            );
            record_participant_usage_and_source(
                interaction.callee.as_str(),
                &node.source_location,
                used_participants,
                participant_sources,
            );

            observed_calls.push(ObservedSequenceCall {
                caller: interaction.caller.clone(),
                callee: interaction.callee.clone(),
                method: interaction.method.clone(),
                source_location: node.source_location.clone(),
            });
        }
        Event::Return(ret) => {
            let (source_file, source_line) = node.source_location.display();
            validate_required_endpoints(
                result,
                RequiredEndpointsCheck {
                    item_kind: "sequence return",
                    caller: ret.caller.as_str(),
                    callee: ret.callee.as_str(),
                    label_value: ret.return_content.as_str(),
                    label_name: "return content",
                    source_file: source_file.as_str(),
                    source_line,
                },
            );

            record_participant_usage_and_source(
                ret.caller.as_str(),
                &node.source_location,
                used_participants,
                participant_sources,
            );
            record_participant_usage_and_source(
                ret.callee.as_str(),
                &node.source_location,
                used_participants,
                participant_sources,
            );
        }
        Event::Condition(_) => {}
    }

    for child in &node.branches_node {
        collect_sequence_data(
            child,
            used_participants,
            observed_calls,
            participant_sources,
            result,
        );
    }
}

fn record_participant_usage_and_source(
    participant: &str,
    source_location: &SourceLocation,
    used_participants: &mut BTreeSet<String>,
    participant_sources: &mut BTreeMap<String, SourceLocation>,
) {
    if participant.is_empty() {
        return;
    }

    used_participants.insert(participant.to_string());
    participant_sources
        .entry(participant.to_string())
        .or_insert_with(|| source_location.clone());
}

fn participant_name(participant: &SequenceParticipant) -> String {
    participant
        .alias
        .clone()
        .filter(|alias| !alias.is_empty())
        .unwrap_or_else(|| participant.display_name.clone())
}

struct RequiredEndpointsCheck<'a> {
    item_kind: &'a str,
    caller: &'a str,
    callee: &'a str,
    label_value: &'a str,
    label_name: &'a str,
    source_file: &'a str,
    source_line: u32,
}

fn validate_required_endpoints(result: &mut ValidationResult, check: RequiredEndpointsCheck<'_>) {
    let RequiredEndpointsCheck {
        item_kind,
        caller,
        callee,
        label_value,
        label_name,
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

    let fix = format!(
        "add the missing {missing_endpoints} for {item_kind} \"{label_value}\" in the sequence diagram"
    );

    result.add_failure(
        ErrorBuilder::new(ErrorCategory::Method)
            .title(format!(
                "{item_kind} \"{label_value}\" is missing {missing_endpoints}."
            ))
            .field(label_name, format!("\"{label_value}\""))
            .field("caller unit", format!("\"{caller}\""))
            .field("callee unit", format!("\"{callee}\""))
            .field("sequence source file", format!("\"{source_file}\""))
            .field("sequence source line", source_line.to_string())
            .fix(fix)
            .build(),
    );
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
                participants: Vec::new(),
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
                participants: Vec::new(),
                root_interactions: vec![interaction("", "unit_2", "GetData()", Vec::new())],
            }],
        };

        let mut result = ValidationResult::default();
        let _index = inputs.to_sequence_diagram_index(&mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0]
            .contains("[Method] Sequence function \"GetData()\" is missing caller."));
        assert!(result.failures[0].contains("\"unit_2\""));
    }

    #[test]
    fn sequence_index_reports_interaction_with_missing_callee() {
        let inputs = SequenceDiagramInputs {
            diagrams: vec![SequenceTree {
                name: Some("seq".to_string()),
                participants: Vec::new(),
                root_interactions: vec![interaction("unit_1", "", "GetData()", Vec::new())],
            }],
        };

        let mut result = ValidationResult::default();
        let _index = inputs.to_sequence_diagram_index(&mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0]
            .contains("[Method] Sequence function \"GetData()\" is missing callee."));
        assert!(result.failures[0].contains("\"unit_1\""));
    }
}
