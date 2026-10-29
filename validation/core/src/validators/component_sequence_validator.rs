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

//! Validation: compare component-diagram unit IDs and interface connections
//! with sequence-diagram participant uids and function-call connections.

use std::collections::{BTreeMap, BTreeSet};

use super::shared::{
    best_id_suggestion, build_container_units, build_observed_call_contexts, build_unit_bindings,
    format_name_list, intersect_interfaces, match_element, ContainerUnits, ElementMatch,
    SequenceCallContext, UnitBindings,
};
use crate::models::{
    ComponentDiagramArchitecture, Environment, SequenceDiagramIndex, SequenceParticipantInfo,
};
use crate::results::{ErrorBuilder, ErrorCategory};
use crate::{Diagnostics, ValidationResult};

/// Run component-vs-sequence naming validation.
pub fn validate_component_sequence(
    component_diagram: &ComponentDiagramArchitecture,
    sequence_diagram: &SequenceDiagramIndex,
) -> ValidationResult {
    ComponentSequenceValidator::new(component_diagram, sequence_diagram).run()
}

type ConnectedUnitPairs = BTreeMap<(String, String), BTreeSet<String>>;

struct ComponentSequenceValidator<'a> {
    participants: &'a BTreeMap<String, SequenceParticipantInfo>,
    observed_call_contexts: Vec<SequenceCallContext>,
    connected_unit_pairs: ConnectedUnitPairs,
    unit_bindings: UnitBindings,
    container_units: ContainerUnits,
    result: ValidationResult,
}

impl SequenceCallContext {
    fn normalized_left_unit(&self) -> &str {
        if self.caller_unit <= self.callee_unit {
            &self.caller_unit
        } else {
            &self.callee_unit
        }
    }

    fn normalized_right_unit(&self) -> &str {
        if self.caller_unit <= self.callee_unit {
            &self.callee_unit
        } else {
            &self.caller_unit
        }
    }

    fn left_interfaces(&self) -> &BTreeSet<String> {
        if self.normalized_left_unit() == self.caller_unit {
            &self.caller_interfaces
        } else {
            &self.callee_interfaces
        }
    }

    fn right_interfaces(&self) -> &BTreeSet<String> {
        if self.normalized_right_unit() == self.caller_unit {
            &self.caller_interfaces
        } else {
            &self.callee_interfaces
        }
    }
}

impl<'a> ComponentSequenceValidator<'a> {
    fn new(
        component_diagram: &ComponentDiagramArchitecture,
        sequence_diagram: &'a SequenceDiagramIndex,
    ) -> Self {
        let unit_bindings = build_unit_bindings(component_diagram);
        let container_units = build_container_units(component_diagram);
        let observed_call_contexts = build_observed_call_contexts(
            sequence_diagram.observed_calls(),
            &unit_bindings,
            &container_units,
        );

        Self {
            participants: sequence_diagram.participants(),
            observed_call_contexts,
            connected_unit_pairs: build_connected_unit_pairs(&unit_bindings),
            unit_bindings,
            container_units,
            result: ValidationResult::default(),
        }
    }

    fn run(mut self) -> ValidationResult {
        append_debug_log(
            &mut self.result.diagnostics,
            self.participants.keys(),
            &self.observed_call_contexts,
            &self.unit_bindings,
            &self.connected_unit_pairs,
        );
        self.check_consistency();
        self.result
    }

    fn check_consistency(&mut self) {
        self.check_participant_units();
        self.check_interface_connected_units_have_sequence_calls();
        self.check_sequence_calls_have_interface_connections();
    }

