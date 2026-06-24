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

use crate::models::{
    ComponentDiagramArchitecture, Errors, InternalApiIndex, InternalApiInterface,
    LogicComponentExt, ObservedSequenceCall, SequenceDiagramIndex,
};

/// Run sequence-vs-internal-API method and coverage validation.
pub fn validate_sequence_internal_api(
    sequence_diagram: &SequenceDiagramIndex,
    internal_api_diagram: &InternalApiIndex,
    component_diagram: Option<&ComponentDiagramArchitecture>,
    errors: Errors,
) -> Errors {
    SequenceInternalApiValidator::new(
        sequence_diagram,
        internal_api_diagram,
        component_diagram,
        errors,
    )
    .run()
}

struct SequenceInternalApiValidator<'a> {
    sequence_diagram: &'a SequenceDiagramIndex,
    internal_api_interfaces_by_id: BTreeMap<String, &'a InternalApiInterface>,
    component_context: Option<ComponentContext<'a>>,
    errors: Errors,
}

struct ComponentContext<'a> {
    observed_call_contexts: Vec<SequenceCallContext<'a>>,
    unit_bindings: BTreeMap<String, BTreeSet<String>>,
    all_interfaces: BTreeSet<String>,
}

struct SequenceCallContext<'a> {
    caller_unit: &'a str,
    callee_unit: &'a str,
    method: &'a str,
    caller_interfaces: BTreeSet<String>,
    callee_interfaces: BTreeSet<String>,
}

impl SequenceCallContext<'_> {
    fn has_shared_interfaces(&self) -> bool {
        !self.caller_interfaces.is_disjoint(&self.callee_interfaces)
    }
}

impl<'a> SequenceInternalApiValidator<'a> {
    fn new(
        sequence_diagram: &'a SequenceDiagramIndex,
        internal_api_diagram: &'a InternalApiIndex,
        component_diagram: Option<&ComponentDiagramArchitecture>,
        errors: Errors,
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
            errors,
        }
    }

    fn run(mut self) -> Errors {
        self.errors.debug_output = self.build_debug_log();
        self.check_sequence_call_method_consistency_with_component_context();
        self.check_interface_method_coverage();
        self.errors
    }

    fn build_debug_log(&self) -> String {
        let mut log = String::new();

        if let Some(component_context) = self.component_context.as_ref() {
            log.push_str(
                "DEBUG: Sequence calls checked against related internal API interfaces:\n",
            );
            for call_context in &component_context.observed_call_contexts {
                log.push_str(&format!(
                    "  {} -> {} : {}\n",
                    call_context.caller_unit, call_context.callee_unit, call_context.method
                ));
            }

            log.push_str("DEBUG: Unit interface targets from component diagrams:\n");
            for (unit_alias, bindings) in &component_context.unit_bindings {
                log.push_str(&format!(
                    "  {unit_alias} -> {}\n",
                    format_name_list(bindings)
                ));
            }

            log.push_str(&format!(
                "DEBUG: All interfaces for self-call validation:\n  {}\n",
                format_name_list(&component_context.all_interfaces)
            ));
        } else {
            log.push_str(
                "DEBUG: Sequence method-name consistency skipped because component context is unavailable:\n",
            );
            for call in self.sequence_diagram.observed_calls() {
                log.push_str(&format!(
                    "  {} -> {} : {}\n",
                    call.caller, call.callee, call.method
                ));
            }
        }

        log.push_str("DEBUG: Internal API interfaces available for sequence validation:\n");
        for interface_id in self.internal_api_interfaces_by_id.keys() {
            log.push_str(&format!("  {interface_id}\n"));
        }

        log
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
                continue;
            }

            if is_self_call {
                let matching_interfaces = matching_interfaces_with_method(
                    &self.internal_api_interfaces_by_id,
                    &component_context.all_interfaces,
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
                // The structural component-sequence validator reports that this
                // cross-unit call has no usable shared interface relation.
                continue;
            }

            if units_with_missing_internal_api_interfaces.contains(call_context.caller_unit)
                || units_with_missing_internal_api_interfaces.contains(call_context.callee_unit)
            {
                continue;
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

            if !caller_matching_interfaces.is_disjoint(&callee_matching_interfaces) {
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

            self.errors.push(format_interface_method_coverage_error(
                interface,
                &missing_methods,
            ));
        }
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

fn build_unit_bindings(
    component_diagram: &ComponentDiagramArchitecture,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut unit_bindings = BTreeMap::new();

    for entity in component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_unit())
    {
        let Some(alias) = entity.alias.clone() else {
            continue;
        };

        let mut bindings = BTreeSet::new();

        for relation in &entity.relations {
            let Some(interface_id) = component_diagram
                .entities
                .iter()
                .find(|candidate| candidate.is_interface() && candidate.id == relation.target)
                .map(|candidate| candidate.id.clone())
            else {
                continue;
            };

            bindings.insert(interface_id);
        }

        unit_bindings.insert(alias, bindings);
    }

    unit_bindings
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

fn all_interfaces_for_alias(
    unit_bindings: &BTreeMap<String, BTreeSet<String>>,
    alias: &str,
) -> BTreeSet<String> {
    unit_bindings.get(alias).cloned().unwrap_or_default()
}

fn build_observed_call_contexts<'a>(
    observed_calls: &'a [ObservedSequenceCall],
    unit_bindings: &BTreeMap<String, BTreeSet<String>>,
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
    unit_bindings: &BTreeMap<String, BTreeSet<String>>,
    internal_api_interfaces_by_id: &BTreeMap<String, &InternalApiInterface>,
) -> BTreeSet<String> {
    unit_bindings
        .iter()
        .filter(|(_, bindings)| {
            bindings.iter().any(|interface_id| {
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
        "Coverage consistency violation: internal API interface functions are not exercised in sequence diagrams:\n\
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
        "Method consistency violation: {description}:\n\
          Sequence call       : {sequence_call}\n\
          Action              : {action}",
    )
}

fn format_sequence_call(caller_unit: &str, callee_unit: &str, method_name: &str) -> String {
    format!("\"{caller_unit}\" -> \"{callee_unit}\" : \"{method_name}\"")
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
#[path = "test/sequence_internal_api_validator_test.rs"]
mod tests;
