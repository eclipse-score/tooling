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
//! interfaces only when component context is available. Messages from a gate
//! to an element are checked against the interfaces the element provides
//! (internal API or public API). Calls to or from a component lifeline are
//! checked against the interfaces both ends share.

use std::collections::{BTreeMap, BTreeSet};

use super::shared::{
    best_string_suggestion, build_container_units, build_observed_call_contexts,
    build_unit_bindings, extract_method_name, format_name_list, format_sequence_call,
    intersect_interfaces, SequenceCallContext, UnitBindings, UnitInterfaces,
};
use crate::models::{
    ComponentDiagramArchitecture, Environment, InternalApiIndex, InternalApiInterface,
    LogicComponentExt, PublicApiIndex, SequenceDiagramIndex,
};
use crate::results::{ErrorBuilder, ErrorCategory};
use crate::{Diagnostics, ValidationResult};

/// Run sequence-vs-internal-API method and coverage validation. `public_api`
/// supplies the methods of public interfaces bound in the component diagram.
pub fn validate_sequence_internal_api(
    sequence_diagram: &SequenceDiagramIndex,
    internal_api_diagram: &InternalApiIndex,
    component_diagram: Option<&ComponentDiagramArchitecture>,
    public_api: Option<&PublicApiIndex>,
) -> ValidationResult {
    SequenceInternalApiValidator::new(
        sequence_diagram,
        internal_api_diagram,
        component_diagram,
        public_api,
    )
    .run()
}

struct SequenceInternalApiValidator<'a> {
    sequence_diagram: &'a SequenceDiagramIndex,
    internal_api_interfaces_by_id: BTreeMap<String, &'a InternalApiInterface>,
    component_context: Option<ComponentContext>,
    public_api: Option<&'a PublicApiIndex>,
    result: ValidationResult,
}

struct ComponentContext {
    observed_call_contexts: Vec<SequenceCallContext>,
    unit_bindings: UnitBindings,
    all_interfaces: BTreeSet<String>,
    units_with_missing_internal_api_interfaces: BTreeSet<String>,
}

