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
    all_interfaces_for_alias, build_unit_bindings, format_name_list, intersect_interfaces,
    UnitBindings,
};
use crate::models::{ComponentDiagramArchitecture, SequenceDiagramIndex};
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
    observed_participants: &'a BTreeSet<String>,
    observed_call_contexts: Vec<SequenceCallContext<'a>>,
    connected_unit_pairs: ConnectedUnitPairs,
    unit_bindings: UnitBindings,
    result: ValidationResult,
}

struct SequenceCallContext<'a> {
    caller_unit: &'a str,
    callee_unit: &'a str,
    method: &'a str,
    caller_interfaces: BTreeSet<String>,
    callee_interfaces: BTreeSet<String>,
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

    fn has_shared_interfaces(&self) -> bool {
        !self.caller_interfaces.is_disjoint(&self.callee_interfaces)
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
            self.result.add_failure(format!(
                "Naming consistency failure: component unit alias not found in sequence participants:\n\
                  Unit alias         : \"{alias}\"\n\
                  Source             : Component diagram unit aliases\n\
                  Action             : Add a matching sequence participant for this unit alias",
            ));
        }

        for participant in self
            .observed_participants
            .iter()
            .filter(|participant| !self.unit_bindings.contains_key(*participant))
        {
            self.result.add_failure(format!(
                "Naming consistency failure: sequence participant not found in component unit aliases:\n\
                  Participant        : \"{participant}\"\n\
                  Source             : Sequence diagram participants\n\
                  Action             : Add a matching component unit alias or remove this participant",
            ));
        }
    }

    fn check_interface_connected_units_have_sequence_calls(&mut self) {
        for ((left_unit, right_unit), interfaces) in &self.connected_unit_pairs {
            if self.has_observed_call_between_units(left_unit, right_unit) {
                continue;
            }

            self.result.add_failure(format!(
                "Interface consistency failure: interface-connected units are missing a sequence function-call connection:\n\
                  Unit pair          : {unit_pair}\n\
                  Shared interfaces  : {shared_interfaces}\n\
                  Action             : Add a function-call connection between these units in a sequence diagram",
                unit_pair = format_unit_pair(left_unit, right_unit),
                shared_interfaces = format_name_list(interfaces),
            ));
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

            self.result.add_failure(format!(
                "Interface consistency failure: sequence-connected units have no corresponding shared interface connection in the component diagram:\n\
                  Unit pair          : {unit_pair}\n\
                  Interfaces for \"{left_unit}\"  : {left_interfaces}\n\
                  Interfaces for \"{right_unit}\" : {right_interfaces}\n\
                Action             : Add a shared interface relation between these units in the component diagram",
                unit_pair = format_unit_pair(
                    call_context.normalized_left_unit(),
                    call_context.normalized_right_unit(),
                ),
                left_unit = call_context.normalized_left_unit(),
                right_unit = call_context.normalized_right_unit(),
                left_interfaces = format_name_list(left_interfaces),
                right_interfaces = format_name_list(right_interfaces),
            ));
        }
    }
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

fn build_observed_call_contexts<'a>(
    observed_calls: &'a [crate::models::ObservedSequenceCall],
    unit_bindings: &UnitBindings,
) -> Vec<SequenceCallContext<'a>> {
    observed_calls
        .iter()
        .map(|call| {
            let caller_interfaces = all_interfaces_for_alias(unit_bindings, &call.caller);
            let callee_interfaces = all_interfaces_for_alias(unit_bindings, &call.callee);

            SequenceCallContext {
                caller_unit: call.caller.as_str(),
                callee_unit: call.callee.as_str(),
                method: call.method.as_str(),
                caller_interfaces,
                callee_interfaces,
            }
        })
        .collect()
}

fn format_unit_pair(left_unit: &str, right_unit: &str) -> String {
    format!("\"{left_unit}\" <-> \"{right_unit}\"")
}

#[cfg(test)]
#[path = "test/component_sequence_validator_test.rs"]
mod tests;
