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

use super::super::fixtures::*;
use super::*;
use crate::models::{
    ComponentDiagramInputs, ComponentRelationType, EndpointRole, LogicComponent,
    SequenceDiagramInputs,
};
use crate::ValidationResult;

fn validate(
    sequence_diagrams: SequenceDiagramInputs,
    internal_api: &InternalApiIndex,
) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(
        setup_result.is_empty(),
        "test fixture setup failed: {:?}",
        setup_result.failures
    );

    validate_sequence_internal_api(&sequence_index, internal_api, None)
}

fn validate_with_component_context(
    component_diagrams: ComponentDiagramInputs,
    sequence_diagrams: SequenceDiagramInputs,
    internal_api: &InternalApiIndex,
) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(
        setup_result.is_empty(),
        "test fixture setup failed: {:?}",
        setup_result.failures
    );

    validate_sequence_internal_api(&sequence_index, internal_api, Some(&component_arch))
}

fn unit_with_non_binding_interface(alias: &str, interface_id: &str) -> LogicComponent {
    let mut unit = unit_without_interfaces(alias);
    unit.relations = vec![relation(
        interface_id,
        ComponentRelationType::Dependency,
        EndpointRole::None,
    )];
    unit
}

#[test]
fn does_not_check_sequence_method_names_without_component_context() {
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec![])]);

    let validation_result = validate(sequence_diagrams, &internal_api);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn reports_internal_api_interface_function_not_exercised_without_method_name_check() {
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["OtherMethod"])]);

    let validation_result = validate(sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("internal API interface functions are not exercised in sequence diagrams"));
    assert!(validation_result.failures[0].contains("\"InternalInterface\""));
    assert!(validation_result.failures[0].contains("\"OtherMethod\""));
    assert!(validation_result
        .failures
        .iter()
        .all(|message| !message.contains("Method consistency failure")));
}

#[test]
fn reports_internal_api_interface_function_not_exercised() {
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData", "SetData"])]);

    let validation_result = validate(sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("internal API interface functions are not exercised in sequence diagrams"));
    assert!(validation_result.failures[0].contains("\"InternalInterface\""));
    assert!(validation_result.failures[0].contains("\"SetData\""));
}

#[test]
fn self_calls_count_as_internal_api_method_usage() {
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let validation_result = validate(sequence_diagrams, &internal_api);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn reports_sequence_function_missing_from_available_interfaces_with_component_context() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        unit("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["OtherMethod"])]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("sequence function name was not found in available interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\"")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"InternalInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
}

#[test]
fn reports_sequence_function_missing_when_shared_interface_has_no_direction_roles() {
    let component_diagrams = component_diagram(vec![
        unit_with_non_binding_interface("u1", "InternalInterface"),
        unit_with_non_binding_interface("u2", "InternalInterface"),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["OtherMethod"])]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("sequence function name was not found in available interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\"")
    }));
    assert!(validation_result
        .failures
        .iter()
        .all(|message| !message.contains("consumer/provider roles")));
    assert!(validation_result.failures.iter().all(|message| {
        !message.contains("sequence function name was not found in the related interface methods")
    }));
}

#[test]
fn reports_interface_function_not_exercised_in_sequence_diagrams_with_component_context() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        unit("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData", "SetData"])]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("internal API interface functions are not exercised in sequence diagrams"));
    assert!(validation_result.failures[0].contains("\"InternalInterface\""));
    assert!(validation_result.failures[0].contains("\"SetData\""));
}

#[test]
fn reports_unreferenced_internal_api_interface_function_not_exercised_without_self_calls() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        unit("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![
        ("InternalInterface", vec!["GetData"]),
        ("OtherInterface", vec!["SetData"]),
    ]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("internal API interface functions are not exercised in sequence diagrams"));
    assert!(validation_result.failures[0].contains("\"OtherInterface\""));
    assert!(validation_result.failures[0].contains("\"SetData\""));
}