impl<'a> SequenceInternalApiValidator<'a> {
    fn new(
        sequence_diagram: &'a SequenceDiagramIndex,
        internal_api_diagram: &'a InternalApiIndex,
        component_diagram: Option<&ComponentDiagramArchitecture>,
        public_api: Option<&'a PublicApiIndex>,
    ) -> Self {
        let internal_api_interfaces_by_id =
            build_internal_api_interfaces_by_id(internal_api_diagram);
        let component_context = component_diagram.map(|component_diagram| {
            build_component_context(
                component_diagram,
                sequence_diagram,
                internal_api_diagram,
                &internal_api_interfaces_by_id,
                public_api,
            )
        });

        Self {
            sequence_diagram,
            internal_api_interfaces_by_id,
            component_context,
            public_api,
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

        let mut seen_calls = BTreeSet::new();
        let mut consistency_errors = Vec::new();

        for call_context in &component_context.observed_call_contexts {
            let method_name = extract_method_name(&call_context.method);
            if method_name.is_empty() {
                continue;
            }

            let call_key = (
                call_context.caller_unit.clone(),
                call_context.callee_unit.clone(),
                method_name.to_string(),
            );
            if !seen_calls.insert(call_key) {
                // Repeated calls exercise the same method relation once.
                continue;
            }

            if call_context.involves_environment() {
                // Actor messages are checked against the public API and
                // external systems are not checked; only gate messages
                // into an element are checked here.
                if let Some(error) =
                    self.check_gate_call(component_context, call_context, method_name)
                {
                    consistency_errors.push(error);
                }
                continue;
            }

            let is_self_call = call_context.caller_unit == call_context.callee_unit;

            if let Some(error) = self.check_method_exists_in_internal_api(
                component_context,
                call_context,
                method_name,
            ) {
                consistency_errors.push(error);
                continue;
            }

            if is_self_call || call_context.ends_overlap() {
                // Self-calls and calls between an element and its own part
                // are not checked for cross-unit role consistency.
                continue;
            }

            if call_context.involves_container {
                // A component or SEooC lifeline has no interface roles of
                // its own; only the shared interfaces are checked.
                if let Some(error) = self.check_container_call_consistency(
                    component_context,
                    call_context,
                    method_name,
                ) {
                    consistency_errors.push(error);
                }
                continue;
            }

            if let Some(error) =
                self.check_cross_unit_call_consistency(component_context, call_context, method_name)
            {
                consistency_errors.push(error);
            }
        }

        for error in consistency_errors {
            self.result.add_failure(error);
        }
    }

    /// Interfaces of `interface_ids` that declare `method_name` in the
    /// internal API or the public API diagram.
    fn interfaces_declaring(
        &self,
        interface_ids: &BTreeSet<String>,
        method_name: &str,
    ) -> BTreeSet<String> {
        interface_ids
            .iter()
            .filter(|interface_id| self.interface_declares_method(interface_id, method_name))
            .cloned()
            .collect()
    }

    fn interface_declares_method(&self, interface_id: &str, method_name: &str) -> bool {
        self.internal_api_interfaces_by_id
            .get(interface_id)
            .is_some_and(|interface| interface.method_names.contains(method_name))
            || self.public_api.is_some_and(|public_api| {
                public_api.interface_declares_method(interface_id, method_name)
            })
    }

    /// Methods declared by `interface_ids` in the internal API or the public
    /// API diagram.
    fn method_candidates(&self, interface_ids: &BTreeSet<String>) -> BTreeSet<String> {
        let mut method_names = BTreeSet::new();

        for interface_id in interface_ids {
            if let Some(interface) = self
                .internal_api_interfaces_by_id
                .get(interface_id.as_str())
            {
                method_names.extend(interface.method_names.iter().cloned());
            }
            if let Some(public_api) = self.public_api {
                method_names.extend(
                    public_api
                        .interface_method_names(interface_id)
                        .map(str::to_string),
                );
            }
        }

        method_names
    }

    /// A message from a gate to an element must name an operation of an
    /// interface the element provides.
    fn check_gate_call(
        &self,
        component_context: &ComponentContext,
        call_context: &SequenceCallContext,
        method_name: &str,
    ) -> Option<String> {
        if call_context.caller_environment != Some(Environment::Gate)
            || call_context.callee_environment.is_some()
            || call_context.callee_contained_units.is_empty()
        {
            return None;
        }

        if call_context.callee_contained_units.iter().any(|unit_id| {
            component_context
                .units_with_missing_internal_api_interfaces
                .contains(unit_id)
        }) {
            // The component-internal-api validator reports missing interface declarations first.
            return None;
        }

        let provided_interfaces = &call_context.callee_provided_interfaces;
        if !self
            .interfaces_declaring(provided_interfaces, method_name)
            .is_empty()
        {
            return None;
        }

        let candidate_method_names = self.method_candidates(provided_interfaces);
        let suggested_method = best_string_suggestion(
            method_name,
            candidate_method_names.iter().map(String::as_str),
        );

        Some(format_gate_call_error(
            call_context,
            method_name,
            suggested_method.as_deref(),
        ))
    }

    fn check_method_exists_in_internal_api(
        &self,
        component_context: &ComponentContext,
        call_context: &SequenceCallContext,
        method_name: &str,
    ) -> Option<String> {
        let matching_interfaces =
            self.interfaces_declaring(&component_context.all_interfaces, method_name);

        if matching_interfaces.is_empty() {
            if call_context.involves_container
                && self
                    .public_api
                    .is_some_and(|public_api| public_api.declares_method(method_name))
            {
                return None;
            }

            let candidate_method_names = self.method_candidates(&component_context.all_interfaces);
            let suggested_method = best_string_suggestion(
                method_name,
                candidate_method_names.iter().map(String::as_str),
            );
            return Some(format_sequence_method_consistency_error(
                call_context,
                method_name,
                "sequence function name was not found in available interface methods",
                suggested_method.as_deref(),
            ));
        }

        None
    }

    fn check_cross_unit_call_consistency(
        &self,
        component_context: &ComponentContext,
        call_context: &SequenceCallContext,
        method_name: &str,
    ) -> Option<String> {
        if !call_context.has_shared_interfaces() {
            // The structural component-sequence validator reports missing shared interface relations.
            return None;
        }

        if component_context
            .units_with_missing_internal_api_interfaces
            .contains(&call_context.caller_unit)
            || component_context
                .units_with_missing_internal_api_interfaces
                .contains(&call_context.callee_unit)
        {
            // The component-internal-api validator reports missing interface declarations first.
            return None;
        }

        let caller_matching_interfaces =
            self.interfaces_declaring(&call_context.caller_interfaces, method_name);
        let callee_matching_interfaces =
            self.interfaces_declaring(&call_context.callee_interfaces, method_name);

        let shared_method_interfaces =
            intersect_interfaces(&caller_matching_interfaces, &callee_matching_interfaces);

        if shared_method_interfaces.is_empty() {
            let shared_interface_ids = intersect_interfaces(
                &call_context.caller_interfaces,
                &call_context.callee_interfaces,
            );
            let candidate_method_names = self.method_candidates(&shared_interface_ids);
            let suggested_method = best_string_suggestion(
                method_name,
                candidate_method_names.iter().map(String::as_str),
            );
            return Some(format_sequence_method_consistency_error(
                call_context,
                method_name,
                "sequence function name was not found in the related interface methods",
                suggested_method.as_deref(),
            ));
        }

        self.check_cross_unit_call_role_consistency(
            component_context,
            call_context,
            method_name,
            &shared_method_interfaces,
        )
    }

    /// A call to or from a component or SEooC lifeline must name an operation
    /// of an interface both ends share. Roles are not checked.
    fn check_container_call_consistency(
        &self,
        component_context: &ComponentContext,
        call_context: &SequenceCallContext,
        method_name: &str,
    ) -> Option<String> {
        if !call_context.has_shared_interfaces() {
            // The structural component-sequence validator reports missing shared interface relations.
            return None;
        }

        if call_context
            .caller_contained_units
            .iter()
            .chain(&call_context.callee_contained_units)
            .any(|unit_id| {
                component_context
                    .units_with_missing_internal_api_interfaces
                    .contains(unit_id)
            })
        {
            // The component-internal-api validator reports missing interface declarations first.
            return None;
        }

        let shared_interface_ids = intersect_interfaces(
            &call_context.caller_interfaces,
            &call_context.callee_interfaces,
        );
        if !self
            .interfaces_declaring(&shared_interface_ids, method_name)
            .is_empty()
        {
            return None;
        }

        let candidate_method_names = self.method_candidates(&shared_interface_ids);
        let suggested_method = best_string_suggestion(
            method_name,
            candidate_method_names.iter().map(String::as_str),
        );

        Some(format_sequence_method_consistency_error(
            call_context,
            method_name,
            "sequence function name was not found in the related interface methods",
            suggested_method.as_deref(),
        ))
    }

    fn check_cross_unit_call_role_consistency(
        &self,
        component_context: &ComponentContext,
        call_context: &SequenceCallContext,
        method_name: &str,
        shared_method_interfaces: &BTreeSet<String>,
    ) -> Option<String> {
        let caller_bindings = component_context
            .unit_bindings
            .get(&call_context.caller_unit)?;
        let callee_bindings = component_context
            .unit_bindings
            .get(&call_context.callee_unit)?;

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
        let exercised_method_names = self.collect_exercised_method_names();

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

    /// Internal API operations are exercised by calls between elements and by
    /// gate messages into an element that pass the gate-call check. Calls
    /// with an actor or external system, and calls to a gate, do not count.
    fn collect_exercised_method_names(&self) -> BTreeSet<String> {
        let mut exercised_method_names = BTreeSet::new();

        match &self.component_context {
            Some(component_context) => {
                for call_context in &component_context.observed_call_contexts {
                    let method_name = extract_method_name(&call_context.method);
                    if !method_name.is_empty()
                        && exercises_internal_api(
                            call_context.caller_environment,
                            call_context.callee_environment,
                            || {
                                self.check_gate_call(component_context, call_context, method_name)
                                    .is_none()
                            },
                        )
                    {
                        exercised_method_names.insert(method_name.to_string());
                    }
                }
            }
            None => {
                for call in self.sequence_diagram.observed_calls() {
                    let method_name = extract_method_name(&call.method);
                    if !method_name.is_empty()
                        && exercises_internal_api(
                            call.caller_environment,
                            call.callee_environment,
                            || true,
                        )
                    {
                        exercised_method_names.insert(method_name.to_string());
                    }
                }
            }
        }

        exercised_method_names
    }
}

/// `gate_call_is_valid` is only evaluated for a message from a gate to an
/// element.
fn exercises_internal_api(
    caller_environment: Option<Environment>,
    callee_environment: Option<Environment>,
    gate_call_is_valid: impl FnOnce() -> bool,
) -> bool {
    match (caller_environment, callee_environment) {
        (Some(Environment::Actor | Environment::External), _)
        | (_, Some(Environment::Actor | Environment::External | Environment::Gate)) => false,
        (Some(Environment::Gate), None) => gate_call_is_valid(),
        (None, None) => true,
    }
}

fn append_debug_log(
    diagnostics: &mut Diagnostics,
    component_context: &Option<ComponentContext>,
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
        for (unit_id, bindings) in &component_context.unit_bindings {
            diagnostics.debug(|| {
                format!(
                    "  {unit_id} -> {}",
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

    if !sequence_diagram.skipped_messages().is_empty() {
        diagnostics
            .debug(|| "Sequence messages not checked (reply, create or delete):".to_string());
        for message in sequence_diagram.skipped_messages() {
            diagnostics.debug(|| {
                format!(
                    "  {} -> {} : {} ({:?})",
                    message.sender, message.receiver, message.message, message.kind
                )
            });
        }
    }
}

fn build_component_context(
    component_diagram: &ComponentDiagramArchitecture,
    sequence_diagram: &SequenceDiagramIndex,
    internal_api_diagram: &InternalApiIndex,
    internal_api_interfaces_by_id: &BTreeMap<String, &InternalApiInterface>,
    public_api: Option<&PublicApiIndex>,
) -> ComponentContext {
    let unit_bindings = build_unit_bindings(component_diagram);
    let container_units = build_container_units(component_diagram);
    let all_interfaces = build_all_interfaces(component_diagram, internal_api_diagram);
    let observed_call_contexts = build_observed_call_contexts(
        sequence_diagram.observed_calls(),
        &unit_bindings,
        &container_units,
    );
    let units_with_missing_internal_api_interfaces =
        collect_units_with_missing_internal_api_interfaces(
            &unit_bindings,
            internal_api_interfaces_by_id,
            public_api,
        );

    ComponentContext {
        observed_call_contexts,
        unit_bindings,
        all_interfaces,
        units_with_missing_internal_api_interfaces,
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

/// Units binding an interface that has neither an internal API nor a public
/// API declaration.
fn collect_units_with_missing_internal_api_interfaces(
    unit_bindings: &UnitBindings,
    internal_api_interfaces_by_id: &BTreeMap<String, &InternalApiInterface>,
    public_api: Option<&PublicApiIndex>,
) -> BTreeSet<String> {
    unit_bindings
        .iter()
        .filter(|(_, bindings)| {
            bindings.all_interfaces.iter().any(|interface_id| {
                !internal_api_interfaces_by_id.contains_key(interface_id.as_str())
                    && !public_api
                        .is_some_and(|public_api| public_api.declares_interface(interface_id))
            })
        })
        .map(|(unit_id, _)| unit_id.clone())
        .collect()
}

fn format_interface_method_coverage_error(
    interface: &InternalApiInterface,
    missing_methods: &BTreeSet<String>,
) -> String {
    let missing_functions = format_name_list(missing_methods);
    let (source_file, source_line) = interface.source_location.display();

    ErrorBuilder::new(ErrorCategory::Coverage)
        .title(format!(
            "methods {missing_functions} declared on internal API interface \"{}\" in the internal API diagram are not exercised in the sequence diagram",
            interface.id
        ))
        .field("interface id", format!("\"{}\"", interface.id))
        .field("internal API source file", format!("\"{source_file}\""))
        .field("internal API source line", source_line.to_string())
        .field("missing functions", missing_functions.clone())
        .fix(format!(
            "add sequence interactions for functions {missing_functions} in the sequence diagram, or remove function declarations {missing_functions} in internal API interface \"{}\"",
            interface.id
        ))
        .build()
}

fn format_gate_call_error(
    call_context: &SequenceCallContext,
    method_name: &str,
    suggested_method: Option<&str>,
) -> String {
    let sequence_call = format_sequence_call(
        &call_context.caller_unit,
        &call_context.callee_unit,
        method_name,
    );
    let (source_file, source_line) = call_context.source_location.display();

    let mut error = ErrorBuilder::new(ErrorCategory::Method)
        .title(format!(
            "sequence function \"{method_name}\" from gate message {sequence_call} in the sequence diagram is not declared on an interface provided by the callee"
        ))
        .field("sequence call", sequence_call.clone())
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .field(
            "detail",
            format!(
                "no interface provided by \"{}\" declares the method",
                call_context.callee_unit
            ),
        );

    if let Some(suggested_method) = suggested_method {
        error = error.suggest(method_name, Some("method"), suggested_method);
    }

    error
        .fix(format!(
            "add method \"{method_name}\" to an internal API or public API interface provided by \"{}\", or remove sequence function \"{method_name}\" in sequence call {sequence_call} in the sequence diagram",
            call_context.callee_unit
        ))
        .build()
}

fn format_sequence_method_consistency_error(
    call_context: &SequenceCallContext,
    method_name: &str,
    description: &str,
    suggested_method: Option<&str>,
) -> String {
    let sequence_call = format_sequence_call(
        &call_context.caller_unit,
        &call_context.callee_unit,
        method_name,
    );
    let (source_file, source_line) = call_context.source_location.display();

    let mut error = ErrorBuilder::new(ErrorCategory::Method)
        .title(format!(
            "sequence function \"{method_name}\" from sequence call {sequence_call} in the sequence diagram not found in the internal API diagram"
        ))
        .field("sequence call", sequence_call.clone())
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .field("detail", description);

    if let Some(suggested_method) = suggested_method {
        error = error.suggest(method_name, Some("method"), suggested_method);
    }

    error
        .fix(format!(
            "add method \"{method_name}\" in a matching internal API interface in the internal API diagram, or remove sequence function \"{method_name}\" in sequence call {sequence_call} in the sequence diagram"
        ))
        .build()
}

fn format_sequence_role_consistency_error(
    call_context: &SequenceCallContext,
    method_name: &str,
    expected_interfaces: &BTreeSet<String>,
) -> String {
    let sequence_call = format_sequence_call(
        &call_context.caller_unit,
        &call_context.callee_unit,
        method_name,
    );

    let expected_interfaces = format_name_list(expected_interfaces);
    let (source_file, source_line) = call_context.source_location.display();

    ErrorBuilder::new(ErrorCategory::Interface)
        .title(format!(
            "sequence call {sequence_call} in the sequence diagram does not match the required/provided interface roles in the component diagram"
        ))
        .field("sequence call", sequence_call.clone())
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .field(
            "expected caller role",
            format!(
                "\"{}\" should require shared interface(s) {}",
                call_context.caller_unit, expected_interfaces
            ),
        )
        .field(
            "expected callee role",
            format!(
                "\"{}\" should provide shared interface(s) {}",
                call_context.callee_unit, expected_interfaces
            ),
        )
        .fix(format!(
            "add required/provided interface bindings for shared interface(s) {expected_interfaces} in the component diagram, or remove sequence call {sequence_call} in the sequence diagram"
        ))
        .build()
}

#[cfg(test)]
#[path = "test/sequence_internal_api_validator_test.rs"]
mod tests;
