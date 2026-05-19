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

//! Validation: compare component-diagram unit IDs with sequence-diagram
//! used participant IDs.

use std::collections::BTreeSet;

use crate::models::{ComponentDiagramArchitecture, Errors, SequenceDiagramIndex};

/// Run component-vs-sequence naming validation.
pub fn validate_component_sequence(
    component_diagram: &ComponentDiagramArchitecture,
    sequence_diagram: &SequenceDiagramIndex,
    errors: Errors,
) -> Errors {
    ComponentSequenceValidator::new(
        build_expected_unit_aliases(component_diagram),
        sequence_diagram.used_participants(),
        errors,
    )
    .run()
}

struct ComponentSequenceValidator<'a> {
    expected_unit_aliases: BTreeSet<String>,
    observed_participants: &'a BTreeSet<String>,
    errors: Errors,
}

impl<'a> ComponentSequenceValidator<'a> {
    fn new(
        expected_unit_aliases: BTreeSet<String>,
        observed_participants: &'a BTreeSet<String>,
        errors: Errors,
    ) -> Self {
        Self {
            expected_unit_aliases,
            observed_participants,
            errors,
        }
    }

    fn run(mut self) -> Errors {
        self.errors.debug_output = self.build_debug_log();
        self.check_consistency();
        self.errors
    }

    fn build_debug_log(&self) -> String {
        let mut log = String::new();

        log.push_str("DEBUG: Expected unit aliases from component diagrams:\n");
        for alias in &self.expected_unit_aliases {
            log.push_str(&format!("  {alias}\n"));
        }

        log.push_str("DEBUG: Observed participants from sequence diagrams:\n");
        for participant in self.observed_participants {
            log.push_str(&format!("  {participant}\n"));
        }

        log
    }

    fn check_consistency(&mut self) {
        for alias in &self.expected_unit_aliases {
            if !self.observed_participants.contains(alias) {
                self.errors.push(format!(
                    "Naming consistency violation: component unit alias not found in sequence participants:\n\
                      Unit alias         : \"{alias}\"\n\
                      Source             : Component diagram unit aliases\n\
                      Action             : Add a matching sequence participant for this unit alias",
                ));
            }
        }

        for participant in self.observed_participants {
            if !self.expected_unit_aliases.contains(participant) {
                self.errors.push(format!(
                    "Naming consistency violation: sequence participant not found in component unit aliases:\n\
                      Participant        : \"{participant}\"\n\
                      Source             : Sequence diagram participants\n\
                      Action             : Add a matching component unit alias or remove this participant",
                ));
            }
        }
    }
}

fn build_expected_unit_aliases(
    component_diagram: &ComponentDiagramArchitecture,
) -> BTreeSet<String> {
    component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_unit())
        .filter_map(|entity| entity.alias.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ComponentDiagramInput, ComponentDiagramInputs, SequenceDiagramInput, SequenceDiagramInputs,
    };
    use sequence_logic::{Event, Interaction, SequenceNode, SequenceTree};

    fn component_diagrams(aliases: &[&str]) -> ComponentDiagramInputs {
        ComponentDiagramInputs {
            entities: aliases
                .iter()
                .map(|alias| ComponentDiagramInput {
                    id: format!("some_id.{alias}"),
                    alias: Some((*alias).to_string()),
                    parent_id: None,
                    stereotype: Some("unit".to_string()),
                })
                .collect(),
        }
    }

    fn sequence_diagrams(participants: &[&str]) -> SequenceDiagramInputs {
        SequenceDiagramInputs {
            diagrams: vec![SequenceDiagramInput {
                tree: SequenceTree {
                    name: Some("seq".to_string()),
                    root_interactions: participants
                        .iter()
                        .map(|participant| SequenceNode {
                            event: Event::Interaction(Interaction {
                                caller: (*participant).to_string(),
                                callee: (*participant).to_string(),
                                method: String::new(),
                            }),
                            branches_node: Vec::new(),
                        })
                        .collect(),
                },
                source_files: Vec::new(),
                version: None,
            }],
        }
    }

    #[test]
    fn passes_when_aliases_and_participants_are_identical() {
        let component_diagrams = component_diagrams(&["unit_1", "unit_2"]);
        let sequence_diagrams = sequence_diagrams(&["unit_1", "unit_2"]);

        let mut errors = Errors::default();
        let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
        let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

        let errors = validate_component_sequence(&component_arch, &sequence_index, errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn reports_missing_and_extra() {
        let component_diagrams = component_diagrams(&["unit_1", "unit_2", "unit_3"]);
        let sequence_diagrams = sequence_diagrams(&["unit_2", "unit_4"]);

        let mut errors = Errors::default();
        let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
        let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

        let errors = validate_component_sequence(&component_arch, &sequence_index, errors);

        assert!(!errors.is_empty());
        assert_eq!(errors.messages.len(), 3);

        let missing_count = errors
            .messages
            .iter()
            .filter(|msg| msg.contains("unit alias not found in sequence participants"))
            .count();
        let unexpected_count = errors
            .messages
            .iter()
            .filter(|msg| msg.contains("sequence participant not found in component unit aliases"))
            .count();

        assert_eq!(missing_count, 2);
        assert_eq!(unexpected_count, 1);
    }

    #[test]
    fn units_without_alias_are_ignored() {
        let component_diagrams = ComponentDiagramInputs {
            entities: vec![ComponentDiagramInput {
                id: "module_a.unit_1".to_string(),
                alias: None,
                parent_id: None,
                stereotype: Some("unit".to_string()),
            }],
        };
        let sequence_diagrams = sequence_diagrams(&[]);

        let mut errors = Errors::default();
        let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
        let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

        let errors = validate_component_sequence(&component_arch, &sequence_index, errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn reports_alias_missing_from_participants() {
        let component_diagrams = component_diagrams(&["u1", "u2"]);
        let sequence_diagrams = sequence_diagrams(&["u1"]);

        let mut errors = Errors::default();
        let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
        let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

        let errors = validate_component_sequence(&component_arch, &sequence_index, errors);
        assert_eq!(errors.messages.len(), 1);
        assert!(errors.messages[0].contains("\"u2\""));
    }

    #[test]
    fn reports_participant_not_in_aliases() {
        let component_diagrams = component_diagrams(&["u1"]);
        let sequence_diagrams = sequence_diagrams(&["u1", "orphan"]);

        let mut errors = Errors::default();
        let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
        let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

        let errors = validate_component_sequence(&component_arch, &sequence_index, errors);
        assert_eq!(errors.messages.len(), 1);
        assert!(errors.messages[0].contains("\"orphan\""));
    }
}
