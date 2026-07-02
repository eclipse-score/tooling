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

//! Validation: compare sequence-diagram usage with methods declared by
//! internal API interfaces. Method names are checked against related shared
//! interfaces only when component context is available.

use std::collections::{BTreeMap, BTreeSet};

use super::shared::{
    build_observed_call_contexts, build_unit_bindings, extract_method_name, format_name_list,
    format_sequence_call, intersect_interfaces, SequenceCallContext, UnitBindings, UnitInterfaces,
};
use crate::models::{
    ComponentDiagramArchitecture, InternalApiIndex, InternalApiInterface, LogicComponentExt,
    SequenceDiagramIndex,
};
use crate::{Diagnostics, ValidationResult};

/// Run sequence-vs-internal-API method and coverage validation.
pub fn validate_sequence_internal_api(
    sequence_diagram: &SequenceDiagramIndex,
    internal_api_diagram: &InternalApiIndex,
    component_diagram: Option<&ComponentDiagramArchitecture>,
) -> ValidationResult {
    SequenceInternalApiValidator::new(sequence_diagram, internal_api_diagram, component_diagram)
        .run()
}

struct SequenceInternalApiValidator<'a> {
    sequence_diagram: &'a SequenceDiagramIndex,
    internal_api_interfaces_by_id: BTreeMap<String, &'a InternalApiInterface>,
    component_context: Option<ComponentContext<'a>>,
    result: ValidationResult,
}

struct ComponentContext<'a> {
    observed_call_contexts: Vec<SequenceCallContext<'a>>,
    unit_bindings: UnitBindings,
    all_interfaces: BTreeSet<String>,
}

impl<'a> SequenceInternalApiValidator<'a> {
    fn new(
        sequence_diagram: &'a SequenceDiagramIndex,
        internal_api_diagram: &'a InternalApiIndex,
        component_diagram: Option<&ComponentDiagramArchitecture>,
    ) -> Self {
        let internal_api_interfaces_by_id =
            build_internal_api_interfaces_by_id(internal_api_diagram);
        let component_context = component_diagram.map(|component_diagram| {
            build_component_context(component_diagram, sequence_diagram, internal_api_diagram)
        });

        Self {
            sequence_diagram,
            internal_api_interfaces_by_id,
            component_context,
            result: ValidationResult::default(),
        }
    }

    fn run(mut self) -> ValidationResult {
        append_debug_log(
            &mut self.result.diagnostics,
            &self.component_context,
            self.sequence_diagram,
            &self.internal_api_interfaces_by_id,
        );
        self.check_sequence_call_method_consistency_with_component_context();
        self.check_interface_method_coverage();
        self.result
    }

    fn check_sequence_call_method_consistency_with_component_context(&mut self) {
        let Some(component_context) = self.component_context.as_ref() else {
            return;
        };
        let units_with_missing_internal_api_interfaces =
            collect_units_with_missing_internal_api_interfaces(
                &component_context.unit_bindings,
                &self.internal_api_interfaces_by_id,
            );

        let mut seen_calls = BTreeSet::new();
        let mut consistency_errors = Vec::new();

        for call_context in &component_context.observed_call_contexts {
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
                // Repeated calls exercise the same method relation once.
                continue;
            }

            if let Some(error) = self.check_method_exists_in_internal_api(
                component_context,
                call_context,
                method_name,
            ) {
                consistency_errors.push(error);
                continue;
            }

            if is_self_call {
                // Self-calls are not checked for cross-unit role consistency.
                continue;
            }

            if let Some(error) = self.check_cross_unit_call_consistency(
                component_context,
                &units_with_missing_internal_api_interfaces,
                call_context,
                method_name,
            ) {
                consistency_errors.push(error);
            }
        }

