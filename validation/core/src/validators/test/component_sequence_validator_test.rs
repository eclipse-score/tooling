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
use crate::models::{ComponentDiagramInputs, ComponentType, LogicComponent, SequenceDiagramInputs};
use crate::ValidationResult;
use component_diagram::SourceLocation;

fn validate(
    component_diagrams: ComponentDiagramInputs,
    sequence_diagrams: SequenceDiagramInputs,
) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    validate_component_sequence(&component_arch, &sequence_index)
}

#[test]
fn passes_when_aliases_and_participants_are_identical() {
    let component_diagrams = component_diagrams(&["unit_1", "unit_2"]);
    let sequence_diagrams = sequence_diagrams(&["unit_1", "unit_2"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert!(validation_result.is_empty());
}

#[test]
fn reports_missing_and_extra() {
    let component_diagrams = component_diagrams(&["unit_1", "unit_2", "unit_3"]);
    let sequence_diagrams = sequence_diagrams(&["unit_2", "unit_4"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert!(!validation_result.is_empty());
    assert_eq!(validation_result.failures.len(), 3);

    let missing_count = validation_result
        .failures
        .iter()
        .filter(|msg| msg.contains("unit alias not found in sequence participants"))
        .count();
    let unexpected_count = validation_result
        .failures
        .iter()
        .filter(|msg| msg.contains("sequence participant not found in component unit aliases"))
        .count();

    assert_eq!(missing_count, 2);
    assert_eq!(unexpected_count, 1);
}

#[test]
fn units_without_alias_are_ignored() {
    let component_diagrams = ComponentDiagramInputs {
        entities: vec![LogicComponent {
            id: "module_a.unit_1".to_string(),
            name: Some("unit_1".to_string()),
            alias: None,
            parent_id: None,
            element_type: ComponentType::Component,
            stereotype: Some("unit".to_string()),
            relations: Vec::new(),
            source_location: SourceLocation::new("", 0),
        }],
    };
    let sequence_diagrams = sequence_diagrams(&[]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert!(validation_result.is_empty());
}

#[test]
fn reports_alias_missing_from_participants() {
    let component_diagrams = component_diagrams(&["u1", "u2"]);
    let sequence_diagrams = sequence_diagrams(&["u1"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("\"u2\""));
}

#[test]
fn reports_participant_not_in_aliases() {
    let component_diagrams = component_diagrams(&["u1"]);
    let sequence_diagrams = sequence_diagrams(&["u1", "orphan"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("\"orphan\""));
}

#[test]
fn reports_missing_component_alias_and_interface_connection_for_sequence_call() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "orphan", "GetData()")]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("sequence participant not found in component unit aliases")
            && message.contains("\"orphan\"")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message
            .contains("sequence-connected units have no corresponding shared interface connection")
            && message.contains("\"u1\"")
            && message.contains("\"orphan\"")
            && message.contains("\"InternalInterface\"")
    }));
}

#[test]
fn reports_missing_sequence_call_for_interface_connected_units() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["u1", "u2"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("interface-connected units are missing a sequence function-call connection"));
    assert!(validation_result.failures[0].contains("\"InternalInterface\""));
}

#[test]
fn reports_missing_participant_and_missing_sequence_call_for_interface_connected_units() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["u1"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("component unit alias not found in sequence participants")
            && message.contains("\"u2\"")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message
            .contains("interface-connected units are missing a sequence function-call connection")
            && message.contains("\"InternalInterface\"")
    }));
}

#[test]
fn reports_sequence_call_without_corresponding_shared_interface_connection() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["CallerInterface"]),
        unit("u2", &["CalleeInterface"]),
        interface("CallerInterface"),
        interface("CalleeInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("sequence-connected units have no corresponding shared interface connection"));
    assert!(validation_result.failures[0].contains("\"CallerInterface\""));
    assert!(validation_result.failures[0].contains("\"CalleeInterface\""));
}

#[test]
fn passes_when_interface_connected_units_have_sequence_call() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert!(validation_result.is_empty());
}
