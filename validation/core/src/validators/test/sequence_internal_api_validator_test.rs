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

use super::*;
use crate::models::{
    ComponentDiagramInputs, ComponentRelationType, ComponentType, EndpointRole, Errors,
    LogicComponent, LogicRelation, SequenceDiagramInputs,
};
use class_diagram::{ClassDiagram, EntityType, Method, SimpleEntity, Visibility};
use sequence_logic::{Event, Interaction, SequenceNode, SequenceTree};

fn method(name: &str) -> Method {
    Method {
        name: name.to_string(),
        return_type: None,
        visibility: Visibility::Public,
        parameters: Vec::new(),
        template_parameters: None,
        modifiers: Vec::new(),
    }
}

fn internal_api_index(interfaces: Vec<(&str, Vec<&str>)>) -> InternalApiIndex {
    let diagrams = vec![ClassDiagram {
        name: "internal_api".to_string(),
        entities: interfaces
            .into_iter()
            .map(|(interface_name, methods)| SimpleEntity {
                id: interface_name.to_string(),
                name: interface_name.to_string(),
                enclosing_namespace_id: None,
                entity_type: EntityType::Interface,
                type_aliases: Vec::new(),
                variables: Vec::new(),
                methods: methods.into_iter().map(method).collect(),
                template_parameters: None,
                enum_literals: Vec::new(),
                relationships: Vec::new(),
                source_file: None,
                source_line: None,
            })
            .collect(),
        relationships: Vec::new(),
        source_files: Vec::new(),
        version: None,
    }];

    let mut errors = Errors::default();
    let index = InternalApiIndex::build_index(&diagrams, &mut errors);
    assert!(errors.is_empty());
    index
}

fn relation_with_role(target: &str, source_role: EndpointRole) -> LogicRelation {
    LogicRelation {
        target: target.to_string(),
        annotation: None,
        relation_type: ComponentRelationType::InterfaceBinding,
        source_role,
    }
}

fn unit(alias: &str, interface_targets: &[&str]) -> LogicComponent {
    unit_with_interface_roles(alias, interface_targets, interface_targets)
}

fn unit_with_interface_roles(
    alias: &str,
    required_interfaces: &[&str],
    provided_interfaces: &[&str],
) -> LogicComponent {
    let mut relations = Vec::new();
    for target in required_interfaces {
        relations.push(relation_with_role(target, EndpointRole::Required));
    }
    for target in provided_interfaces {
        relations.push(relation_with_role(target, EndpointRole::Provided));
    }

    LogicComponent {
        id: format!("some_id.{alias}"),
        name: Some(alias.to_string()),
        alias: Some(alias.to_string()),
        parent_id: None,
        element_type: ComponentType::Component,
        stereotype: Some("unit".to_string()),
        relations,
    }
}

fn interface(alias: &str) -> LogicComponent {
    LogicComponent {
        id: alias.to_string(),
        name: Some(alias.to_string()),
        alias: Some(alias.to_string()),
        parent_id: None,
        element_type: ComponentType::Interface,
        stereotype: None,
        relations: Vec::new(),
    }
}

fn component_diagrams(aliases: &[&str]) -> ComponentDiagramInputs {
    ComponentDiagramInputs {
        entities: aliases.iter().map(|alias| unit(alias, &[])).collect(),
    }
}

fn component_diagrams_with_entities(entities: Vec<LogicComponent>) -> ComponentDiagramInputs {
    ComponentDiagramInputs { entities }
}

fn sequence_calls(calls: &[(&str, &str, &str)]) -> SequenceDiagramInputs {
    SequenceDiagramInputs {
        diagrams: vec![SequenceTree {
            name: Some("seq".to_string()),
            root_interactions: calls
                .iter()
                .map(|(caller, callee, method)| SequenceNode {
                    event: Event::Interaction(Interaction {
                        caller: (*caller).to_string(),
                        callee: (*callee).to_string(),
                        method: (*method).to_string(),
                    }),
                    branches_node: Vec::new(),
                })
                .collect(),
        }],
    }
}

fn validate(sequence_diagrams: SequenceDiagramInputs, internal_api: &InternalApiIndex) -> Errors {
    let mut errors = Errors::default();
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    validate_sequence_internal_api(&sequence_index, internal_api, None, errors)
}

fn validate_with_component_context(
    component_diagrams: ComponentDiagramInputs,
    sequence_diagrams: SequenceDiagramInputs,
    internal_api: &InternalApiIndex,
) -> Errors {
    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    validate_sequence_internal_api(&sequence_index, internal_api, Some(&component_arch), errors)
}

#[test]
fn does_not_check_sequence_method_names_without_component_context() {
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec![])]);

    let errors = validate(sequence_diagrams, &internal_api);

    assert!(errors.is_empty());
}