        for error in consistency_errors {
            self.result.add_failure(error);
        }
    }

    fn check_method_exists_in_internal_api(
        &self,
        component_context: &ComponentContext<'_>,
        call_context: &SequenceCallContext<'_>,
        method_name: &str,
    ) -> Option<String> {
        let matching_interfaces = matching_interfaces_with_method(
            &self.internal_api_interfaces_by_id,
            &component_context.all_interfaces,
            method_name,
        );

        if matching_interfaces.is_empty() {
            return Some(format_sequence_method_consistency_error(
                call_context,
                method_name,
                "sequence function name was not found in available interface methods",
                "Declare this method on one of the available interfaces in the internal API diagram",
            ));
        }

        None
    }

    fn check_cross_unit_call_consistency(
        &self,
        component_context: &ComponentContext<'_>,
        units_with_missing_internal_api_interfaces: &BTreeSet<String>,
        call_context: &SequenceCallContext<'_>,
        method_name: &str,
    ) -> Option<String> {
        if !call_context.has_shared_interfaces() {
            // The structural component-sequence validator reports missing shared interface relations.
            return None;
        }

        if units_with_missing_internal_api_interfaces.contains(call_context.caller_unit)
            || units_with_missing_internal_api_interfaces.contains(call_context.callee_unit)
        {
            // The component-internal-api validator reports missing interface declarations first.
            return None;
        }

        let caller_matching_interfaces = matching_interfaces_with_method(
            &self.internal_api_interfaces_by_id,
            &call_context.caller_interfaces,
            method_name,
        );
        let callee_matching_interfaces = matching_interfaces_with_method(
            &self.internal_api_interfaces_by_id,
            &call_context.callee_interfaces,
            method_name,
        );

        let shared_method_interfaces =
            intersect_interfaces(&caller_matching_interfaces, &callee_matching_interfaces);

        if shared_method_interfaces.is_empty() {
            return Some(format_sequence_method_consistency_error(
                call_context,
                method_name,
                "sequence function name was not found in the related interface methods",
                "Declare this method on a shared interface referenced by both participating units in the internal API diagram",
            ));
        }

        self.check_cross_unit_call_role_consistency(
            component_context,
            call_context,
            method_name,
            &shared_method_interfaces,
        )
    }

    fn check_cross_unit_call_role_consistency(
        &self,
        component_context: &ComponentContext<'_>,
        call_context: &SequenceCallContext<'_>,
        method_name: &str,
        shared_method_interfaces: &BTreeSet<String>,
    ) -> Option<String> {
        let caller_bindings = component_context
            .unit_bindings
            .get(call_context.caller_unit)?;
        let callee_bindings = component_context
            .unit_bindings
            .get(call_context.callee_unit)?;

        let caller_method_role_interfaces =
            intersect_interfaces(shared_method_interfaces, &role_interfaces(caller_bindings));

        let callee_method_role_interfaces =
            intersect_interfaces(shared_method_interfaces, &role_interfaces(callee_bindings));

        if caller_method_role_interfaces.is_empty()
            || callee_method_role_interfaces.is_empty()
            || intersect_interfaces(
                &caller_method_role_interfaces,
                &callee_method_role_interfaces,
            )
            .is_empty()
        {
            log::warn!("sequence call between units \"{}\" and \"{}\" for method \"{}\" has shared interface(s) {:?} but no matching required/provided roles in the component diagram",
                call_context.caller_unit, call_context.callee_unit, method_name, shared_method_interfaces);
            return None;
        }

        let caller_method_required_interfaces = intersect_interfaces(
            shared_method_interfaces,
            &caller_bindings.required_interfaces,
        );

        let callee_method_provided_interfaces = intersect_interfaces(
            shared_method_interfaces,
            &callee_bindings.provided_interfaces,
        );

        let directional_method_interfaces = intersect_interfaces(
            &caller_method_required_interfaces,
            &callee_method_provided_interfaces,
        );

        if directional_method_interfaces.is_empty() {
            return Some(format_sequence_role_consistency_error(
                call_context,
                method_name,
                shared_method_interfaces,
            ));
        }

        None
    }

    fn check_interface_method_coverage(&mut self) {
        let exercised_method_names = collect_exercised_method_names(self.sequence_diagram);

        for interface in self.internal_api_interfaces_by_id.values().copied() {
            let missing_methods: BTreeSet<String> = interface
                .method_names
                .difference(&exercised_method_names)
                .cloned()
                .collect();

            if missing_methods.is_empty() {
                continue;
            }

            self.result
                .add_failure(format_interface_method_coverage_error(
                    interface,
                    &missing_methods,
                ));
        }
    }
}

fn append_debug_log(
    diagnostics: &mut Diagnostics,
    component_context: &Option<ComponentContext<'_>>,
    sequence_diagram: &SequenceDiagramIndex,
    internal_api_interfaces_by_id: &BTreeMap<String, &InternalApiInterface>,
) {
    if let Some(component_context) = component_context.as_ref() {
        diagnostics.debug(|| {
            "Sequence calls checked against related internal API interfaces:".to_string()
        });
        for call_context in &component_context.observed_call_contexts {
            diagnostics.debug(|| {
                format!(
                    "  {} -> {} : {}",
                    call_context.caller_unit, call_context.callee_unit, call_context.method
                )
            });
        }

        diagnostics.debug(|| "Unit interface targets from component diagrams:".to_string());
        for (unit_alias, bindings) in &component_context.unit_bindings {
            diagnostics.debug(|| {
                format!(
                    "  {unit_alias} -> {}",
                    format_name_list(&bindings.all_interfaces)
                )
            });
        }

        diagnostics.debug(|| {
            format!(
                "All interfaces for self-call validation:\n  {}",
                format_name_list(&component_context.all_interfaces)
            )
        });
    } else {
        diagnostics.debug(|| {
            "Sequence method-name consistency skipped because component context is unavailable:"
                .to_string()
        });
        for call in sequence_diagram.observed_calls() {
            diagnostics.debug(|| format!("  {} -> {} : {}", call.caller, call.callee, call.method));
        }
    }

    diagnostics.debug(|| "Internal API interfaces available for sequence validation:".to_string());
    for interface_id in internal_api_interfaces_by_id.keys() {
        diagnostics.debug(|| format!("  {interface_id}"));
    }
}

