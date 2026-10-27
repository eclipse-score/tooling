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

//! Validation: messages between an actor and the SEooC are checked against
//! the public API. External systems and gates are not checked.
//!
//! With a component diagram, a call is checked against the public interfaces
//! the element provides (actor call) or requires (callback). An element that
//! binds no public interface, or a missing component diagram, falls back to
//! the whole public API by name.

use std::collections::BTreeSet;

use super::shared::{
    best_string_suggestion, build_container_units, build_observed_call_contexts,
    build_unit_bindings, extract_method_name, format_name_list, format_sequence_call,
    ContainerUnits, SequenceCallContext, UnitBindings,
};
use crate::models::{
    ComponentDiagramArchitecture, Environment, PublicApiIndex, SequenceDiagramIndex,
};
use crate::results::{ErrorBuilder, ErrorCategory};
use crate::ValidationResult;

#[derive(Clone, Copy)]
enum BoundaryDirection {
    /// Actor calls a SEooC element.
    Incoming,
    /// SEooC element calls back an actor.
    Callback,
}

/// Calls from an actor must use public API operations; calls to an actor
/// (callbacks) are labelled with a public API interface or operation.
pub fn validate_sequence_public_api_boundary(
    sequence_diagram: &SequenceDiagramIndex,
    public_api: &PublicApiIndex,
    component_diagram: Option<&ComponentDiagramArchitecture>,
) -> ValidationResult {
    let mut result = ValidationResult::default();
    let mut seen_calls = BTreeSet::new();
    let (unit_bindings, container_units) = match component_diagram {
        Some(component_diagram) => (
            build_unit_bindings(component_diagram),
            build_container_units(component_diagram),
        ),
        None => (UnitBindings::new(), ContainerUnits::new()),
    };

    for call in build_observed_call_contexts(
        sequence_diagram.observed_calls(),
        &unit_bindings,
        &container_units,
    ) {
        let (direction, element_interfaces) =
            match (call.caller_environment, call.callee_environment) {
                (Some(Environment::Actor), None) => (
                    BoundaryDirection::Incoming,
                    &call.callee_provided_interfaces,
                ),
                (None, Some(Environment::Actor)) => (
                    BoundaryDirection::Callback,
                    &call.caller_required_interfaces,
                ),
                _ => continue,
            };

        let label = extract_method_name(&call.method);
        if label.is_empty()
            || !seen_calls.insert((
                call.caller_unit.clone(),
                call.callee_unit.clone(),
                label.to_string(),
            ))
        {
            continue;
        }

        let scope: BTreeSet<String> = element_interfaces
            .iter()
            .filter(|interface_id| public_api.declares_interface(interface_id))
            .cloned()
            .collect();

        if is_declared(public_api, direction, &scope, label) {
            continue;
        }

        let candidates = boundary_candidates(public_api, direction, &scope);
        result.add_failure(format_boundary_call_error(
            &call,
            label,
            direction,
            &scope,
            best_string_suggestion(label, candidates.iter().map(String::as_str)).as_deref(),
        ));
    }

    result
}

fn is_declared(
    public_api: &PublicApiIndex,
    direction: BoundaryDirection,
    scope: &BTreeSet<String>,
    label: &str,
) -> bool {
    if scope.is_empty() {
        return match direction {
            BoundaryDirection::Incoming => public_api.declares_method(label),
            BoundaryDirection::Callback => {
                public_api.declares_interface(label) || public_api.declares_method(label)
            }
        };
    }

    scope.iter().any(|interface_id| {
        public_api.interface_declares_method(interface_id, label)
            || (matches!(direction, BoundaryDirection::Callback)
                && public_api.is_interface_named(interface_id, label))
    })
}

fn boundary_candidates(
    public_api: &PublicApiIndex,
    direction: BoundaryDirection,
    scope: &BTreeSet<String>,
) -> Vec<String> {
    let callback = matches!(direction, BoundaryDirection::Callback);

    if scope.is_empty() {
        let mut candidates: Vec<String> = public_api.method_names().map(str::to_string).collect();
        if callback {
            candidates.extend(public_api.api_names().cloned());
        }
        return candidates;
    }

    let mut candidates = Vec::new();
    for interface_id in scope {
        candidates.extend(
            public_api
                .interface_method_names(interface_id)
                .map(str::to_string),
        );
        if callback {
            candidates.push(
                interface_id
                    .rsplit('.')
                    .next()
                    .unwrap_or(interface_id)
                    .to_string(),
            );
        }
    }
    candidates
}

fn format_boundary_call_error(
    call: &SequenceCallContext,
    label: &str,
    direction: BoundaryDirection,
    scope: &BTreeSet<String>,
    suggested: Option<&str>,
) -> String {
    let sequence_call = format_sequence_call(&call.caller_unit, &call.callee_unit, label);
    let (source_file, source_line) = call.source_location.display();
    let (title, fix, suggestion_subject) = match direction {
        BoundaryDirection::Incoming => (
            format!(
                "sequence function \"{label}\" from actor call {sequence_call} in the sequence diagram not found in the public API diagram"
            ),
            format!(
                "add method \"{label}\" to a public API interface in the public API diagram, or call a declared public API operation in sequence call {sequence_call}"
            ),
            "method",
        ),
        BoundaryDirection::Callback => (
            format!(
                "callback \"{label}\" from sequence call {sequence_call} in the sequence diagram not found in the public API diagram"
            ),
            format!(
                "label the callback with its public API interface name or operation, or declare \"{label}\" in the public API diagram"
            ),
            "interface or method",
        ),
    };

    let mut error = ErrorBuilder::new(ErrorCategory::Method)
        .title(title)
        .field("sequence call", sequence_call)
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string());

    if !scope.is_empty() {
        let interfaces = format_name_list(scope);
        let detail = match direction {
            BoundaryDirection::Incoming => format!(
                "\"{}\" provides public API interface(s) {interfaces}; none declares the method",
                call.callee_unit
            ),
            BoundaryDirection::Callback => format!(
                "\"{}\" requires public API interface(s) {interfaces}; the label names none of them or their methods",
                call.caller_unit
            ),
        };
        error = error.field("detail", detail);
    }

    if let Some(suggested) = suggested {
        error = error.suggest(label, Some(suggestion_subject), suggested);
    }

    error.fix(fix).build()
}

#[cfg(test)]
#[path = "test/sequence_public_api_boundary_validator_test.rs"]
mod tests;