#[test]
fn reports_internal_api_interface_function_not_exercised_without_method_name_check() {
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["OtherMethod"])]);

    let errors = validate(sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0]
        .contains("internal API interface functions are not exercised in sequence diagrams"));
    assert!(errors.messages[0].contains("\"InternalInterface\""));
    assert!(errors.messages[0].contains("\"OtherMethod\""));
    assert!(errors
        .messages
        .iter()
        .all(|message| !message.contains("Method consistency violation")));
}

#[test]
fn reports_internal_api_interface_function_not_exercised() {
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData", "SetData"])]);

    let errors = validate(sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0]
        .contains("internal API interface functions are not exercised in sequence diagrams"));
    assert!(errors.messages[0].contains("\"InternalInterface\""));
    assert!(errors.messages[0].contains("\"SetData\""));
}

#[test]
fn self_calls_count_as_internal_api_method_usage() {
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let errors = validate(sequence_diagrams, &internal_api);

    assert!(errors.is_empty());
}

#[test]
fn reports_sequence_function_missing_from_related_interface_methods_with_component_context() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["OtherMethod"])]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\"")
    }));
    assert!(errors.messages.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"InternalInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
}

#[test]
fn reports_interface_function_not_exercised_in_sequence_diagrams_with_component_context() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData", "SetData"])]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0]
        .contains("internal API interface functions are not exercised in sequence diagrams"));
    assert!(errors.messages[0].contains("\"InternalInterface\""));
    assert!(errors.messages[0].contains("\"SetData\""));
}

#[test]
fn reports_unreferenced_internal_api_interface_function_not_exercised_without_self_calls() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![
        ("InternalInterface", vec!["GetData"]),
        ("OtherInterface", vec!["SetData"]),
    ]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0]
        .contains("internal API interface functions are not exercised in sequence diagrams"));
    assert!(errors.messages[0].contains("\"OtherInterface\""));
    assert!(errors.messages[0].contains("\"SetData\""));
}

#[test]
fn reports_self_call_method_mismatch_when_unit_has_missing_internal_api_interface() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["MissingInterface"]),
        interface("MissingInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence self-call function name was not found")
            && message.contains("Sequence call       : \"u1\" -> \"u1\" : \"GetData\"")
    }));
}

#[test]
fn passes_when_sequence_function_exists_on_related_interface_with_component_context() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert!(errors.is_empty());
}

#[test]
fn reports_self_call_function_missing_from_available_interfaces() {
    let component_diagrams = component_diagrams(&["u1"]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["OtherMethod"])]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence self-call function name was not found")
            && message.contains("Sequence call       : \"u1\" -> \"u1\" : \"GetData\"")
    }));
    assert!(errors.messages.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"InternalInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
}

#[test]
fn passes_when_self_call_uses_internal_api_interface_without_component_interfaces() {
    let component_diagrams = component_diagrams(&["u1"]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert!(errors.is_empty());
}

#[test]
fn passes_when_all_interface_functions_are_exercised_by_self_calls() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()"), ("u1", "u1", "SetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData", "SetData"])]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert!(errors.is_empty());
}

#[test]
fn reports_self_call_without_any_available_interfaces() {
    let component_diagrams = component_diagrams(&["u1"]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("sequence self-call function name was not found"));
    assert!(errors.messages[0].contains("Sequence call       : \"u1\" -> \"u1\" : \"GetData\""));
}

#[test]
fn reports_method_declared_only_on_caller_side_interfaces() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["SharedInterface", "CallerOnlyInterface"]),
        unit("u2", &["SharedInterface"]),
        interface("SharedInterface"),
        interface("CallerOnlyInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![
        ("SharedInterface", vec!["OtherMethod"]),
        ("CallerOnlyInterface", vec!["GetData"]),
    ]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\"")
    }));
    assert!(errors.messages.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"SharedInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
    assert!(errors
        .messages
        .iter()
        .all(|message| !message.contains("Missing functions   : \"GetData\"")));
}

#[test]
fn reports_method_declared_only_on_callee_side_interfaces() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["SharedInterface"]),
        unit("u2", &["SharedInterface", "CalleeOnlyInterface"]),
        interface("SharedInterface"),
        interface("CalleeOnlyInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![
        ("SharedInterface", vec!["OtherMethod"]),
        ("CalleeOnlyInterface", vec!["GetData"]),
    ]);

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\"")
    }));
    assert!(errors.messages.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"SharedInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
    assert!(errors
        .messages
        .iter()
        .all(|message| !message.contains("Missing functions   : \"GetData\"")));
}

#[test]
fn reports_method_declared_on_both_sides_but_not_on_shared_interface() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["SharedInterface", "CallerOnlyInterface"]),
        unit("u2", &["SharedInterface", "CalleeOnlyInterface"]),
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

    let errors =
        validate_with_component_context(component_diagrams, sequence_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\"")
    }));
    assert!(errors.messages.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"SharedInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
    assert!(errors
        .messages
        .iter()
        .all(|message| !message.contains("Missing functions   : \"GetData\"")));
}
