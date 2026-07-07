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

use crate::models::{
    ComponentDiagramInputs, ComponentRelationType, ComponentType, EndpointRole, InternalApiIndex,
    LogicComponent, LogicRelation, SequenceDiagramInputs,
};
use crate::ValidationResult;
use class_diagram::{ClassDiagram, EntityType, Method, SimpleEntity, Visibility};
use sequence_logic::{Event, Interaction, SequenceNode, SequenceTree};

pub(super) fn relation_with_role(target: &str, source_role: EndpointRole) -> LogicRelation {
    relation_with_type_and_role(target, ComponentRelationType::InterfaceBinding, source_role)
}

pub(super) fn relation_with_type_and_role(
    target: &str,
    relation_type: ComponentRelationType,
    source_role: EndpointRole,
) -> LogicRelation {
    LogicRelation {
        target: target.to_string(),
        annotation: None,
        relation_type,
        source_role,
    }
}

pub(super) fn unit(alias: &str, interface_targets: &[&str]) -> LogicComponent {
    unit_with_interface_roles(alias, interface_targets, interface_targets)
}

pub(super) fn unit_with_interface_roles(
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

pub(super) fn interface(alias: &str) -> LogicComponent {
    interface_with_id(alias, alias)
}

pub(super) fn interface_with_id(id: &str, alias: &str) -> LogicComponent {
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

pub(super) fn component_diagrams(aliases: &[&str]) -> ComponentDiagramInputs {
    ComponentDiagramInputs {
        entities: aliases.iter().map(|alias| unit(alias, &[])).collect(),
    }
}

pub(super) fn component_diagrams_with_entities(
    entities: Vec<LogicComponent>,
) -> ComponentDiagramInputs {
    ComponentDiagramInputs { entities }
}

pub(super) fn sequence_diagrams(participants: &[&str]) -> SequenceDiagramInputs {
    sequence_calls(
        &participants
            .iter()
            .map(|participant| (*participant, *participant, ""))
            .collect::<Vec<_>>(),
    )
}

pub(super) fn sequence_calls(calls: &[(&str, &str, &str)]) -> SequenceDiagramInputs {
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

pub(super) fn internal_api_index(interfaces: Vec<(&str, Vec<&str>)>) -> InternalApiIndex {
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

    let mut setup_result = ValidationResult::default();
    let index = InternalApiIndex::build_index(&diagrams, &mut setup_result);
    assert!(
        setup_result.is_empty(),
        "test fixture construction failed: {:?}",
        setup_result.failures
    );
    index
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