#[test]
fn reports_self_call_method_mismatch_when_unit_has_missing_internal_api_interface() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["MissingInterface"], &[]),
        interface("MissingInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("sequence function name was not found")
            && message.contains("Sequence call       : \"u1\" -> \"u1\" : \"GetData\"")
    }));
}

#[test]
fn passes_when_sequence_function_exists_on_related_interface_with_component_context() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        unit("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn reports_self_call_function_missing_from_available_interfaces() {
    let component_diagrams = component_diagram(vec![unit_without_interfaces("u1")]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["OtherMethod"])]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("sequence function name was not found")
            && message.contains("Sequence call       : \"u1\" -> \"u1\" : \"GetData\"")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"InternalInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
}

#[test]
fn passes_when_self_call_uses_internal_api_interface_without_component_interfaces() {
    let component_diagrams = component_diagram(vec![unit_without_interfaces("u1")]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn passes_when_all_interface_functions_are_exercised_by_self_calls() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()"), ("u1", "u1", "SetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData", "SetData"])]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn reports_self_call_without_any_available_interfaces() {
    let component_diagrams = component_diagram(vec![unit_without_interfaces("u1")]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("sequence function name was not found"));
    assert!(validation_result.failures[0]
        .contains("Sequence call       : \"u1\" -> \"u1\" : \"GetData\""));
}

#[test]
fn reports_method_declared_only_on_caller_side_interfaces() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["SharedInterface", "CallerOnlyInterface"], &[]),
        unit("u2", &[], &["SharedInterface"]),
        interface("SharedInterface"),
        interface("CallerOnlyInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![
        ("SharedInterface", vec!["OtherMethod"]),
        ("CallerOnlyInterface", vec!["GetData"]),
    ]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\"")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"SharedInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
    assert!(validation_result
        .failures
        .iter()
        .all(|message| !message.contains("Missing functions   : \"GetData\"")));
}

#[test]
fn reports_method_declared_only_on_callee_side_interfaces() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["SharedInterface"], &[]),
        unit("u2", &[], &["SharedInterface", "CalleeOnlyInterface"]),
        interface("SharedInterface"),
        interface("CalleeOnlyInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![
        ("SharedInterface", vec!["OtherMethod"]),
        ("CalleeOnlyInterface", vec!["GetData"]),
    ]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\"")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"SharedInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
    assert!(validation_result
        .failures
        .iter()
        .all(|message| !message.contains("Missing functions   : \"GetData\"")));
}

#[test]
fn reports_method_declared_on_both_sides_but_not_on_shared_interface() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["SharedInterface", "CallerOnlyInterface"], &[]),
        unit("u2", &[], &["SharedInterface", "CalleeOnlyInterface"]),
        interface("SharedInterface"),
        interface("CallerOnlyInterface"),
        interface("CalleeOnlyInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![
        ("SharedInterface", vec!["OtherMethod"]),
        ("CallerOnlyInterface", vec!["GetData"]),
        ("CalleeOnlyInterface", vec!["GetData"]),
    ]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\"")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"SharedInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
    assert!(validation_result
        .failures
        .iter()
        .all(|message| !message.contains("Missing functions   : \"GetData\"")));
}

#[test]
fn reports_role_violation_when_method_exists_only_on_reverse_direction_interface() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &[], &["InternalInterface"]),
        unit("u2", &["InternalInterface"], &[]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("sequence interaction does not match consumer/provider roles"));
    assert!(validation_result.failures[0]
        .contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\""));
    assert!(validation_result.failures[0].contains(
        "Expected caller role: \"u1\" should require shared interface(s) \"InternalInterface\""
    ));
    assert!(validation_result.failures[0].contains(
        "Expected callee role: \"u2\" should provide shared interface(s) \"InternalInterface\""
    ));
    assert!(!validation_result.failures[0].contains("Method consistency failure"));
}

#[test]
fn passes_when_method_interface_matches_call_direction_roles() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        unit("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert!(
        validation_result.failures.is_empty(),
        "Expected pass, got: {:?}",
        validation_result.failures
    );
}
