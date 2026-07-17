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
use crate::models::ComponentDiagramInputs;
use crate::ValidationResult;

fn validate(
    component_diagrams: ComponentDiagramInputs,
    internal_api: &InternalApiIndex,
) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    assert!(
        setup_result.is_empty(),
        "test fixture setup failed: {:?}",
        setup_result.failures
    );
    validate_component_internal_api(&component_arch, internal_api)
}

#[test]
fn reports_missing_component_interface_declared_by_internal_api() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface_with_parent("InternalInterface", Some("component_example")),
    ]);
    let internal_api = internal_api_index(vec![("OtherInterface", vec!["GetData"])]);

    let validation_result = validate(component_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("Missing internal API interface"));
    assert!(validation_result.failures[0]
        .contains("Missing interfaces  : \"component_example.InternalInterface\""));
    assert!(!validation_result.failures[0].contains("Unit                :"));
}

#[test]
fn reports_each_missing_component_interface_once() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface", "InternalInterface1"]),
        unit("u2", &["InternalInterface"]),
        interface_with_parent("InternalInterface", Some("component_example")),
        interface_with_parent("InternalInterface1", Some("component_example")),
    ]);
    let internal_api = internal_api_index(vec![(
        "component_example.InternalInterface",
        vec!["GetData"],
    )]);

    let validation_result = validate(component_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("Missing internal API interface"));
    assert!(validation_result.failures[0]
        .contains("Missing interfaces  : \"component_example.InternalInterface1\""));
}

#[test]
fn reports_missing_component_interface_even_without_unit_relation() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &[]),
        interface_with_parent("UnusedInterface", Some("component_example")),
    ]);
    let internal_api = internal_api_index(vec![]);

    let validation_result = validate(component_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("Missing internal API interface"));
    assert!(validation_result.failures[0]
        .contains("Missing interfaces  : \"component_example.UnusedInterface\""));
}

#[test]
fn reports_all_missing_component_interfaces_in_one_message() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface", "InternalInterface1"]),
        unit("u2", &["InternalInterface"]),
        unit("u3", &["InternalInterface"]),
        interface_with_parent("InternalInterface", Some("component_example")),
        interface_with_parent("InternalInterface1", Some("component_example")),
    ]);
    let internal_api = internal_api_index(vec![(
        "component_example.InternalInterface",
        vec!["GetData"],
    )]);

    let validation_result = validate(component_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("Missing internal API interface"));
    assert!(validation_result.failures[0]
        .contains("Missing interfaces  : \"component_example.InternalInterface1\""));
}

#[test]
fn reports_missing_component_interface_without_sequence_method_call() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        interface_with_parent("InternalInterface", Some("component_example")),
    ]);
    let internal_api = internal_api_index(vec![("OtherInterface", vec!["GetData"])]);

    let validation_result = validate(component_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("Missing internal API interface"));
    assert!(validation_result.failures[0]
        .contains("Missing interfaces  : \"component_example.InternalInterface\""));
}

#[test]
fn reports_case_mismatch_between_component_and_internal_api_interface_names() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface_with_parent("InternalInterface", Some("component_example")),
    ]);
    let internal_api = internal_api_index(vec![("internalinterface", vec!["GetData"])]);

    let validation_result = validate(component_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("Missing internal API interface"));
    assert!(validation_result.failures[0]
        .contains("Missing interfaces  : \"component_example.InternalInterface\""));
}

#[test]
fn matches_internal_api_by_component_interface_id_when_alias_differs() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["pkg.InternalInterface"]),
        unit("u2", &["pkg.InternalInterface"]),
        interface_with_parent("InternalInterface", Some("pkg")),
    ]);
    let internal_api = internal_api_index(vec![("pkg.InternalInterface", vec!["GetData"])]);

    let validation_result = validate(component_diagrams, &internal_api);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn ignores_component_interface_without_parent_id() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        interface("InternalInterface"), // no parent_id
    ]);
    let internal_api = internal_api_index(vec![]);

    let validation_result = validate(component_diagrams, &internal_api);

    assert!(validation_result.failures.is_empty());
}
