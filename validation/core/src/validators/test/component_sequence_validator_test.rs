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
    ComponentDiagramElementType, ComponentDiagramInput, ComponentDiagramInputs,
    ComponentDiagramRelation, SequenceDiagramInput, SequenceDiagramInputs,
};
use class_diagram::{ClassDiagram, EntityType, Method, SimpleEntity, Visibility};
use sequence_logic::{Event, Interaction, SequenceNode, SequenceTree};

fn relation_with_role(target: &str, source_role: &str) -> ComponentDiagramRelation {
    ComponentDiagramRelation {
        target: target.to_string(),
        annotation: None,
        relation_type: Some("InterfaceBinding".to_string()),
        source_role: Some(source_role.to_string()),
    }
}

fn required_relation(target: &str) -> ComponentDiagramRelation {
    relation_with_role(target, "Required")
}

fn provided_relation(target: &str) -> ComponentDiagramRelation {
    relation_with_role(target, "Provided")
}

fn unit(alias: &str, interface_targets: &[&str]) -> ComponentDiagramInput {
    unit_with_interface_roles(alias, interface_targets, interface_targets)
}

fn unit_with_interface_roles(
    alias: &str,
    required_interfaces: &[&str],
    provided_interfaces: &[&str],
) -> ComponentDiagramInput {
    let mut relations = Vec::new();
    for target in required_interfaces {
        relations.push(required_relation(target));
    }
    for target in provided_interfaces {
        relations.push(provided_relation(target));
    }

    unit_with_relations(alias, relations)
}

fn unit_with_relations(
    alias: &str,
    relations: Vec<ComponentDiagramRelation>,
) -> ComponentDiagramInput {
    ComponentDiagramInput {
        id: format!("some_id.{alias}"),
        alias: Some(alias.to_string()),
        parent_id: None,
        element_type: ComponentDiagramElementType::Component,
        stereotype: Some("unit".to_string()),
        relations,
    }
}

fn interface(alias: &str) -> ComponentDiagramInput {
    ComponentDiagramInput {
        id: alias.to_string(),
        alias: Some(alias.to_string()),
        parent_id: None,
        element_type: ComponentDiagramElementType::Interface,
        stereotype: None,
        relations: Vec::new(),
    }
}

fn interface_with_id(id: &str, alias: &str) -> ComponentDiagramInput {
    ComponentDiagramInput {
        id: id.to_string(),
        alias: Some(alias.to_string()),
        parent_id: None,
        element_type: ComponentDiagramElementType::Interface,
        stereotype: None,
        relations: Vec::new(),
    }
}

fn component_diagrams(aliases: &[&str]) -> ComponentDiagramInputs {
    ComponentDiagramInputs {
        entities: aliases.iter().map(|alias| unit(alias, &[])).collect(),
    }
}

fn component_diagrams_with_entities(
    entities: Vec<ComponentDiagramInput>,
) -> ComponentDiagramInputs {
    ComponentDiagramInputs { entities }
}

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

fn sequence_diagrams(participants: &[&str]) -> SequenceDiagramInputs {
    sequence_calls(
        &participants
            .iter()
            .map(|participant| (*participant, *participant, ""))
            .collect::<Vec<_>>(),
    )
}

fn sequence_calls(calls: &[(&str, &str, &str)]) -> SequenceDiagramInputs {
    SequenceDiagramInputs {
        diagrams: vec![SequenceDiagramInput {
            tree: SequenceTree {
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
            },
            source_files: Vec::new(),
            version: None,
        }],
    }
}

#[test]
fn passes_when_aliases_and_participants_are_identical() {
    let component_diagrams = component_diagrams(&["unit_1", "unit_2"]);
    let sequence_diagrams = sequence_diagrams(&["unit_1", "unit_2"]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);
    assert!(errors.is_empty());
}

#[test]
fn reports_missing_and_extra() {
    let component_diagrams = component_diagrams(&["unit_1", "unit_2", "unit_3"]);
    let sequence_diagrams = sequence_diagrams(&["unit_2", "unit_4"]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);

    assert!(!errors.is_empty());
    assert_eq!(errors.messages.len(), 3);

    let missing_count = errors
        .messages
        .iter()
        .filter(|msg| msg.contains("unit alias not found in sequence participants"))
        .count();
    let unexpected_count = errors
        .messages
        .iter()
        .filter(|msg| msg.contains("sequence participant not found in component unit aliases"))
        .count();

    assert_eq!(missing_count, 2);
    assert_eq!(unexpected_count, 1);
}

