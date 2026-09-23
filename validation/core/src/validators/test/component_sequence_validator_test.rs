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
use crate::models::{ComponentDiagramInputs, SequenceDiagramInputs};
use crate::ValidationResult;

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
fn passes_when_participant_uids_match_component_unit_ids() {
    let component_diagrams = component_diagram(vec![
        unit_without_interfaces("unit_1"),
        unit_without_interfaces("unit_2"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["unit_1", "unit_2"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert!(validation_result.is_empty());
}

#[test]
fn reports_missing_and_extra() {
    let component_diagrams = component_diagram(vec![
        unit_without_interfaces("unit_1"),
        unit_without_interfaces("unit_2"),
        unit_without_interfaces("unit_3"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["unit_2", "unit_4"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert!(!validation_result.is_empty());
    assert_eq!(validation_result.failures.len(), 3);

    let missing_count = validation_result
        .failures
        .iter()
        .filter(|msg| {
            msg.contains(
                "from the component diagram not found as a participant uid in the sequence diagram",
            )
        })
        .count();
    let unexpected_count = validation_result
        .failures
        .iter()
        .filter(|msg| msg.contains("from the sequence diagram not found in the component diagram"))
        .count();

    assert_eq!(missing_count, 2);
    assert_eq!(unexpected_count, 1);
}

#[test]
fn units_without_alias_are_validated_by_id() {
    let component_diagrams = component_diagram(vec![unit_with_id("module_a.unit_1", None)]);
    let sequence_diagrams = sequence_diagrams(&[]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("\"unit_1\""));
}

#[test]
fn reports_unit_id_missing_from_participants() {
    let component_diagrams = component_diagram(vec![
        unit_without_interfaces("u1"),
        unit_without_interfaces("u2"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["u1"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("\"u2\""));
}

#[test]
fn reports_participant_uid_missing_from_component_diagram() {
    let component_diagrams = component_diagram(vec![unit_without_interfaces("u1")]);
    let sequence_diagrams = sequence_diagrams(&["u1", "orphan"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("\"orphan\""));
}

#[test]
fn reports_unmatched_sequence_participant_and_missing_interface_connection_for_sequence_call() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "orphan", "GetData()")]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("from the sequence diagram not found in the component diagram")
            && message.contains("\"orphan\"")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message
            .contains("have no corresponding shared interface connection in the component diagram")
            && message.contains("\"u1\"")
            && message.contains("\"orphan\"")
            && message.contains("\"InternalInterface\"")
    }));
}

#[test]
fn reports_missing_sequence_call_for_interface_connected_units() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        unit("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["u1", "u2"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("have no corresponding function-call in the sequence diagram"));
    assert!(validation_result.failures[0].contains("\"InternalInterface\""));
}

#[test]
fn reports_missing_participant_and_missing_sequence_call_for_interface_connected_units() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        unit("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["u1"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains(
            "from the component diagram not found as a participant uid in the sequence diagram",
        ) && message.contains("\"u2\"")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("have no corresponding function-call in the sequence diagram")
            && message.contains("\"InternalInterface\"")
    }));
}

#[test]
fn reports_sequence_call_without_corresponding_shared_interface_connection() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["CallerInterface"], &[]),
        unit("u2", &[], &["CalleeInterface"]),
        interface("CallerInterface"),
        interface("CalleeInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("have no corresponding shared interface connection in the component diagram"));
    assert!(validation_result.failures[0].contains("\"CallerInterface\""));
    assert!(validation_result.failures[0].contains("\"CalleeInterface\""));
}

#[test]
fn passes_when_interface_connected_units_have_sequence_call() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        unit("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert!(validation_result.is_empty());
}

#[test]
fn passes_when_participant_uid_matches_component_id_but_reference_names_differ() {
    let component_diagrams = component_diagram(vec![
        unit_with_fields(
            "validation.core.example.unit_1",
            Some("u1"),
            Some("Component Unit One"),
            None,
            &["InternalInterface"],
            &[],
        ),
        unit_with_fields(
            "validation.core.example.unit_2",
            Some("u2"),
            Some("Component Unit Two"),
            None,
            &[],
            &["InternalInterface"],
        ),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls_with_custom_participants(
        vec![
            sequence_participant_with_fields(
                "validation.core.example.unit_1",
                Some("participant_a"),
                "Unit One",
            ),
            sequence_participant_with_fields(
                "validation.core.example.unit_2",
                Some("participant_b"),
                "Unit Two",
            ),
        ],
        &[("participant_a", "participant_b", "GetData()")],
    );

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert!(validation_result.is_empty());
}

#[test]
fn reports_uid_mismatch_even_when_alias_and_display_name_match_component_units() {
    let component_diagrams = component_diagram(vec![
        unit_with_fields(
            "validation.core.example.unit_1",
            Some("participant_a"),
            Some("Component Unit One"),
            None,
            &[],
            &[],
        ),
        unit_with_fields(
            "validation.core.example.unit_2",
            Some("participant_b"),
            Some("Component Unit Two"),
            None,
            &[],
            &[],
        ),
    ]);
    let sequence_diagrams = sequence_calls_with_custom_participants(
        vec![
            sequence_participant_with_fields(
                "validation.core.example.other_unit_1",
                Some("participant_a"),
                "participant_a",
            ),
            sequence_participant_with_fields(
                "validation.core.example.other_unit_2",
                Some("participant_b"),
                "participant_b",
            ),
        ],
        &[],
    );

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 4);
    let missing_component_units = validation_result
        .failures
        .iter()
        .filter(|message| {
            message.contains(
                "from the component diagram not found as a participant uid in the sequence diagram",
            )
        })
        .count();
    let unexpected_participant_uids = validation_result
        .failures
        .iter()
        .filter(|message| {
            message.contains("from the sequence diagram not found in the component diagram")
        })
        .count();

    assert_eq!(missing_component_units, 2);
    assert_eq!(unexpected_participant_uids, 2);
}

#[test]
fn reports_mismatch_when_sequence_calls_use_raw_ids_instead_of_declared_participant_reference_names(
) {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["InternalInterface"], &[]),
        unit("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls_with_participants(
        &["u1", "u2"],
        &[(
            "validation.core.example.u1",
            "validation.core.example.u2",
            "GetData()",
        )],
    );

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains(
            "[Interface] Component-connected units \"u1\" and \"u2\" have no corresponding function-call in the sequence diagram."
        )
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains(
            "[Interface] Sequence-connected units \"example.u1\" and \"example.u2\" have no corresponding shared interface connection in the component diagram."
        )
    }));
}