    fn check_participant_units(&mut self) {
        let mut linked_units = BTreeSet::new();
        let mut participant_errors = Vec::new();

        for (participant, info) in self.participants {
            let matched = match_element(&self.unit_bindings, &self.container_units, participant);

            if let Some(environment) = info.environment {
                // An actor or external system is environment, not an element:
                // naming a unit or component is a modelling error. The named
                // unit counts as linked to avoid a second, misleading error.
                let element_ids: Vec<&str> = match matched {
                    ElementMatch::Unit(id) | ElementMatch::Container(id) => vec![id],
                    ElementMatch::Ambiguous(ids) => ids,
                    ElementMatch::Unresolved => continue,
                };
                linked_units.extend(
                    element_ids
                        .iter()
                        .copied()
                        .filter(|id| self.unit_bindings.contains_key(*id)),
                );
                participant_errors.push(environment_names_element_error(
                    participant,
                    info,
                    environment,
                    &element_ids,
                    self.element_noun(&element_ids),
                ));
                continue;
            }

            match matched {
                ElementMatch::Unit(unit_id) => {
                    linked_units.insert(unit_id);
                }
                // A component or SEooC lifeline is a black box and does not
                // cover its units.
                ElementMatch::Container(_) => {}
                ElementMatch::Ambiguous(element_ids) => {
                    linked_units.extend(
                        element_ids
                            .iter()
                            .copied()
                            .filter(|id| self.unit_bindings.contains_key(*id)),
                    );
                    participant_errors.push(ambiguous_participant_error(
                        participant,
                        info,
                        &element_ids,
                        self.element_noun(&element_ids),
                    ));
                }
                ElementMatch::Unresolved => {
                    participant_errors.push(self.unknown_participant_error(participant, info));
                }
            }
        }

        let missing_unit_errors: Vec<String> = self
            .unit_bindings
            .keys()
            .filter(|unit_id| !linked_units.contains(unit_id.as_str()))
            .map(|unit_id| self.missing_participant_error(unit_id))
            .collect();

        for error in missing_unit_errors.into_iter().chain(participant_errors) {
            self.result.add_failure(error);
        }
    }

    /// "unit" or "component" when all ids are of that kind, else "element".
    fn element_noun(&self, element_ids: &[&str]) -> &'static str {
        let units = element_ids
            .iter()
            .filter(|id| self.unit_bindings.contains_key(**id))
            .count();

