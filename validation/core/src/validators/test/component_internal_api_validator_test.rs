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
    ComponentDiagramInputs, ComponentRelationType, ComponentType, EndpointRole, LogicComponent,
    LogicRelation,
};
use class_diagram::{ClassDiagram, EntityType, Method, SimpleEntity, Visibility};

fn relation_with_role(target: &str, source_role: EndpointRole) -> LogicRelation {
    LogicRelation {
        target: target.to_string(),
        annotation: None,
        relation_type: ComponentRelationType::InterfaceBinding,
        source_role,
    }
}

fn unit(alias: &str, interface_targets: &[&str]) -> LogicComponent {
    let mut relations = Vec::new();
    for target in interface_targets {
        relations.push(relation_with_role(target, EndpointRole::Required));
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

fn interface_with_id(id: &str, alias: &str) -> LogicComponent {
    LogicComponent {
        id: id.to_string(),
        name: Some(alias.to_string()),
        alias: Some(alias.to_string()),
        parent_id: None,
        element_type: ComponentType::Interface,
        stereotype: None,
        relations: Vec::new(),
    }
}

fn component_diagrams_with_entities(entities: Vec<LogicComponent>) -> ComponentDiagramInputs {
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

fn validate(component_diagrams: ComponentDiagramInputs, internal_api: &InternalApiIndex) -> Errors {
    let mut errors = Errors::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut errors);

    validate_component_internal_api(&component_arch, internal_api, errors)
}

#[test]
fn reports_missing_component_interface_declared_by_internal_api() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let internal_api = internal_api_index(vec![("OtherInterface", vec!["GetData"])]);

    let errors = validate(component_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("Missing internal API interface"));
    assert!(errors.messages[0].contains("Missing interfaces  : \"InternalInterface\""));
    assert!(!errors.messages[0].contains("Unit                :"));
}

#[test]
fn reports_each_missing_component_interface_once() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface", "InternalInterface1"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
        interface("InternalInterface1"),
    ]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let errors = validate(component_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("Missing internal API interface"));
    assert!(errors.messages[0].contains("Missing interfaces  : \"InternalInterface1\""));
}

#[test]
fn reports_missing_component_interface_even_without_unit_relation() {
    let component_diagrams =
        component_diagrams_with_entities(vec![unit("u1", &[]), interface("UnusedInterface")]);
    let internal_api = internal_api_index(vec![]);

    let errors = validate(component_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("Missing internal API interface"));
    assert!(errors.messages[0].contains("Missing interfaces  : \"UnusedInterface\""));
}

#[test]
fn reports_all_missing_component_interfaces_in_one_message() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface", "InternalInterface1"]),
        unit("u2", &["InternalInterface"]),
        unit("u3", &["InternalInterface"]),
        interface("InternalInterface"),
        interface("InternalInterface1"),
    ]);
    let internal_api = internal_api_index(vec![("InternalInterface", vec!["GetData"])]);

    let errors = validate(component_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("Missing internal API interface"));
    assert!(errors.messages[0].contains("Missing interfaces  : \"InternalInterface1\""));
}

#[test]
fn reports_missing_component_interface_without_sequence_method_call() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let internal_api = internal_api_index(vec![("OtherInterface", vec!["GetData"])]);

    let errors = validate(component_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("Missing internal API interface"));
    assert!(errors.messages[0].contains("Missing interfaces  : \"InternalInterface\""));
}

#[test]
fn reports_case_mismatch_between_component_and_internal_api_interface_names() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["InternalInterface"]),
        unit("u2", &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let internal_api = internal_api_index(vec![("internalinterface", vec!["GetData"])]);

    let errors = validate(component_diagrams, &internal_api);

    assert_eq!(errors.messages.len(), 1);
    assert!(errors.messages[0].contains("Missing internal API interface"));
    assert!(errors.messages[0].contains("Missing interfaces  : \"InternalInterface\""));
}

#[test]
fn matches_internal_api_by_component_interface_id_when_alias_differs() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit("u1", &["pkg.InternalInterface"]),
        unit("u2", &["pkg.InternalInterface"]),
        interface_with_id("pkg.InternalInterface", "InternalInterface"),
    ]);
    let internal_api = internal_api_index(vec![("pkg.InternalInterface", vec!["GetData"])]);

    let errors = validate(component_diagrams, &internal_api);

    assert!(errors.is_empty());
}
