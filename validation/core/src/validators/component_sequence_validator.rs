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
//! with sequence-diagram participants and function-call connections.

use std::collections::{BTreeMap, BTreeSet};

use super::shared::{
    best_string_suggestion, build_observed_call_contexts, build_unit_bindings,
    display_name_from_source_path_in_context, display_names_without_common_prefix,
    display_unit_pair_from_optional_source_paths, format_display_names, format_name_list,
    intersect_interfaces, SequenceCallContext, UnitBindings,
};
use crate::models::{
    is_external_endpoint, ComponentDiagramArchitecture, SequenceDiagramIndex,
    SequenceParticipantInfo,
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
    observed_call_contexts: Vec<SequenceCallContext<'a>>,
    connected_unit_pairs: ConnectedUnitPairs,
    unit_bindings: UnitBindings,
    result: ValidationResult,
}

impl SequenceCallContext<'_> {
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
        let observed_call_contexts = build_observed_call_contexts(
            sequence_diagram.observed_calls(),
            sequence_diagram.participants(),
            &unit_bindings,
        );

        Self {
            participants: sequence_diagram.participants(),
            observed_call_contexts,
            connected_unit_pairs: build_connected_unit_pairs(&unit_bindings),
            unit_bindings,
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
        self.check_participant_uids();
        self.check_interface_connected_units_have_sequence_calls();
        self.check_sequence_calls_have_interface_connections();
    }

    fn check_participant_uids(&mut self) {
        let declared_unit_ids: BTreeSet<String> = self
            .participants
            .iter()
            .filter(|(participant, _)| !is_external_endpoint(participant))
            .map(|(participant, _)| (*participant).clone())
            .filter(|unit_id| self.unit_bindings.contains_key(unit_id))
            .collect();

        for unit_id in self
            .unit_bindings
            .keys()
            .filter(|unit_id| !declared_unit_ids.contains(*unit_id))
        {
            let (source_file, source_line) = self
                .unit_bindings
                .get(unit_id)
                .and_then(|bindings| bindings.source_location.as_ref())
                .map(|source_location| source_location.display())
                .unwrap_or_default();
            let display_name = display_missing_unit_name(
                unit_id,
                &source_file,
                self.participants.keys().map(String::as_str),
            );

            let error = ErrorBuilder::new(ErrorCategory::Naming)
                .title(format!(
                    "unit id \"{}\" from the component diagram not found as a participant uid in the sequence diagram",
                    display_name
                ))
                .field("unit id", format!("\"{display_name}\""))
                .field("component source file", format!("\"{source_file}\""))
                .field("component source line", source_line.to_string())
                .fix(format!(
                    "add sequence participant uid \"{}\" in the sequence diagram, or remove it from the component diagram",
                    display_name
                ));

            let error = if let Some(suggested_name) = best_string_suggestion(
                unit_id,
                self.participants
                    .keys()
                    .filter(|participant| !is_external_endpoint(participant))
                    .map(String::as_str),
            )
            .as_deref()
            {
                error.suggest(
                    &display_name,
                    None,
                    &display_missing_unit_suggestion(suggested_name),
                )
            } else {
                error
            };

            self.result.add_failure(error.build());
        }

        for participant_uid in self.participants.keys().filter(|participant_uid| {
            !is_external_endpoint(participant_uid)
                && !self.unit_bindings.contains_key(*participant_uid)
        }) {
            let (source_file, source_line) =
                self.participants[participant_uid].source_location.display();
            let display_name = display_missing_unit_name(
                participant_uid,
                &source_file,
                self.unit_bindings.keys().map(String::as_str),
            );

            let error = ErrorBuilder::new(ErrorCategory::Naming)
                .title(format!(
                    "participant uid \"{}\" from the sequence diagram not found in the component diagram",
                    display_name
                ))
                .field("participant uid", format!("\"{display_name}\""))
                .field("sequence source file", format!("\"{source_file}\""))
                .field("sequence source line", source_line.to_string())
                .fix(format!(
                    "add component unit id \"{}\" in the component diagram, or remove it from the sequence diagram",
                    display_name
                ));

            let error = if let Some(suggested_name) = best_string_suggestion(
                participant_uid,
                self.unit_bindings.keys().map(String::as_str),
            )
            .as_deref()
            {
                error.suggest(
                    &display_name,
                    None,
                    &display_missing_unit_suggestion(suggested_name),
                )
            } else {
                error
            };

            self.result.add_failure(error.build());
        }
    }

    fn check_interface_connected_units_have_sequence_calls(&mut self) {
        for ((left_unit, right_unit), interfaces) in &self.connected_unit_pairs {
            if self.has_observed_call_between_units(left_unit, right_unit) {
                continue;
            }

            let unit_pair =
                format_component_connected_unit_pair(&self.unit_bindings, left_unit, right_unit);
            let shared_interfaces = format_trimmed_interface_list(interfaces);
            let remove_connection_fix = if interfaces.len() == 1 {
                "remove that shared interface connection from the component diagram"
            } else {
                "remove those shared interface connections from the component diagram"
            };
            let [left_display, right_display] =
                displayed_component_connected_unit_pair(&self.unit_bindings, left_unit, right_unit);
            let (left_source_file, left_source_line) = unit_source(&self.unit_bindings, left_unit);
            let (right_source_file, right_source_line) =
                unit_source(&self.unit_bindings, right_unit);

            self.result.add_failure(
                ErrorBuilder::new(ErrorCategory::Interface)
                    .title(format!(
                        "component-connected units \"{left_display}\" and \"{right_display}\" have no corresponding function-call in the sequence diagram"
                    ))
                    .field("unit pair", unit_pair)
                    .field(
                        format!("component source file for \"{left_display}\""),
                        format!("\"{left_source_file}\""),
                    )
                    .field(
                        format!("component source line for \"{left_display}\""),
                        left_source_line.to_string(),
                    )
                    .field(
                        format!("component source file for \"{right_display}\""),
                        format!("\"{right_source_file}\""),
                    )
                    .field(
                        format!("component source line for \"{right_display}\""),
                        right_source_line.to_string(),
                    )
                    .field("shared interfaces", shared_interfaces.clone())
                    .fix(format!(
                        "add a function-call between \"{left_display}\" and \"{right_display}\" in the sequence diagram, or {remove_connection_fix}"
                    ))
                    .build(),
            );
        }
    }

    fn has_observed_call_between_units(&self, left_unit: &str, right_unit: &str) -> bool {
        self.observed_call_contexts.iter().any(|call_context| {
            call_context.normalized_left_unit() == left_unit
                && call_context.normalized_right_unit() == right_unit
        })
    }

    fn check_sequence_calls_have_interface_connections(&mut self) {
        let mut seen_pairs = BTreeSet::new();

        for call_context in &self.observed_call_contexts {
            if call_involves_external_endpoint(call_context) {
                continue;
            }

            if call_context.caller_unit == call_context.callee_unit {
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
            let unit_pair =
                format_component_connected_unit_pair(&self.unit_bindings, left_unit, right_unit);
            let [left_display, right_display] =
                displayed_component_connected_unit_pair(&self.unit_bindings, left_unit, right_unit);
            let left_interface_label = format!("interfaces for \"{left_display}\"");
            let right_interface_label = format!("interfaces for \"{right_display}\"");
            let (source_file, source_line) = sequence_call_source(call_context);

            self.result.add_failure(
                ErrorBuilder::new(ErrorCategory::Interface)
                    .title(format!(
                        "sequence-connected units \"{left_display}\" and \"{right_display}\" have no corresponding shared interface connection in the component diagram."
                    ))
                    .field("unit pair", unit_pair)
                    .field("sequence source file", format!("\"{source_file}\""))
                    .field("sequence source line", source_line.to_string())
                    .field(
                        &left_interface_label,
                        format_trimmed_interface_list(left_interfaces),
                    )
                    .field(
                        &right_interface_label,
                        format_trimmed_interface_list(right_interfaces),
                    )
                    .fix(format!(
                        "add a shared interface connection between \"{left_display}\" and \"{right_display}\" in the component diagram, or remove that function-call from the sequence diagram."
                    ))
                    .build(),
            );
        }
    }
}