#[test]
fn units_without_alias_are_ignored() {
    let component_diagrams = ComponentDiagramInputs {
        entities: vec![ComponentDiagramInput {
            id: "module_a.unit_1".to_string(),
            alias: None,
            parent_id: None,
            element_type: ComponentDiagramElementType::Component,
            stereotype: Some("unit".to_string()),
            relations: Vec::new(),
        }],
    };
    let sequence_diagrams = sequence_diagrams(&[]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);
    assert!(errors.is_empty());
}

#[test]
fn reports_alias_missing_from_participants() {
    let component_diagrams = component_diagrams(&["u1", "u2"]);
    let sequence_diagrams = sequence_diagrams(&["u1"]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);
    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("\"u2\""));
}

#[test]
fn reports_participant_not_in_aliases() {
    let component_diagrams = component_diagrams(&["u1"]);
    let sequence_diagrams = sequence_diagrams(&["u1", "orphan"]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);
    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("\"orphan\""));
}

#[test]
fn reports_missing_component_alias_and_interface_connection_for_sequence_call() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "orphan", "GetData()")]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence participant not found in component unit aliases")
            && message.contains("\"orphan\"")
    }));
    assert!(errors.messages.iter().any(|message| {
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

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0]
        .contains("interface-connected units are missing a sequence function-call connection"));
    assert!(errors.messages[0].contains("\"InternalInterface\""));
}

#[test]
fn reports_missing_participant_and_missing_sequence_call_for_interface_connected_units() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["u1"]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("component unit alias not found in sequence participants")
            && message.contains("\"u2\"")
    }));
    assert!(errors.messages.iter().any(|message| {
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

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0]
        .contains("sequence-connected units have no corresponding shared interface connection"));
    assert!(errors.messages[0].contains("\"CallerInterface\""));
    assert!(errors.messages[0].contains("\"CalleeInterface\""));
}

#[test]
fn passes_when_interface_connected_units_have_sequence_call() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);
    assert!(errors.is_empty());
}

#[test]
fn passes_when_sequence_call_matches_consumer_provider_roles() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit_with_interface_roles("u1", &["InternalInterface"], &[]),
        unit_with_interface_roles("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);
    assert!(errors.is_empty());
}

#[test]
fn reports_cross_unit_sequence_call_with_invalid_consumer_provider_roles() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit_with_interface_roles("u1", &[], &["InternalInterface"]),
        unit_with_interface_roles("u2", &["InternalInterface"], &[]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);

    assert_eq!(errors.messages.len(), 1);
    assert!(
        errors.messages[0].contains("sequence interaction does not match consumer/provider roles")
    );
    assert!(errors.messages[0].contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData()\""));
    assert!(errors.messages[0].contains("Caller consumes     : <none>"));
    assert!(errors.messages[0].contains("Callee provides     : <none>"));
}

#[test]
fn reports_self_call_without_bidirectional_interface_roles() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit_with_interface_roles("u1", &["InternalInterface"], &[]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(&component_arch, &sequence_index, None, errors);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains(
        "self-call unit does not act as both consumer and provider of a referenced interface"
    ));
    assert!(errors.messages[0].contains("Sequence call       : \"u1\" -> \"u1\" : \"GetData()\""));
    assert!(errors.messages[0].contains("Unit requires       : \"InternalInterface\""));
    assert!(errors.messages[0].contains("Unit provides       : <none>"));
}

#[test]
fn reports_sequence_function_missing_from_related_interface_methods() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["OtherMethod"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData\"")
            && message.contains("Shared interfaces   : \"InternalInterface\"")
            && message.contains("\"GetData\"")
    }));
    assert!(errors.messages.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"InternalInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
}

#[test]
fn reports_self_call_function_missing_from_unit_interfaces() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["OtherMethod"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("Sequence call       : \"u1\" -> \"u1\" : \"GetData\"")
            && message.contains("Unit interfaces     : \"InternalInterface\"")
            && message.contains("Method matches      : <none>")
    }));
    assert!(errors.messages.iter().any(|message| {
        message.contains("internal API interface functions are not exercised in sequence diagrams")
            && message.contains("\"InternalInterface\"")
            && message.contains("\"OtherMethod\"")
    }));
}

#[test]
fn reports_interface_function_not_exercised_in_sequence_diagrams() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData", "SetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0]
        .contains("internal API interface functions are not exercised in sequence diagrams"));
    assert!(errors.messages[0].contains("\"InternalInterface\""));
    assert!(errors.messages[0].contains("\"SetData\""));
}

