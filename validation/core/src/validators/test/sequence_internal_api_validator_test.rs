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
    ComponentDiagramInputs, ComponentRelationType, EndpointRole, LogicComponent, PublicApiIndex,
    SequenceDiagramInputs,
};
use crate::ValidationResult;
use sequence_logic::MessageKind;

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

    validate_sequence_internal_api(&sequence_index, internal_api, None, None)
}

fn validate_with_public_api(
    component_diagrams: ComponentDiagramInputs,
    sequence_diagrams: SequenceDiagramInputs,
    internal_api: &InternalApiIndex,
    public_api: &PublicApiIndex,
) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(
        setup_result.is_empty(),
        "test fixture setup failed: {:?}",
        setup_result.failures
    );

    validate_sequence_internal_api(
        &sequence_index,
        internal_api,
        Some(&component_arch),
        Some(public_api),
    )
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

    validate_sequence_internal_api(&sequence_index, internal_api, Some(&component_arch), None)
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
    assert!(validation_result.failures[0].contains(
        "Methods \"OtherMethod\" declared on internal API interface \"InternalInterface\" in the internal API diagram are not exercised in the sequence diagram."
    ));
    assert!(validation_result.failures[0].contains("\"InternalInterface\""));
    assert!(validation_result.failures[0].contains("\"OtherMethod\""));
    assert!(validation_result
        .failures
        .iter()
        .all(|message| !message.contains("[Method]")));
}