fn call_involves_external_endpoint(call_context: &SequenceCallContext<'_>) -> bool {
    is_external_endpoint(&call_context.caller_unit)
        || is_external_endpoint(&call_context.callee_unit)
}

fn append_debug_log<'a>(
    diagnostics: &mut Diagnostics,
    observed_participants: impl Iterator<Item = &'a String>,
    observed_call_contexts: &[SequenceCallContext<'_>],
    unit_bindings: &UnitBindings,
    connected_unit_pairs: &BTreeMap<(String, String), BTreeSet<String>>,
) {
    diagnostics.debug(|| "Expected unit ids from component diagrams:".to_string());
    for unit_id in unit_bindings.keys() {
        diagnostics.debug(|| format!("  {unit_id}"));
    }

    diagnostics.debug(|| "Observed participant uids from sequence diagrams:".to_string());
    for participant_uid in observed_participants {
        diagnostics.debug(|| format!("  {participant_uid}"));
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
            let left_unit_id = unit_ids[index];
            let right_unit_id = unit_ids[other_index];
            let left_bindings = &unit_bindings[left_unit_id];
            let right_bindings = &unit_bindings[right_unit_id];
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

            connected_unit_pairs.insert(
                (left_unit_id.clone(), right_unit_id.clone()),
                shared_interfaces,
            );
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

fn sequence_call_source(call_context: &SequenceCallContext<'_>) -> (String, u32) {
    call_context.source_location.display()
}

fn format_component_connected_unit_pair(
    unit_bindings: &UnitBindings,
    left_unit: &str,
    right_unit: &str,
) -> String {
    let [left_display, right_display] =
        displayed_component_connected_unit_pair(unit_bindings, left_unit, right_unit);

    format!("\"{left_display}\" <-> \"{right_display}\"")
}

fn displayed_component_connected_unit_pair(
    unit_bindings: &UnitBindings,
    left: &str,
    right: &str,
) -> [String; 2] {
    let (left_source_file, _) = unit_source(unit_bindings, left);
    let (right_source_file, _) = unit_source(unit_bindings, right);

    display_unit_pair_from_optional_source_paths(
        left,
        (!left_source_file.is_empty()).then_some(left_source_file.as_str()),
        right,
        (!right_source_file.is_empty()).then_some(right_source_file.as_str()),
    )
}

fn format_trimmed_interface_list(names: &BTreeSet<String>) -> String {
    format_display_names(names.iter().map(String::as_str), 2)
}

fn display_missing_unit_name<'a>(
    target: &'a str,
    source_file: &str,
    other_names: impl Iterator<Item = &'a str>,
) -> String {
    display_name_from_source_path_in_context(target, source_file, other_names)
}

fn display_missing_unit_suggestion(name: &str) -> String {
    display_names_without_common_prefix([name], 3)
        .into_iter()
        .next()
        .unwrap_or_else(|| name.to_string())
}

#[cfg(test)]
#[path = "test/component_sequence_validator_test.rs"]
mod tests;
