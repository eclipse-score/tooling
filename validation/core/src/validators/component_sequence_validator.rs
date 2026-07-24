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
    build_observed_call_contexts, build_unit_bindings, format_name_list, intersect_interfaces,
    SequenceCallContext, UnitBindings,
};
use crate::models::{ComponentDiagramArchitecture, SequenceDiagramIndex};
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

const EXTERNAL_ENDPOINT_NAME: &str = "ExternalEndpoint";

struct ComponentSequenceValidator<'a> {
    observed_participants: &'a BTreeSet<String>,
    observed_call_contexts: Vec<SequenceCallContext<'a>>,
    connected_unit_pairs: ConnectedUnitPairs,
    sequence_diagram: &'a SequenceDiagramIndex,
    unit_bindings: UnitBindings,
    result: ValidationResult,
}

impl SequenceCallContext<'_> {
    fn normalized_left_unit(&self) -> &str {
        if self.caller_unit <= self.callee_unit {
            self.caller_unit
        } else {
            self.callee_unit
        }
    }

    fn normalized_right_unit(&self) -> &str {
        if self.caller_unit <= self.callee_unit {
            self.callee_unit
        } else {
            self.caller_unit
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
        let observed_call_contexts =
            build_observed_call_contexts(sequence_diagram.observed_calls(), &unit_bindings);

        Self {
            observed_participants: sequence_diagram.used_participants(),
            observed_call_contexts,
            connected_unit_pairs: build_connected_unit_pairs(&unit_bindings),
            sequence_diagram,
            unit_bindings,
            result: ValidationResult::default(),
        }
    }

    fn run(mut self) -> ValidationResult {
        append_debug_log(
            &mut self.result.diagnostics,
            self.observed_participants,
            &self.observed_call_contexts,
            &self.unit_bindings,
            &self.connected_unit_pairs,
        );
        self.check_consistency();
        self.result
    }

    fn check_consistency(&mut self) {
        self.check_participant_aliases();
        self.check_interface_connected_units_have_sequence_calls();
        self.check_sequence_calls_have_interface_connections();
    }

    fn check_participant_aliases(&mut self) {
        for alias in self
            .unit_bindings
            .keys()
            .filter(|alias| !self.observed_participants.contains(*alias))
        {
            let (source_file, source_line) = self
                .unit_bindings
                .get(alias)
                .and_then(|bindings| bindings.source_location.as_ref())
                .map(|source_location| source_location.display())
                .unwrap_or_default();

            self.result.add_failure(
                ErrorBuilder::new(ErrorCategory::Naming)
                    .title(format!(
                        "alias \"{alias}\" from the component diagram not found in the sequence diagram"
                    ))
                    .field("alias", format!("\"{alias}\""))
                    .field("component source file", format!("\"{source_file}\""))
                    .field("component source line", source_line.to_string())
                    .fix(format!(
                        "add sequence participant \"{alias}\" in the sequence diagram, or remove it from the component diagram"
                    ))
                    .build(),
            );
        }

        for participant in self
            .observed_participants
            .iter()
            .filter(|participant| {
                !is_external_endpoint(participant)
                    && !self.unit_bindings.contains_key(*participant)
            })
        {
            let (source_file, source_line) = self
                .sequence_diagram
                .participant_source(participant)
                .map(|source_location| source_location.display())
                .unwrap_or_default();

            self.result.add_failure(
                ErrorBuilder::new(ErrorCategory::Naming)
                    .title(format!(
                        "participant \"{participant}\" from the sequence diagram not found in the component diagram"
                    ))
                    .field("participant", format!("\"{participant}\""))
                    .field("sequence source file", format!("\"{source_file}\""))
                    .field("sequence source line", source_line.to_string())
                    .fix(format!(
                        "add component unit alias \"{participant}\" in the component diagram, or remove it from the sequence diagram"
                    ))
                    .build(),
            );
        }
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
            let unit_pair = format_unit_pair(left_unit, right_unit);
            let left_interface_label = format!("interfaces for \"{left_unit}\"");
            let right_interface_label = format!("interfaces for \"{right_unit}\"");
            let (source_file, source_line) = sequence_call_source(call_context);

            self.result.add_failure(
                ErrorBuilder::new(ErrorCategory::Interface)
                    .title(format!(
                        "sequence-connected units \"{left_unit}\" and \"{right_unit}\" have no corresponding shared interface connection in the component diagram."
                    ))
                    .field("unit pair", unit_pair)
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

fn is_external_endpoint(participant: &str) -> bool {
    participant == EXTERNAL_ENDPOINT_NAME
}

fn call_involves_external_endpoint(call_context: &SequenceCallContext<'_>) -> bool {
    is_external_endpoint(call_context.caller_unit)
        || is_external_endpoint(call_context.callee_unit)
}

fn append_debug_log(
    diagnostics: &mut Diagnostics,
    observed_participants: &BTreeSet<String>,
    observed_call_contexts: &[SequenceCallContext<'_>],
    unit_bindings: &UnitBindings,
    connected_unit_pairs: &BTreeMap<(String, String), BTreeSet<String>>,
) {
    diagnostics.debug(|| "Expected unit aliases from component diagrams:".to_string());
    for alias in unit_bindings.keys() {
        diagnostics.debug(|| format!("  {alias}"));
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
    for (unit_alias, bindings) in unit_bindings {
        diagnostics.debug(|| {
            format!(
                "  {unit_alias} -> {}",
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
    let aliases: Vec<&String> = unit_bindings.keys().collect();

    for index in 0..aliases.len() {
        for other_index in (index + 1)..aliases.len() {
            let left_alias = aliases[index];
            let right_alias = aliases[other_index];
            let left_bindings = &unit_bindings[left_alias];
            let right_bindings = &unit_bindings[right_alias];
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

            connected_unit_pairs
                .insert((left_alias.clone(), right_alias.clone()), shared_interfaces);
        }
    }

    connected_unit_pairs
}

fn unit_source(unit_bindings: &UnitBindings, unit_alias: &str) -> (String, u32) {
    unit_bindings
        .get(unit_alias)
        .and_then(|bindings| bindings.source_location.as_ref())
        .map(|source_location| source_location.display())
        .unwrap_or_default()
}

fn sequence_call_source(call_context: &SequenceCallContext<'_>) -> (String, u32) {
    call_context.source_location.display()
}

fn format_unit_pair(left_unit: &str, right_unit: &str) -> String {
    format!("\"{left_unit}\" <-> \"{right_unit}\"")
}

#[cfg(test)]
#[path = "test/component_sequence_validator_test.rs"]
mod tests;
