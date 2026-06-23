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

use crate::models::{
    ComponentDiagramArchitecture, Errors, InternalApiIndex, InternalApiInterface,
    SequenceDiagramIndex,
};

/// Run component-vs-sequence naming validation.
pub fn validate_component_sequence(
    component_diagram: &ComponentDiagramArchitecture,
    sequence_diagram: &SequenceDiagramIndex,
    internal_api_diagram: Option<&InternalApiIndex>,
    errors: Errors,
) -> Errors {
    ComponentSequenceValidator::new(
        component_diagram,
        sequence_diagram,
        internal_api_diagram,
        errors,
    )
    .run()
}

struct ComponentSequenceValidator<'a> {
    observed_participants: &'a BTreeSet<String>,
    observed_call_contexts: Vec<SequenceCallContext<'a>>,
    connected_unit_pairs: BTreeMap<(String, String), BTreeSet<String>>,
    unit_bindings: BTreeMap<String, UnitInterfaces>,
    all_interfaces: BTreeSet<String>,
    internal_api_interfaces_by_id: Option<BTreeMap<String, &'a InternalApiInterface>>,
    errors: Errors,
}

#[derive(Clone, Default)]
struct UnitInterfaces {
    all_interfaces: BTreeSet<String>,
    required_interfaces: BTreeSet<String>,
    provided_interfaces: BTreeSet<String>,
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
        internal_api_diagram: Option<&'a InternalApiIndex>,
        errors: Errors,
    ) -> Self {
        let unit_bindings = build_unit_bindings(component_diagram);
        let all_interfaces =
            build_all_interfaces(component_diagram, &unit_bindings, internal_api_diagram);
        let observed_call_contexts =
            build_observed_call_contexts(sequence_diagram.observed_calls(), &unit_bindings);

        Self {
            observed_participants: sequence_diagram.used_participants(),
            observed_call_contexts,
            connected_unit_pairs: build_connected_unit_pairs(&unit_bindings),
            unit_bindings,
            all_interfaces,
            internal_api_interfaces_by_id: build_internal_api_interfaces_by_id(
                internal_api_diagram,
            ),
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
        for alias in self.unit_bindings.keys() {
            log.push_str(&format!("  {alias}\n"));
        }

        log.push_str("DEBUG: Observed participants from sequence diagrams:\n");
        for participant in self.observed_participants {
            log.push_str(&format!("  {participant}\n"));
        }

        log.push_str("DEBUG: Observed sequence calls from sequence diagrams:\n");
        for call_context in &self.observed_call_contexts {
            log.push_str(&format!(
                "  {} -> {} : {}\n",
                call_context.caller_unit, call_context.callee_unit, call_context.method
            ));
        }

        log.push_str("DEBUG: Unit interface targets from component diagrams:\n");
        for (unit_alias, bindings) in &self.unit_bindings {
            log.push_str(&format!(
                "  {unit_alias} -> {}\n",
                format_interface_names(&bindings.all_interfaces)
            ));
        }

        log.push_str(&format!(
            "DEBUG: All interfaces for self-call validation:\n  {}\n",
            format_interface_names(&self.all_interfaces)
        ));

        if let Some(internal_api_interfaces_by_id) = self.internal_api_interfaces_by_id.as_ref() {
            log.push_str("DEBUG: Internal API interfaces checked for method validation:\n");
            for interface_id in internal_api_interfaces_by_id.keys() {
                log.push_str(&format!("  {interface_id}\n"));
            }
        }

        log.push_str("DEBUG: Interface-connected unit pairs from component diagrams:\n");
        for ((left, right), interfaces) in &self.connected_unit_pairs {
            log.push_str(&format!(
                "  {left} <-> {right} via {}\n",
                format_interface_names(interfaces)
            ));
        }

        log
    }

    fn check_consistency(&mut self) {
        self.check_participant_aliases();
        self.check_interface_connected_units_have_sequence_calls();
        self.check_sequence_calls_have_interface_connections();
        self.check_sequence_call_interface_roles();
        self.check_sequence_call_method_consistency();
        self.check_interface_method_coverage();
    }

    fn check_participant_aliases(&mut self) {
        for alias in self
            .unit_bindings
            .keys()
            .filter(|alias| !self.observed_participants.contains(*alias))
        {
            self.errors.push(format!(
                "Naming consistency violation: component unit alias not found in sequence participants:\n\
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
            self.errors.push(format!(
                "Naming consistency violation: sequence participant not found in component unit aliases:\n\
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

            self.errors.push(format!(
                "Interface consistency violation: interface-connected units are missing a sequence function-call connection:\n\
                  Unit pair          : {unit_pair}\n\
                  Shared interfaces  : {shared_interfaces}\n\
                  Action             : Add a function-call connection between these units in a sequence diagram",
                unit_pair = format_unit_pair(left_unit, right_unit),
                shared_interfaces = format_interface_names(interfaces),
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

            self.errors.push(format!(
                "Interface consistency violation: sequence-connected units have no corresponding shared interface connection in the component diagram:\n\
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
                left_interfaces = format_interface_names(left_interfaces),
                right_interfaces = format_interface_names(right_interfaces),
            ));
        }
    }

    fn check_sequence_call_interface_roles(&mut self) {
        let mut seen_interactions = BTreeSet::new();

        for call_context in &self.observed_call_contexts {
            if extract_method_name(call_context.method).is_empty() {
                continue;
            }

            if !self.unit_bindings.contains_key(call_context.caller_unit)
                || !self.unit_bindings.contains_key(call_context.callee_unit)
            {
                continue;
            }

            if call_context.caller_unit == call_context.callee_unit {
                continue;
            }

            if !seen_interactions.insert((
                call_context.caller_unit.to_string(),
                call_context.callee_unit.to_string(),
            )) {
                continue;
            }

            let caller_bindings =
                unit_bindings_for_alias(&self.unit_bindings, call_context.caller_unit);

            if !call_context.has_shared_interfaces() {
                continue;
            }

            let callee_bindings =
                unit_bindings_for_alias(&self.unit_bindings, call_context.callee_unit);
            let directional_interfaces = intersect_interfaces(
                &caller_bindings.required_interfaces,
                &callee_bindings.provided_interfaces,
            );

            if !directional_interfaces.is_empty() {
                continue;
            }

            self.errors.push(format_sequence_role_consistency_error(
                call_context,
                &caller_bindings.required_interfaces,
                &callee_bindings.provided_interfaces,
            ));
        }
    }

    fn check_sequence_call_method_consistency(&mut self) {
        let Some(internal_api_interfaces_by_id) = self.internal_api_interfaces_by_id.as_ref()
        else {
            return;
        };

        let missing_internal_api_interfaces_by_unit =
            self.collect_missing_internal_api_interfaces_by_unit(internal_api_interfaces_by_id);
        for (unit_alias, missing_interfaces) in &missing_internal_api_interfaces_by_unit {
            self.errors
                .push(format_missing_internal_api_interface_error(
                    unit_alias,
                    missing_interfaces,
                ));
        }

        let mut seen_calls = BTreeSet::new();

        for call_context in &self.observed_call_contexts {
            let is_self_call = call_context.caller_unit == call_context.callee_unit;

            let method_name = extract_method_name(call_context.method);
            if method_name.is_empty() {
                continue;
            }

            let call_key = (
                call_context.caller_unit.to_string(),
                call_context.callee_unit.to_string(),
                method_name.to_string(),
            );
            if !seen_calls.insert(call_key) {
                continue;
            }

            if is_self_call {
                let matching_interfaces = matching_interfaces_with_method(
                    internal_api_interfaces_by_id,
                    &self.all_interfaces,
                    method_name,
                );

                if matching_interfaces.is_empty() {
                    self.errors.push(format_sequence_method_consistency_error(
                        call_context,
                        method_name,
                        "sequence self-call function name was not found in available interface methods",
                        "Declare this method on one of the available interfaces in the internal API diagram",
                    ));
                }

                continue;
            }

            if !call_context.has_shared_interfaces() {
                // The structural interface check above already reported that this
                // cross-unit call has no usable shared interface relation.
                continue;
            }

            if missing_internal_api_interfaces_by_unit.contains_key(call_context.caller_unit)
                || missing_internal_api_interfaces_by_unit.contains_key(call_context.callee_unit)
            {
                continue;
            }

            let caller_matching_interfaces = matching_interfaces_with_method(
                internal_api_interfaces_by_id,
                &call_context.caller_interfaces,
                method_name,
            );
            let callee_matching_interfaces = matching_interfaces_with_method(
                internal_api_interfaces_by_id,
                &call_context.callee_interfaces,
                method_name,
            );
            let shared_matching_interfaces =
                intersect_interfaces(&caller_matching_interfaces, &callee_matching_interfaces);

            if !shared_matching_interfaces.is_empty() {
                continue;
            }

            self.errors.push(format_sequence_method_consistency_error(
                call_context,
                method_name,
                "sequence function name was not found in the related interface methods",
                "Declare this method on a shared interface referenced by both participating units in the internal API diagram",
            ));
        }
    }

    fn check_interface_method_coverage(&mut self) {
        let Some(internal_api_interfaces_by_id) = self.internal_api_interfaces_by_id.as_ref()
        else {
            return;
        };

        let exercised_method_names = self.collect_exercised_method_names();

        for interface in internal_api_interfaces_by_id.values().copied() {
            let missing_methods: BTreeSet<String> = interface
                .method_names
                .difference(&exercised_method_names)
                .cloned()
                .collect();

            if missing_methods.is_empty() {
                continue;
            }

            self.errors.push(format!(
                "Coverage consistency violation: internal API interface functions are not exercised in sequence diagrams:\n\
                  Interface id        : \"{interface_id}\"\n\
                  Missing functions   : {missing_functions}\n\
                  Action              : Add sequence interactions that call each missing function",
                interface_id = interface.id,
                missing_functions = format_name_list(&missing_methods),
            ));
        }
    }

    fn collect_missing_internal_api_interfaces_by_unit(
        &self,
        internal_api_interfaces_by_id: &BTreeMap<String, &'a InternalApiInterface>,
    ) -> BTreeMap<String, BTreeSet<String>> {
        self.unit_bindings
            .iter()
            .filter_map(|(unit_alias, bindings)| {
                let missing_interfaces = missing_internal_api_interfaces(
                    internal_api_interfaces_by_id,
                    &bindings.all_interfaces,
                );

                if missing_interfaces.is_empty() {
                    None
                } else {
                    Some((unit_alias.clone(), missing_interfaces))
                }
            })
            .collect()
    }

    fn collect_exercised_method_names(&self) -> BTreeSet<String> {
        let mut exercised_method_names = BTreeSet::new();

        for call_context in &self.observed_call_contexts {
            let method_name = extract_method_name(call_context.method);
            if method_name.is_empty() {
                continue;
            }

            exercised_method_names.insert(method_name.to_string());
        }

        exercised_method_names
    }
}

fn build_connected_unit_pairs(
    unit_bindings: &BTreeMap<String, UnitInterfaces>,
) -> BTreeMap<(String, String), BTreeSet<String>> {
    let mut connected_unit_pairs = BTreeMap::new();
    let aliases: Vec<&String> = unit_bindings.keys().collect();

    for index in 0..aliases.len() {
        for other_index in (index + 1)..aliases.len() {
            let left_alias = aliases[index];
            let right_alias = aliases[other_index];
            let shared_interfaces: BTreeSet<String> = unit_bindings[left_alias]
                .all_interfaces
                .intersection(&unit_bindings[right_alias].all_interfaces)
                .cloned()
                .collect();

            if shared_interfaces.is_empty() {
                continue;
            }

            connected_unit_pairs
                .insert((left_alias.clone(), right_alias.clone()), shared_interfaces);
        }
    }

    connected_unit_pairs
}

fn build_unit_bindings(
    component_diagram: &ComponentDiagramArchitecture,
) -> BTreeMap<String, UnitInterfaces> {
    let mut unit_bindings = BTreeMap::new();

    for entity in component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_unit())
    {
        let Some(alias) = entity.alias.clone() else {
            continue;
        };

        let mut bindings = UnitInterfaces::default();

        for relation in &entity.relations {
            let Some(interface_id) = component_diagram
                .entities
                .iter()
                .find(|candidate| candidate.is_interface() && candidate.id == relation.target)
                .map(|candidate| candidate.id.clone())
            else {
                continue;
            };

            bindings.all_interfaces.insert(interface_id.clone());

            match relation.source_role.as_deref() {
                Some("Required") => {
                    bindings.required_interfaces.insert(interface_id);
                }
                Some("Provided") => {
                    bindings.provided_interfaces.insert(interface_id);
                }
                _ => {}
            }
        }

        unit_bindings.insert(alias, bindings);
    }

    unit_bindings
}

fn build_all_interfaces(
    component_diagram: &ComponentDiagramArchitecture,
    unit_bindings: &BTreeMap<String, UnitInterfaces>,
    internal_api_diagram: Option<&InternalApiIndex>,
) -> BTreeSet<String> {
    let mut interface_ids: BTreeSet<String> = component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_interface())
        .map(|entity| entity.id.clone())
        .collect();

    if let Some(internal_api_diagram) = internal_api_diagram {
        interface_ids.extend(
            internal_api_diagram
                .interfaces()
                .map(|interface| interface.id.clone()),
        );
    }

    for bindings in unit_bindings.values() {
        interface_ids.extend(bindings.all_interfaces.iter().cloned());
    }

    interface_ids
}

fn all_interfaces_for_alias(
    unit_bindings: &BTreeMap<String, UnitInterfaces>,
    alias: &str,
) -> BTreeSet<String> {
    unit_bindings
        .get(alias)
        .map(|bindings| bindings.all_interfaces.clone())
        .unwrap_or_default()
}

fn unit_bindings_for_alias(
    unit_bindings: &BTreeMap<String, UnitInterfaces>,
    alias: &str,
) -> UnitInterfaces {
    unit_bindings.get(alias).cloned().unwrap_or_default()
}

fn build_observed_call_contexts<'a>(
    observed_calls: &'a [crate::models::ObservedSequenceCall],
    unit_bindings: &BTreeMap<String, UnitInterfaces>,
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

fn intersect_interfaces(
    left_interfaces: &BTreeSet<String>,
    right_interfaces: &BTreeSet<String>,
) -> BTreeSet<String> {
    left_interfaces
        .intersection(right_interfaces)
        .cloned()
        .collect()
}

fn matching_interfaces_with_method(
    internal_api_interfaces_by_id: &BTreeMap<String, &InternalApiInterface>,
    interface_ids: &BTreeSet<String>,
    method_name: &str,
) -> BTreeSet<String> {
    interface_ids
        .iter()
        .filter(|interface_id| {
            matching_internal_api_interface_ids(internal_api_interfaces_by_id, interface_id)
                .into_iter()
                .filter_map(|matched_interface_id| {
                    internal_api_interfaces_by_id
                        .get(&matched_interface_id)
                        .copied()
                })
                .any(|interface| interface.method_names.contains(method_name))
        })
        .cloned()
        .collect()
}

fn build_internal_api_interfaces_by_id(
    internal_api_diagram: Option<&InternalApiIndex>,
) -> Option<BTreeMap<String, &InternalApiInterface>> {
    let mut interfaces_by_id = BTreeMap::new();

    let internal_api_diagram = internal_api_diagram?;

    for interface in internal_api_diagram.interfaces() {
        interfaces_by_id.insert(interface.id.clone(), interface);
    }

    Some(interfaces_by_id)
}

fn missing_internal_api_interfaces(
    internal_api_interfaces_by_id: &BTreeMap<String, &InternalApiInterface>,
    interface_ids: &BTreeSet<String>,
) -> BTreeSet<String> {
    interface_ids
        .iter()
        .filter(|interface_id| {
            !has_matching_internal_api_reference(internal_api_interfaces_by_id, interface_id)
        })
        .cloned()
        .collect()
}

fn matching_internal_api_interface_ids(
    internal_api_interfaces_by_id: &BTreeMap<String, &InternalApiInterface>,
    reference: &str,
) -> BTreeSet<String> {
    let mut interface_ids = BTreeSet::new();

    if internal_api_interfaces_by_id.contains_key(reference) {
        interface_ids.insert(reference.to_string());
    }

    interface_ids
}

fn has_matching_internal_api_reference(
    internal_api_interfaces_by_id: &BTreeMap<String, &InternalApiInterface>,
    reference: &str,
) -> bool {
    internal_api_interfaces_by_id.contains_key(reference)
}

fn format_sequence_role_consistency_error(
    call_context: &SequenceCallContext<'_>,
    caller_required_interfaces: &BTreeSet<String>,
    callee_provided_interfaces: &BTreeSet<String>,
) -> String {
    let sequence_call = format_sequence_call(
        call_context.caller_unit,
        call_context.callee_unit,
        call_context.method,
    );
    let shared_interfaces = intersect_interfaces(
        &call_context.caller_interfaces,
        &call_context.callee_interfaces,
    );

    let expected_interfaces = if shared_interfaces.is_empty() {
        intersect_interfaces(caller_required_interfaces, callee_provided_interfaces)
    } else {
        shared_interfaces
    };

    format!(
        "Interface consistency violation: sequence interaction does not match consumer/provider roles in the component diagram:\n\
          Sequence call       : {sequence_call}\n\
          Expected caller role: \"{caller_unit}\" should require shared interface(s) {expected_interfaces}\n\
          Expected callee role: \"{callee_unit}\" should provide shared interface(s) {expected_interfaces}\n\
          Action              : Reverse the sequence call or align the required/provided interface bindings in the component diagram",
        caller_unit = call_context.caller_unit,
        callee_unit = call_context.callee_unit,
        expected_interfaces = format_interface_names(&expected_interfaces),
    )
}

fn format_sequence_method_consistency_error(
    call_context: &SequenceCallContext<'_>,
    method_name: &str,
    description: &str,
    action: &str,
) -> String {
    let sequence_call = format_sequence_call(
        call_context.caller_unit,
        call_context.callee_unit,
        method_name,
    );

    format!(
        "Method consistency violation: {description}:\n\
          Sequence call       : {sequence_call}\n\
          Action              : {action}",
    )
}

fn format_sequence_call(caller_unit: &str, callee_unit: &str, method_name: &str) -> String {
    format!("\"{caller_unit}\" -> \"{callee_unit}\" : \"{method_name}\"")
}

fn format_unit_pair(left_unit: &str, right_unit: &str) -> String {
    format!("\"{left_unit}\" <-> \"{right_unit}\"")
}

fn format_missing_internal_api_interface_error(
    unit_alias: &str,
    missing_internal_api_interfaces: &BTreeSet<String>,
) -> String {
    format!(
        "Method consistency violation: Missing internal API interface:\n\
          Unit                : \"{unit_alias}\"\n\
          Missing interfaces  : {missing_interfaces}\n\
          Action              : Add the referenced interfaces to the internal API diagram or fix the component diagram references",
        missing_interfaces = format_interface_names(missing_internal_api_interfaces),
    )
}

fn format_interface_names(interfaces: &BTreeSet<String>) -> String {
    format_name_list(interfaces)
}

fn format_name_list(names: &BTreeSet<String>) -> String {
    if names.is_empty() {
        return "<none>".to_string();
    }

    names
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

fn extract_method_name(method: &str) -> &str {
    method.split('(').next().unwrap_or(method).trim()
}

#[cfg(test)]
#[path = "test/component_sequence_validator_test.rs"]
mod tests;