#[test]
fn reports_internal_api_interface_function_not_exercised() {
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData", "SetData"])]);

    let validation_result = validate(sequence_diagrams, &internal_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains(
        "Methods \"SetData\" declared on internal API interface \"InternalInterface\" in the internal API diagram are not exercised in the sequence diagram."
    ));
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
        message.contains(
            "Sequence function \"GetData\" from sequence call \"u1\" -> \"u2\" : \"GetData\" in the sequence diagram not found in the internal API diagram."
        ) && message.contains("sequence function name was not found in available interface methods")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains(
            "Methods \"OtherMethod\" declared on internal API interface \"InternalInterface\" in the internal API diagram are not exercised in the sequence diagram."
        )
            && message.contains("\"InternalInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
}

#[test]
fn resolves_leaf_participants_to_nested_units() {
    let mut caller = unit_with_parent_id("u1", "comp_a");
    caller.relations = vec![relation(
        "InternalInterface",
        ComponentRelationType::InterfaceBinding,
        EndpointRole::Required,
    )];
    let mut callee = unit_with_parent_id("u2", "comp_a");
    callee.relations = vec![relation(
        "InternalInterface",
        ComponentRelationType::InterfaceBinding,
        EndpointRole::Provided,
    )];
    let component_diagrams = component_diagram(vec![
        component("comp_a"),
        caller,
        callee,
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let validation_result =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
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
        message.contains(
            "Sequence function \"GetData\" from sequence call \"u1\" -> \"u2\" : \"GetData\" in the sequence diagram not found in the internal API diagram."
        ) && message.contains("sequence function name was not found in available interface methods")
    }));
    assert!(validation_result
        .failures
        .iter()
        .all(|message| !message.contains("required/provided interface roles")));
    assert!(validation_result.failures.iter().all(|message| {
        !message.contains(
            "Detail       : sequence function name was not found in the related interface methods",
        )
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
    assert!(validation_result.failures[0].contains(
        "Methods \"SetData\" declared on internal API interface \"InternalInterface\" in the internal API diagram are not exercised in the sequence diagram."
    ));
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
    assert!(validation_result.failures[0].contains(
        "Methods \"SetData\" declared on internal API interface \"OtherInterface\" in the internal API diagram are not exercised in the sequence diagram."
    ));
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
        message.contains(
            "Sequence function \"GetData\" from sequence call \"u1\" -> \"u1\" : \"GetData\" in the sequence diagram not found in the internal API diagram."
        ) && message.contains("\"u1\" -> \"u1\" : \"GetData\"")
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
        message.contains(
            "Sequence function \"GetData\" from sequence call \"u1\" -> \"u1\" : \"GetData\" in the sequence diagram not found in the internal API diagram."
        ) && message.contains("\"u1\" -> \"u1\" : \"GetData\"")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains(
            "Methods \"OtherMethod\" declared on internal API interface \"InternalInterface\" in the internal API diagram are not exercised in the sequence diagram."
        )
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
    assert!(validation_result.failures[0].contains(
        "Sequence function \"GetData\" from sequence call \"u1\" -> \"u1\" : \"GetData\" in the sequence diagram not found in the internal API diagram."
    ));
    assert!(validation_result.failures[0].contains("\"u1\" -> \"u1\" : \"GetData\""));
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
        message.contains(
            "Sequence function \"GetData\" from sequence call \"u1\" -> \"u2\" : \"GetData\" in the sequence diagram not found in the internal API diagram."
        ) && message.contains("sequence function name was not found in the related interface methods")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains(
            "Methods \"OtherMethod\" declared on internal API interface \"SharedInterface\" in the internal API diagram are not exercised in the sequence diagram."
        )
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
        message.contains(
            "Sequence function \"GetData\" from sequence call \"u1\" -> \"u2\" : \"GetData\" in the sequence diagram not found in the internal API diagram."
        ) && message.contains("sequence function name was not found in the related interface methods")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains(
            "Methods \"OtherMethod\" declared on internal API interface \"SharedInterface\" in the internal API diagram are not exercised in the sequence diagram."
        )
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
        message.contains(
            "Sequence function \"GetData\" from sequence call \"u1\" -> \"u2\" : \"GetData\" in the sequence diagram not found in the internal API diagram."
        ) && message.contains("sequence function name was not found in the related interface methods")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains(
            "Methods \"OtherMethod\" declared on internal API interface \"SharedInterface\" in the internal API diagram are not exercised in the sequence diagram."
        )
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
    assert!(validation_result.failures[0].contains(
        "Sequence call \"u1\" -> \"u2\" : \"GetData\" in the sequence diagram does not match the required/provided interface roles in the component diagram."
    ));
    assert!(validation_result.failures[0].contains("\"u1\" -> \"u2\" : \"GetData\""));
    assert!(validation_result.failures[0]
        .contains("\"u1\" should require shared interface(s) \"InternalInterface\""));
    assert!(validation_result.failures[0]
        .contains("\"u2\" should provide shared interface(s) \"InternalInterface\""));
    assert!(!validation_result.failures[0].contains("[Method]"));
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

fn actor_sequence(
    calls: &[(Option<&str>, Option<&str>, &str, MessageKind)],
) -> SequenceDiagramInputs {
    sequence_of(
        vec![
            actor_participant("app"),
            external_participant("platform"),
            element_participant("u1"),
        ],
        calls,
    )
}

fn validate_actor_calls(
    calls: &[(Option<&str>, Option<&str>, &str, MessageKind)],
    internal_api: &InternalApiIndex,
) -> ValidationResult {
    validate_with_public_api(
        component_diagram(vec![unit_without_interfaces("u1")]),
        actor_sequence(calls),
        internal_api,
        &public_api_index(vec![("Service", vec!["Run"]), ("Callback", vec![])]),
    )
}

#[test]
fn actor_and_external_messages_are_not_checked_for_method_names() {
    let validation_result = validate_actor_calls(
        &[
            (Some("app"), Some("u1"), "Unknown()", MessageKind::SynchCall),
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
        ],
        &internal_api_index(vec![]),
    );

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

fn validate_gate_calls(
    calls: &[(Option<&str>, Option<&str>, &str, MessageKind)],
    internal_api: &InternalApiIndex,
    public_api: Option<&PublicApiIndex>,
) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagram(vec![
        unit("u1", &[], &["Iface"]),
        component("comp"),
        unit_with_parent_id("inner", "comp"),
        interface("Iface"),
    ])
    .to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_of(
        vec![element_participant("u1"), element_participant("comp")],
        calls,
    )
    .to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty(), "{:?}", setup_result.failures);

    validate_sequence_internal_api(
        &sequence_index,
        internal_api,
        Some(&component_arch),
        public_api,
    )
}

#[test]
fn gate_message_to_declared_interface_method_passes() {
    let validation_result = validate_gate_calls(
        &[(None, Some("u1"), "Get()", MessageKind::SynchCall)],
        &internal_api_index(vec![("Iface", vec!["Get"])]),
        None,
    );

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

/// `u1` provides the public interface `Service` and `u2` provides `Other`.
fn validate_gate_call_to_public_interface(method: &str) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagram(vec![
        unit("u1", &[], &["Service"]),
        unit("u2", &[], &["Other"]),
        interface("Service"),
        interface("Other"),
    ])
    .to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_of(
        vec![element_participant("u1")],
        &[(None, Some("u1"), method, MessageKind::SynchCall)],
    )
    .to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty(), "{:?}", setup_result.failures);

    validate_sequence_internal_api(
        &sequence_index,
        &internal_api_index(vec![]),
        Some(&component_arch),
        Some(&public_api_index(vec![
            ("Service", vec!["Run"]),
            ("Other", vec!["Stop"]),
        ])),
    )
}

#[test]
fn gate_message_to_method_of_provided_public_interface_passes() {
    let validation_result = validate_gate_call_to_public_interface("Run()");

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

#[test]
fn reports_gate_message_to_method_of_interface_the_callee_does_not_provide() {
    let validation_result = validate_gate_call_to_public_interface("Stop()");

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("is not declared on an interface provided by the callee"));
}

#[test]
fn reports_gate_message_with_undeclared_method_and_does_not_count_it_as_coverage() {
    let validation_result = validate_gate_calls(
        &[(None, Some("u1"), "Gett()", MessageKind::SynchCall)],
        &internal_api_index(vec![("Iface", vec!["Get"])]),
        None,
    );

    assert_eq!(validation_result.failures.len(), 2);
    let gate_failure = validation_result
        .failures
        .iter()
        .find(|message| message.contains("gate message"))
        .expect("gate message failure");
    assert!(gate_failure.contains("\"<gate>\" -> \"u1\" : \"Gett\""));
    assert!(gate_failure.contains("Did you mean method \"Get\"?"));
    assert!(validation_result
        .failures
        .iter()
        .any(|message| message.contains("not exercised in the sequence diagram")));
}

#[test]
fn gate_message_to_component_lifeline_uses_interfaces_of_contained_units() {
    let mut inner = unit_with_parent_id("inner", "comp");
    inner.relations = vec![relation(
        "Iface",
        ComponentRelationType::InterfaceBinding,
        EndpointRole::Provided,
    )];

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagram(vec![component("comp"), inner, interface("Iface")])
        .to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_of(
        vec![element_participant("comp")],
        &[(None, Some("comp"), "Get()", MessageKind::SynchCall)],
    )
    .to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty(), "{:?}", setup_result.failures);

    let validation_result = validate_sequence_internal_api(
        &sequence_index,
        &internal_api_index(vec![("Iface", vec!["Get"])]),
        Some(&component_arch),
        None,
    );

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

#[test]
fn message_to_gate_is_not_checked_and_does_not_count_as_coverage() {
    let validation_result = validate_gate_calls(
        &[(Some("u1"), None, "Emit()", MessageKind::SynchCall)],
        &internal_api_index(vec![("Iface", vec!["Emit"])]),
        None,
    );

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("not exercised in the sequence diagram"));
}

#[test]
fn replies_are_not_checked_as_calls() {
    let validation_result = validate_actor_calls(
        &[
            (Some("app"), Some("u1"), "Run()", MessageKind::SynchCall),
            (Some("u1"), Some("app"), "result", MessageKind::Reply),
        ],
        &internal_api_index(vec![]),
    );

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

#[test]
fn actor_calls_do_not_exercise_internal_api_operations() {
    let validation_result = validate_actor_calls(
        &[(Some("app"), Some("u1"), "Run()", MessageKind::SynchCall)],
        &internal_api_index(vec![("InternalInterface", vec!["Run"])]),
    );

    assert!(validation_result
        .failures
        .iter()
        .any(|message| message.contains("not exercised in the sequence diagram")));
}

/// `caller` requires `Iface` and `Other`; `comp.inner` provides `Iface` and
/// `provider` provides `Other`.
fn validate_call_to_component(method: &str) -> ValidationResult {
    let caller = unit("caller", &["Iface", "Other"], &[]);
    let mut inner = unit_with_parent_id("inner", "comp");
    inner.relations = vec![relation(
        "Iface",
        ComponentRelationType::InterfaceBinding,
        EndpointRole::Provided,
    )];

    let component_diagrams = component_diagram(vec![
        caller,
        component("comp"),
        inner,
        unit("provider", &[], &["Other"]),
        interface("Iface"),
        interface("Other"),
    ]);
    let sequence_diagrams = sequence_of(
        vec![element_participant("caller"), element_participant("comp")],
        &[(Some("caller"), Some("comp"), method, MessageKind::SynchCall)],
    );

    validate_with_component_context(
        component_diagrams,
        sequence_diagrams,
        &internal_api_index(vec![("Iface", vec!["Get"]), ("Other", vec!["Put"])]),
    )
}

#[test]
fn component_lifeline_call_to_method_of_shared_interface_passes() {
    let validation_result = validate_call_to_component("Get()");

    assert!(!validation_result
        .failures
        .iter()
        .any(|message| message.contains("related interface methods")));
}

#[test]
fn reports_component_lifeline_call_to_method_outside_shared_interfaces() {
    let validation_result = validate_call_to_component("Put()");

    assert!(validation_result.failures.iter().any(|message| message
        .contains("sequence function name was not found in the related interface methods")));
}

#[test]
fn component_lifeline_accepts_public_api_operation() {
    let component_diagrams = component_diagram(vec![
        component("comp"),
        unit_with_parent_id("u1", "comp"),
        unit_with_parent_id("u2", "comp"),
    ]);
    let sequence_diagrams = sequence_of(
        vec![element_participant("u1"), element_participant("comp")],
        &[(Some("u1"), Some("comp"), "Run()", MessageKind::SynchCall)],
    );

    let validation_result = validate_with_public_api(
        component_diagrams,
        sequence_diagrams,
        &internal_api_index(vec![]),
        &public_api_index(vec![("Service", vec!["Run"])]),
    );

    assert!(
        validation_result.failures.is_empty(),
        "{:?}",
        validation_result.failures
    );
}