        if units == element_ids.len() {
            "unit"
        } else if units == 0 {
            "component"
        } else {
            "element"
        }
    }

    fn missing_participant_error(&self, unit_id: &str) -> String {
        let (source_file, source_line) = unit_source(&self.unit_bindings, unit_id);

        let error = ErrorBuilder::new(ErrorCategory::Naming)
            .title(format!(
                "unit id \"{unit_id}\" from the component diagram not found in the sequence diagram"
            ))
            .field("unit id", format!("\"{unit_id}\""))
            .field("component source file", format!("\"{source_file}\""))
            .field("component source line", source_line.to_string())
            .fix(format!(
                "add sequence participant \"{unit_id}\" in the sequence diagram, or remove it from the component diagram"
            ));

        if let Some(suggested_name) = best_id_suggestion(
            unit_id,
            self.participants
                .iter()
                .filter(|(_, info)| info.environment.is_none())
                .map(|(participant, _)| participant.as_str()),
        )
        .as_deref()
        {
            error.suggest(unit_id, None, suggested_name)
        } else {
            error
        }
        .build()
    }

    fn unknown_participant_error(
        &self,
        participant: &str,
        info: &SequenceParticipantInfo,
    ) -> String {
        let (source_file, source_line) = info.source_location.display();

        let error = ErrorBuilder::new(ErrorCategory::Naming)
            .title(format!(
                "participant \"{participant}\" from the sequence diagram not found in the component diagram"
            ))
            .field("participant", format!("\"{participant}\""))
            .field("sequence source file", format!("\"{source_file}\""))
            .field("sequence source line", source_line.to_string())
            .fix(format!(
                "add component unit \"{participant}\" in the component diagram, declare it as 'actor {participant}' or 'participant {participant} <<external>>' if it is outside the described architecture, or remove it from the sequence diagram"
            ));

        if let Some(suggested_name) = best_id_suggestion(
            participant,
            self.unit_bindings
                .keys()
                .chain(self.container_units.keys())
                .map(String::as_str),
        )
        .as_deref()
        {
            error.suggest(participant, None, suggested_name)
        } else {
            error
        }
        .build()
    }

    fn check_interface_connected_units_have_sequence_calls(&mut self) {
        for ((left_unit, right_unit), interfaces) in &self.connected_unit_pairs {
            if self.has_observed_call_between_units(left_unit, right_unit) {
                continue;
            }

            let unit_pair = format_unit_pair(left_unit, right_unit);
            let shared_interfaces = format_name_list(interfaces);
            let remove_connection_fix = if interfaces.len() == 1 {
                "remove that shared interface connection from the component diagram"
            } else {
                "remove those shared interface connections from the component diagram"
            };
            let (left_source_file, left_source_line) = unit_source(&self.unit_bindings, left_unit);
            let (right_source_file, right_source_line) =
                unit_source(&self.unit_bindings, right_unit);

            self.result.add_failure(
                ErrorBuilder::new(ErrorCategory::Interface)
                    .title(format!(
                        "component-connected units \"{left_unit}\" and \"{right_unit}\" have no corresponding function-call in the sequence diagram"
                    ))
                    .field("unit pair", unit_pair)
                    .field(
                        format!("component source file for \"{left_unit}\""),
                        format!("\"{left_source_file}\""),
                    )
                    .field(
                        format!("component source line for \"{left_unit}\""),
                        left_source_line.to_string(),
                    )
                    .field(
                        format!("component source file for \"{right_unit}\""),
                        format!("\"{right_source_file}\""),
                    )
                    .field(
                        format!("component source line for \"{right_unit}\""),
                        right_source_line.to_string(),
                    )
                    .field("shared interfaces", shared_interfaces.clone())
                    .fix(format!(
                        "add a function-call between \"{left_unit}\" and \"{right_unit}\" in the sequence diagram, or {remove_connection_fix}"
                    ))
                    .build(),
            );
        }
    }

    /// A call between elements exercises a unit pair when its caller and
    /// callee are exactly the two units. A component or SEooC lifeline is a
    /// black box and does not cover the pairs of its units.
    fn has_observed_call_between_units(&self, left_unit: &str, right_unit: &str) -> bool {
        self.observed_call_contexts.iter().any(|call_context| {
            if call_context.involves_environment() {
                return false;
            }

            let (caller, callee) = (&call_context.caller_unit, &call_context.callee_unit);
            (caller == left_unit && callee == right_unit)
                || (caller == right_unit && callee == left_unit)
        })
    }

    fn check_sequence_calls_have_interface_connections(&mut self) {
        let mut seen_pairs = BTreeSet::new();

        for call_context in &self.observed_call_contexts {
            if call_context.involves_environment() {
                continue;
            }

            if call_context.caller_unit == call_context.callee_unit || call_context.ends_overlap() {
                continue;
            }

            if !seen_pairs.insert((
                call_context.normalized_left_unit().to_string(),
                call_context.normalized_right_unit().to_string(),
            )) {
                continue;
            }

            let left_interfaces = call_context.left_interfaces();
            let right_interfaces = call_context.right_interfaces();

            if call_context.has_shared_interfaces() {
                continue;
            }

            let left_unit = call_context.normalized_left_unit();
            let right_unit = call_context.normalized_right_unit();
            let noun = if call_context.involves_container {
                "element"
            } else {
                "unit"
            };
            let unit_pair = format_unit_pair(left_unit, right_unit);
            let left_interface_label = format!("interfaces for \"{left_unit}\"");
            let right_interface_label = format!("interfaces for \"{right_unit}\"");
            let (source_file, source_line) = sequence_call_source(call_context);

            self.result.add_failure(
                ErrorBuilder::new(ErrorCategory::Interface)
                    .title(format!(
                        "sequence-connected {noun}s \"{left_unit}\" and \"{right_unit}\" have no corresponding shared interface connection in the component diagram."
                    ))
                    .field(format!("{noun} pair"), unit_pair)
                    .field("sequence source file", format!("\"{source_file}\""))
                    .field("sequence source line", source_line.to_string())
                    .field(&left_interface_label, format_name_list(left_interfaces))
                    .field(&right_interface_label, format_name_list(right_interfaces))
                    .fix(format!(
                        "add a shared interface connection between \"{left_unit}\" and \"{right_unit}\" in the component diagram, or remove that function-call from the sequence diagram"
                    ))
                    .build(),
            );
        }
    }
}

fn environment_names_element_error(
    participant: &str,
    info: &SequenceParticipantInfo,
    environment: Environment,
    element_ids: &[&str],
    noun: &str,
) -> String {
    let (source_file, source_line) = info.source_location.display();
    let matching_elements = element_ids
        .iter()
        .map(|element_id| format!("\"{element_id}\""))
        .collect::<Vec<_>>()
        .join(", ");

    ErrorBuilder::new(ErrorCategory::Naming)
        .title(format!(
            "participant \"{participant}\" is declared as {} but names a {noun} of the component diagram",
            environment.label()
        ))
        .field("participant", format!("\"{participant}\""))
        .field(format!("matching {noun}s"), matching_elements)
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .fix(format!(
            "declare participant \"{participant}\" as a regular participant if it is part of the described architecture, or rename it so that it does not match a {noun} id"
        ))
        .build()
}