fn build_component_context<'a>(
    component_diagram: &ComponentDiagramArchitecture,
    sequence_diagram: &'a SequenceDiagramIndex,
    internal_api_diagram: &InternalApiIndex,
) -> ComponentContext<'a> {
    let unit_bindings = build_unit_bindings(component_diagram);
    let all_interfaces = build_all_interfaces(component_diagram, internal_api_diagram);
    let observed_call_contexts =
        build_observed_call_contexts(sequence_diagram.observed_calls(), &unit_bindings);

    ComponentContext {
        observed_call_contexts,
        unit_bindings,
        all_interfaces,
    }
}

fn build_all_interfaces(
    component_diagram: &ComponentDiagramArchitecture,
    internal_api_diagram: &InternalApiIndex,
) -> BTreeSet<String> {
    let mut interface_ids: BTreeSet<String> = component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_interface())
        .map(|entity| entity.id.clone())
        .collect();

    interface_ids.extend(
        internal_api_diagram
            .interfaces()
            .map(|interface| interface.id.clone()),
    );

    interface_ids
}

fn matching_interfaces_with_method(
    internal_api_interfaces_by_id: &BTreeMap<String, &InternalApiInterface>,
    interface_ids: &BTreeSet<String>,
    method_name: &str,
) -> BTreeSet<String> {
    interface_ids
        .iter()
        .filter(|interface_id| {
            internal_api_interfaces_by_id
                .get(interface_id.as_str())
                .is_some_and(|interface| interface.method_names.contains(method_name))
        })
        .cloned()
        .collect()
}

fn role_interfaces(bindings: &UnitInterfaces) -> BTreeSet<String> {
    bindings
        .required_interfaces
        .union(&bindings.provided_interfaces)
        .cloned()
        .collect()
}

fn build_internal_api_interfaces_by_id(
    internal_api_diagram: &InternalApiIndex,
) -> BTreeMap<String, &InternalApiInterface> {
    let mut interfaces_by_id = BTreeMap::new();

    for interface in internal_api_diagram.interfaces() {
        interfaces_by_id.insert(interface.id.clone(), interface);
    }

    interfaces_by_id
}

fn collect_units_with_missing_internal_api_interfaces(
    unit_bindings: &UnitBindings,
    internal_api_interfaces_by_id: &BTreeMap<String, &InternalApiInterface>,
) -> BTreeSet<String> {
    unit_bindings
        .iter()
        .filter(|(_, bindings)| {
            bindings.all_interfaces.iter().any(|interface_id| {
                !internal_api_interfaces_by_id.contains_key(interface_id.as_str())
            })
        })
        .map(|(unit_alias, _)| unit_alias.clone())
        .collect()
}

fn collect_exercised_method_names(sequence_diagram: &SequenceDiagramIndex) -> BTreeSet<String> {
    let mut exercised_method_names = BTreeSet::new();

    for call in sequence_diagram.observed_calls() {
        let method_name = extract_method_name(&call.method);
        if method_name.is_empty() {
            continue;
        }

        exercised_method_names.insert(method_name.to_string());
    }

    exercised_method_names
}

fn format_interface_method_coverage_error(
    interface: &InternalApiInterface,
    missing_methods: &BTreeSet<String>,
) -> String {
    format!(
        "Coverage consistency failure: internal API interface functions are not exercised in sequence diagrams:\n\
          Interface id        : \"{interface_id}\"\n\
          Missing functions   : {missing_functions}\n\
          Action              : Add sequence interactions that call each missing function",
        interface_id = interface.id,
        missing_functions = format_name_list(missing_methods),
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
        "Method consistency failure: {description}:\n\
          Sequence call       : {sequence_call}\n\
          Action              : {action}",
    )
}

fn format_sequence_role_consistency_error(
    call_context: &SequenceCallContext<'_>,
    method_name: &str,
    expected_interfaces: &BTreeSet<String>,
) -> String {
    let sequence_call = format_sequence_call(
        call_context.caller_unit,
        call_context.callee_unit,
        method_name,
    );

    format!(
        "Interface consistency failure: sequence interaction does not match consumer/provider roles in the component diagram:\n\
          Sequence call       : {sequence_call}\n\
          Expected caller role: \"{caller_unit}\" should require shared interface(s) {expected_interfaces}\n\
          Expected callee role: \"{callee_unit}\" should provide shared interface(s) {expected_interfaces}\n\
          Action              : Reverse the sequence call or align the required/provided interface bindings in the component diagram",
        caller_unit = call_context.caller_unit,
        callee_unit = call_context.callee_unit,
        expected_interfaces = format_name_list(expected_interfaces),
    )
}

#[cfg(test)]
#[path = "test/sequence_internal_api_validator_test.rs"]
mod tests;
