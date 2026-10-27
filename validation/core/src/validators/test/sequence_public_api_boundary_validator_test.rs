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
use crate::ValidationResult;
use sequence_logic::MessageKind;

type Call<'a> = (Option<&'a str>, Option<&'a str>, &'a str, MessageKind);

fn validate(calls: &[Call]) -> ValidationResult {
    validate_against(calls, None)
}

/// `u1` provides `Service` and requires `Callback`, `u2` provides `Other`,
/// `u3` binds no interface.
fn validate_scoped(calls: &[Call]) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let component = component_diagram(vec![
        unit("u1", &["Callback"], &["Service"]),
        unit("u2", &[], &["Other"]),
        unit_without_interfaces("u3"),
        interface("Service"),
        interface("Callback"),
        interface("Other"),
    ])
    .to_diagram_architecture(&mut setup_result);
    assert!(setup_result.is_empty(), "{:?}", setup_result.failures);

    validate_against(calls, Some(&component))
}

fn validate_against(
    calls: &[Call],
    component: Option<&ComponentDiagramArchitecture>,
) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let sequence_index = sequence_of(
        vec![
            actor_participant("app"),
            external_participant("platform"),
            element_participant("u1"),
            element_participant("u2"),
            element_participant("u3"),
        ],
        calls,
    )
    .to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty(), "{:?}", setup_result.failures);

    validate_sequence_public_api_boundary(
        &sequence_index,
        &public_api_index(vec![
            ("Service", vec!["Run"]),
            ("Callback", vec![]),
            ("Other", vec!["Stop"]),
        ]),
        component,
    )
}

#[test]
fn actor_call_to_declared_public_api_operation_passes() {
    let validation_result = validate(&[(Some("app"), Some("u1"), "Run()", MessageKind::SynchCall)]);

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

#[test]
fn reports_actor_call_to_operation_missing_in_public_api() {
    let validation_result =
        validate(&[(Some("app"), Some("u1"), "Runn()", MessageKind::SynchCall)]);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("not found in the public API diagram"));
    assert!(validation_result.failures[0].contains("\"app\" -> \"u1\" : \"Runn\""));
    assert!(validation_result.failures[0].contains("Did you mean method \"Run\"?"));
}

#[test]
fn callback_may_be_labelled_with_public_api_interface_name() {
    let validation_result =
        validate(&[(Some("u1"), Some("app"), "Callback", MessageKind::AsynchCall)]);

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

#[test]
fn reports_callback_without_public_api_declaration() {
    let validation_result =
        validate(&[(Some("u1"), Some("app"), "Notify()", MessageKind::SynchCall)]);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("allback \"Notify\""));
}

#[test]
fn callback_suggestion_names_interface_or_method() {
    let validation_result =
        validate(&[(Some("u1"), Some("app"), "Callbak", MessageKind::AsynchCall)]);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(
        validation_result.failures[0].contains("Did you mean interface or method \"Callback\"?")
    );
}

#[test]
fn external_system_and_gate_messages_are_not_checked() {
    let validation_result = validate(&[
        (
            Some("u1"),
            Some("platform"),
            "Send()",
            MessageKind::SynchCall,
        ),
        (
            Some("platform"),
            Some("u1"),
            "Receive()",
            MessageKind::AsynchCall,
        ),
        (None, Some("u1"), "Trigger()", MessageKind::SynchCall),
        (Some("u1"), None, "Emit()", MessageKind::SynchCall),
    ]);

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

#[test]
fn replies_are_not_checked_as_calls() {
    let validation_result = validate(&[
        (Some("app"), Some("u1"), "Run()", MessageKind::SynchCall),
        (Some("u1"), Some("app"), "result", MessageKind::Reply),
    ]);

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

#[test]
fn repeated_calls_are_reported_once() {
    let validation_result = validate(&[
        (Some("app"), Some("u1"), "Runn()", MessageKind::SynchCall),
        (Some("app"), Some("u1"), "Runn()", MessageKind::SynchCall),
    ]);

    assert_eq!(validation_result.failures.len(), 1);
}

#[test]
fn actor_call_is_checked_against_interfaces_the_element_provides() {
    let validation_result = validate_scoped(&[
        (Some("app"), Some("u1"), "Run()", MessageKind::SynchCall),
        (Some("app"), Some("u2"), "Stop()", MessageKind::SynchCall),
    ]);

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

#[test]
fn reports_actor_call_to_operation_of_another_elements_interface() {
    let validation_result =
        validate_scoped(&[(Some("app"), Some("u1"), "Stop()", MessageKind::SynchCall)]);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("not found in the public API diagram"));
    assert!(validation_result.failures[0]
        .contains("\"u1\" provides public API interface(s) \"Service\"; none declares the method"));
}

#[test]
fn callback_is_checked_against_interfaces_the_element_requires() {
    let validation_result = validate_scoped(&[
        (Some("u1"), Some("app"), "Callback", MessageKind::AsynchCall),
        (Some("u1"), Some("app"), "Service", MessageKind::AsynchCall),
    ]);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("allback \"Service\""));
    assert!(validation_result.failures[0].contains(
        "\"u1\" requires public API interface(s) \"Callback\"; the label names none of them or their methods"
    ));
}

#[test]
fn element_without_public_interface_falls_back_to_the_whole_public_api() {
    let validation_result = validate_scoped(&[
        (Some("app"), Some("u3"), "Stop()", MessageKind::SynchCall),
        (Some("app"), Some("u3"), "Missing()", MessageKind::SynchCall),
    ]);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("\"Missing\""));
    assert!(!validation_result.failures[0].contains("provides public API"));
}