#[test]
fn reports_missing_internal_api_interface_for_related_interfaces() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("OtherInterface", vec!["GetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("Missing internal API interface")
            && message.contains("Unit                : \"u1\"")
            && message.contains("Missing interfaces  : \"InternalInterface\"")
    }));
    assert!(errors.messages.iter().any(|message| {
        message.contains("Missing internal API interface")
            && message.contains("Unit                : \"u2\"")
            && message.contains("Missing interfaces  : \"InternalInterface\"")
    }));
}

#[test]
fn reports_missing_internal_api_interface_for_caller_only() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface", "InternalInterface1"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
        interface("InternalInterface1"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("Missing internal API interface"));
    assert!(errors.messages[0].contains("Unit                : \"u1\""));
    assert!(errors.messages[0].contains("Missing interfaces  : \"InternalInterface1\""));
}

#[test]
fn reports_missing_internal_api_interface_for_callee_only() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface", "InternalInterface1"]),
        interface("InternalInterface"),
        interface("InternalInterface1"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("Missing internal API interface"));
    assert!(errors.messages[0].contains("Unit                : \"u2\""));
    assert!(errors.messages[0].contains("Missing interfaces  : \"InternalInterface1\""));
}

#[test]
fn reports_missing_internal_api_interface_for_unit_only_once_across_call_roles() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface", "InternalInterface1"]),
        unit("u2", &["InternalInterface"]),
        unit("u3", &["InternalInterface"]),
        interface("InternalInterface"),
        interface("InternalInterface1"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()"), ("u3", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    let missing_internal_api_errors: Vec<&String> = errors
        .messages
        .iter()
        .filter(|message| message.contains("Missing internal API interface"))
        .collect();

    assert_eq!(missing_internal_api_errors.len(), 1);
    assert!(missing_internal_api_errors[0].contains("Unit                : \"u1\""));
    assert!(missing_internal_api_errors[0].contains("Missing interfaces  : \"InternalInterface1\""));
}

#[test]
fn reports_missing_internal_api_interface_without_sequence_method_call() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["u1"]);
    let internal_api = internal_api_index(vec![("OtherInterface", vec!["GetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    let missing_internal_api_errors: Vec<&String> = errors
        .messages
        .iter()
        .filter(|message| message.contains("Missing internal API interface"))
        .collect();

    assert_eq!(missing_internal_api_errors.len(), 1);
    assert!(missing_internal_api_errors[0].contains("Unit                : \"u1\""));
    assert!(missing_internal_api_errors[0].contains("Missing interfaces  : \"InternalInterface\""));
}

#[test]
fn reports_missing_component_alias_for_sequence_method_validation() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "orphan", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence participant not found in component unit aliases")
            && message.contains("\"orphan\"")
    }));
    assert!(errors.messages.iter().any(|message| {
        message
            .contains("sequence-connected units have no corresponding shared interface connection")
            && message.contains("\"u1\"")
            && message.contains("\"orphan\"")
    }));
}

#[test]
fn passes_when_sequence_function_exists_on_related_interface() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert!(errors.is_empty());
}

#[test]
fn passes_when_self_call_function_exists_on_unit_interface() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u1", "GetData()")]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

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

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert!(errors.is_empty());
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

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("\"CallerOnlyInterface\"")
            && message.contains("<none>")
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

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("\"CalleeOnlyInterface\"")
            && message.contains("<none>")
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

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("sequence function name was not found in the related interface methods")
            && message.contains("\"CallerOnlyInterface\"")
            && message.contains("\"CalleeOnlyInterface\"")
            && message.contains("\"SharedInterface\"")
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
fn reports_case_mismatch_between_component_and_internal_api_interface_names() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("internalinterface", vec!["GetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert_eq!(errors.messages.len(), 2);
    assert!(errors.messages.iter().any(|message| {
        message.contains("Missing internal API interface")
            && message.contains("Unit                : \"u1\"")
            && message.contains("Missing interfaces  : \"InternalInterface\"")
    }));
    assert!(errors.messages.iter().any(|message| {
        message.contains("Missing internal API interface")
            && message.contains("Unit                : \"u2\"")
            && message.contains("Missing interfaces  : \"InternalInterface\"")
    }));
}

#[test]
fn matches_internal_api_by_component_interface_id_when_alias_differs() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["pkg.InternalInterface"]),
        unit("u2", &["pkg.InternalInterface"]),
        interface_with_id("pkg.InternalInterface", "InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);
    let internal_api = internal_api_index(vec![("pkg.InternalInterface", vec!["GetData"])]);

    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut errors);

    let errors = validate_component_sequence(
        &component_arch,
        &sequence_index,
        Some(&internal_api),
        errors,
    );

    assert!(errors.is_empty());
}
