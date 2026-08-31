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
use class_diagram::{ClassDiagram, EntityType, Method, SimpleEntity, Visibility};
use component_diagram::SourceLocation;
use sequence_logic::{
    Block, Interaction, Node, ParticipantType, SequenceParticipant, SequenceTree,
};

// Common fixtures

pub(crate) fn dummy_source_location() -> SourceLocation {
    SourceLocation::new("test.puml", 1)
}

fn entity_id(alias: &str, parent_id: Option<&str>) -> String {
    parent_id
        .map(|parent_id| format!("{parent_id}.{alias}"))
        .unwrap_or_else(|| alias.to_string())
}

// Component diagram fixtures

pub(super) fn component_diagram(entities: Vec<LogicComponent>) -> ComponentDiagramInputs {
    ComponentDiagramInputs { entities }
}

pub(super) fn unit(
    alias: &str,
    required_interfaces: &[&str],
    provided_interfaces: &[&str],
) -> LogicComponent {
    let mut relations = Vec::new();
    for target in required_interfaces {
        relations.push(relation(
            target,
            ComponentRelationType::InterfaceBinding,
            EndpointRole::Required,
        ));
    }
    for target in provided_interfaces {
        relations.push(relation(
            target,
            ComponentRelationType::InterfaceBinding,
            EndpointRole::Provided,
        ));
    }

    logic_component(
        alias,
        None,
        ComponentType::Component,
        Some("unit"),
        relations,
    )
}

pub(super) fn unit_without_interfaces(alias: &str) -> LogicComponent {
    unit(alias, &[], &[])
}

pub(super) fn interface(alias: &str) -> LogicComponent {
    interface_entity(alias, None)
}

pub(super) fn interface_with_parent_id(alias: &str, parent_id: &str) -> LogicComponent {
    interface_entity(alias, Some(parent_id))
}

pub(super) fn relation(
    target: &str,
    relation_type: ComponentRelationType,
    source_role: EndpointRole,
) -> LogicRelation {
    LogicRelation {
        target: target.to_string(),
        annotation: None,
        relation_type,
        source_role,
        source_location: dummy_source_location(),
    }
}

fn interface_entity(alias: &str, parent_id: Option<&str>) -> LogicComponent {
    logic_component(alias, parent_id, ComponentType::Interface, None, Vec::new())
}

fn logic_component(
    alias: &str,
    parent_id: Option<&str>,
    element_type: ComponentType,
    stereotype: Option<&str>,
    relations: Vec<LogicRelation>,
) -> LogicComponent {
    LogicComponent {
        id: entity_id(alias, parent_id),
        name: Some(alias.to_string()),
        alias: Some(alias.to_string()),
        parent_id: parent_id.map(str::to_string),
        element_type,
        stereotype: stereotype.map(str::to_string),
        relations,
        source_location: dummy_source_location(),
    }
}

// Sequence diagram fixtures.

pub(super) fn sequence_diagrams(participants: &[&str]) -> SequenceDiagramInputs {
    SequenceDiagramInputs {
        diagrams: vec![SequenceTree {
            name: Some("seq".to_string()),
            participants: participants
                .iter()
                .map(|participant| sequence_participant(participant))
                .collect(),
            root: Block::default(),
        }],
    }
}

pub(super) fn sequence_calls(calls: &[(&str, &str, &str)]) -> SequenceDiagramInputs {
    let participants = calls
        .iter()
        .flat_map(|(caller, callee, _)| [*caller, *callee])
        .filter(|participant| !participant.is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(sequence_participant)
        .collect();

    SequenceDiagramInputs {
        diagrams: vec![SequenceTree {
            name: Some("seq".to_string()),
            participants,
            root: Block {
                items: calls
                    .iter()
                    .map(|(caller, callee, method)| {
                        Node::Interaction(Interaction {
                            sender: Some((*caller).to_string().into()),
                            receiver: Some((*callee).to_string().into()),
                            message: Some((*method).to_string()),
                            source_location: dummy_source_location(),
                        })
                    })
                    .collect(),
            },
        }],
    }
}

fn sequence_participant(participant: &str) -> SequenceParticipant {
    SequenceParticipant {
        display_name: participant.to_string(),
        alias: None,
        participant_type: ParticipantType::Participant,
        source_location: dummy_source_location(),
        stereotype: None,
    }
}

// Class diagram API fixtures.

pub(super) fn internal_api_index(interfaces: Vec<(&str, Vec<&str>)>) -> InternalApiIndex {
    let diagrams = vec![ClassDiagram {
        name: "internal_api".to_string(),
        entities: interfaces
            .into_iter()
            .map(|(interface_name, methods)| {
                let mut interface = class_interface(interface_name, None);
                interface.methods = methods.into_iter().map(method).collect();
                interface
            })
            .collect(),
    }];

    InternalApiIndex::build_index(&diagrams)
}

pub(super) fn class_interface(name: &str, namespace: Option<&str>) -> SimpleEntity {
    simple_entity(name, EntityType::Interface, namespace)
}

fn simple_entity(name: &str, entity_type: EntityType, namespace: Option<&str>) -> SimpleEntity {
    SimpleEntity {
        id: entity_id(name, namespace),
        name: name.to_string(),
        enclosing_namespace_id: namespace.map(str::to_string),
        stereotypes: Vec::new(),
        entity_type,
        type_aliases: Vec::new(),
        variables: Vec::new(),
        methods: Vec::new(),
        template_parameters: None,
        enum_literals: Vec::new(),
        relationships: Vec::new(),
        source_location: dummy_source_location(),
    }
}

fn method(name: &str) -> Method {
    Method {
        name: name.to_string(),
        return_type: None,
        source_location: dummy_source_location(),
        visibility: Visibility::Public,
        parameters: Vec::new(),
        template_parameters: None,
        modifiers: Vec::new(),
    }
}
