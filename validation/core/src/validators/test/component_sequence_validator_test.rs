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
    LogicRelation, SequenceDiagramInputs,
};
use sequence_logic::{Event, Interaction, SequenceNode, SequenceTree};

fn relation_with_role(target: &str, source_role: EndpointRole) -> LogicRelation {
    relation_with_type_and_role(target, ComponentRelationType::InterfaceBinding, source_role)
}

fn relation_with_type_and_role(
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

#[test]
fn passes_when_aliases_and_participants_are_identical() {
    let component_diagrams = component_diagrams(&["unit_1", "unit_2"]);
    let sequence_diagrams = sequence_diagrams(&["unit_1", "unit_2"]);

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);
    assert!(validation_result.is_empty());
}

#[test]
fn reports_missing_and_extra() {
    let component_diagrams = component_diagrams(&["unit_1", "unit_2", "unit_3"]);
    let sequence_diagrams = sequence_diagrams(&["unit_2", "unit_4"]);

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);

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
        }],
    };
    let sequence_diagrams = sequence_diagrams(&[]);

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);
    assert!(validation_result.is_empty());
}

#[test]
fn reports_alias_missing_from_participants() {
    let component_diagrams = component_diagrams(&["u1", "u2"]);
    let sequence_diagrams = sequence_diagrams(&["u1"]);

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);
    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("\"u2\""));
}

#[test]
fn reports_participant_not_in_aliases() {
    let component_diagrams = component_diagrams(&["u1"]);
    let sequence_diagrams = sequence_diagrams(&["u1", "orphan"]);

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);
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

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);

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

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);

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

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);

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

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);

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

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);
    assert!(validation_result.is_empty());
}

#[test]
fn passes_when_sequence_call_matches_consumer_provider_roles() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit_with_interface_roles("u1", &["InternalInterface"], &[]),
        unit_with_interface_roles("u2", &[], &["InternalInterface"]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);
    assert!(validation_result.is_empty());
}

#[test]
fn passes_when_multiple_consumers_share_interface_without_calling_each_other() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit_with_interface_roles("u1", &["InternalInterface"], &[]),
        unit_with_interface_roles("u2", &[], &["InternalInterface"]),
        unit_with_interface_roles("u3", &["InternalInterface"], &[]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()"), ("u3", "u2", "GetData()")]);

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);
    assert!(validation_result.is_empty());
}

#[test]
fn reports_cross_unit_sequence_call_with_invalid_consumer_provider_roles() {
    let component_diagrams = component_diagrams_with_entities(vec![
        unit_with_interface_roles("u1", &[], &["InternalInterface"]),
        unit_with_interface_roles("u2", &["InternalInterface"], &[]),
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("sequence interaction does not match consumer/provider roles"));
    assert!(validation_result.failures[0]
        .contains("Sequence call       : \"u1\" -> \"u2\" : \"GetData()\""));
    assert!(validation_result.failures[0].contains(
        "Expected caller role: \"u1\" should require shared interface(s) \"InternalInterface\""
    ));
    assert!(validation_result.failures[0].contains(
        "Expected callee role: \"u2\" should provide shared interface(s) \"InternalInterface\""
    ));
}

#[test]
fn ignores_source_roles_on_non_interface_binding_relations() {
    let component_diagrams = component_diagrams_with_entities(vec![
        LogicComponent {
            id: "some_id.u1".to_string(),
            name: Some("u1".to_string()),
            alias: Some("u1".to_string()),
            parent_id: None,
            element_type: ComponentType::Component,
            stereotype: Some("unit".to_string()),
            relations: vec![relation_with_type_and_role(
                "InternalInterface",
                ComponentRelationType::Dependency,
                EndpointRole::Provided,
            )],
        },
        LogicComponent {
            id: "some_id.u2".to_string(),
            name: Some("u2".to_string()),
            alias: Some("u2".to_string()),
            parent_id: None,
            element_type: ComponentType::Component,
            stereotype: Some("unit".to_string()),
            relations: vec![relation_with_type_and_role(
                "InternalInterface",
                ComponentRelationType::Dependency,
                EndpointRole::Required,
            )],
        },
        interface("InternalInterface"),
    ]);
    let sequence_diagrams = sequence_calls(&[("u1", "u2", "GetData()")]);

    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty());

    let validation_result = validate_component_sequence(&component_arch, &sequence_index);
    assert!(validation_result.is_empty());
}