fn ambiguous_participant_error(
    participant: &str,
    info: &SequenceParticipantInfo,
    element_ids: &[&str],
    noun: &str,
) -> String {
    let (source_file, source_line) = info.source_location.display();
    let matching_elements = element_ids
        .iter()
        .map(|element_id| format!("\"{element_id}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let example = element_ids.first().copied().unwrap_or(participant);

    ErrorBuilder::new(ErrorCategory::Naming)
        .title(format!(
            "participant \"{participant}\" from the sequence diagram matches multiple {noun}s in the component diagram"
        ))
        .field("participant", format!("\"{participant}\""))
        .field(format!("matching {noun}s"), matching_elements)
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .fix(format!(
            "write the qualified id of the intended {noun} as the label of participant \"{participant}\" in the sequence diagram, for example participant \"{example}\" as {participant}, or rename one of the matching {noun}s in the component diagram"
        ))
        .build()
}

fn append_debug_log<'a>(
    diagnostics: &mut Diagnostics,
    observed_participants: impl Iterator<Item = &'a String>,
    observed_call_contexts: &[SequenceCallContext],
    unit_bindings: &UnitBindings,
    connected_unit_pairs: &BTreeMap<(String, String), BTreeSet<String>>,
) {
    diagnostics.debug(|| "Expected unit ids from component diagrams:".to_string());
    for unit_id in unit_bindings.keys() {
        diagnostics.debug(|| format!("  {unit_id}"));
    }

    diagnostics.debug(|| "Observed participants from sequence diagrams:".to_string());
    for participant in observed_participants {
        diagnostics.debug(|| format!("  {participant}"));
    }

    diagnostics.debug(|| "Observed sequence calls from sequence diagrams:".to_string());
    for call_context in observed_call_contexts {
        diagnostics.debug(|| {
            format!(
                "  {} -> {} : {}",
                call_context.caller_unit, call_context.callee_unit, call_context.method
            )
        });
    }

    diagnostics.debug(|| "Unit interface targets from component diagrams:".to_string());
    for (unit_id, bindings) in unit_bindings {
        diagnostics.debug(|| {
            format!(
                "  {unit_id} -> {}",
                format_name_list(&bindings.all_interfaces)
            )
        });
    }

    diagnostics.debug(|| "Interface-connected unit pairs from component diagrams:".to_string());
    for ((left, right), interfaces) in connected_unit_pairs {
        diagnostics.debug(|| format!("  {left} <-> {right} via {}", format_name_list(interfaces)));
    }
}

fn build_connected_unit_pairs(
    unit_bindings: &UnitBindings,
) -> BTreeMap<(String, String), BTreeSet<String>> {
    let mut connected_unit_pairs = BTreeMap::new();
    let unit_ids: Vec<&String> = unit_bindings.keys().collect();

    for index in 0..unit_ids.len() {
        for other_index in (index + 1)..unit_ids.len() {
            let left_id = unit_ids[index];
            let right_id = unit_ids[other_index];
            let left_bindings = &unit_bindings[left_id];
            let right_bindings = &unit_bindings[right_id];
            let mut shared_interfaces = intersect_interfaces(
                &left_bindings.required_interfaces,
                &right_bindings.provided_interfaces,
            );
            shared_interfaces.extend(intersect_interfaces(
                &right_bindings.required_interfaces,
                &left_bindings.provided_interfaces,
            ));

            if shared_interfaces.is_empty() {
                continue;
            }

            connected_unit_pairs.insert((left_id.clone(), right_id.clone()), shared_interfaces);
        }
    }

    connected_unit_pairs
}

fn unit_source(unit_bindings: &UnitBindings, unit_id: &str) -> (String, u32) {
    unit_bindings
        .get(unit_id)
        .and_then(|bindings| bindings.source_location.as_ref())
        .map(|source_location| source_location.display())
        .unwrap_or_default()
}

fn sequence_call_source(call_context: &SequenceCallContext) -> (String, u32) {
    call_context.source_location.display()
}

fn format_unit_pair(left_unit: &str, right_unit: &str) -> String {
    format!("\"{left_unit}\" <-> \"{right_unit}\"")
}

#[cfg(test)]
#[path = "test/component_sequence_validator_test.rs"]
mod tests;
