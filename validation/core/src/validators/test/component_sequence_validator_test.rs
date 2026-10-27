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
    ComponentDiagramInputs, ComponentRelationType, ComponentType, EndpointRole, LogicComponent,
    SequenceDiagramInputs,
};
use crate::ValidationResult;

fn validate(
    component_diagrams: ComponentDiagramInputs,
    sequence_diagrams: SequenceDiagramInputs,
) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty(), "{:?}", setup_result.failures);

    validate_component_sequence(&component_arch, &sequence_index)
}

#[test]
fn passes_when_aliases_and_participants_are_identical() {
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
        .filter(|msg| msg.contains("from the component diagram not found in the sequence diagram"))
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
fn units_without_alias_are_matched_by_id() {
    let component_diagrams = ComponentDiagramInputs {
        entities: vec![LogicComponent {
            id: "module_a.unit_1".to_string(),
            name: Some("unit_1".to_string()),
            alias: None,
            parent_id: None,
            element_type: ComponentType::Component,
            stereotype: Some("unit".to_string()),
            relations: Vec::new(),
            source_location: dummy_source_location(),
        }],
    };
    let sequence_diagrams = sequence_diagrams(&["module_a.unit_1"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert!(validation_result.is_empty());
}

#[test]
fn links_qualified_participant_to_nested_unit_by_exact_id() {
    let component_diagrams = component_diagram(vec![
        component("comp_a"),
        unit_with_parent_id("unit_1", "comp_a"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["comp_a.unit_1"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert!(validation_result.is_empty());
}

#[test]
fn links_leaf_participant_to_unique_nested_unit() {
    let component_diagrams = component_diagram(vec![
        component("comp_a"),
        unit_with_parent_id("unit_1", "comp_a"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["unit_1"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert!(validation_result.is_empty());
}

#[test]
fn does_not_link_qualified_participant_by_suffix() {
    let component_diagrams = component_diagram(vec![
        component("comp_a"),
        component("comp_b"),
        unit_with_parent_id("unit_1", "comp_a"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["comp_b.unit_1"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 2);
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("\"comp_a.unit_1\" from the component diagram not found")
    }));
    assert!(validation_result.failures.iter().any(|message| {
        message.contains("Participant \"comp_b.unit_1\" from the sequence diagram not found")
    }));
}

#[test]
fn reports_ambiguous_leaf_participant_once() {
    let component_diagrams = component_diagram(vec![
        component("comp_a"),
        component("comp_b"),
        unit_with_parent_id("unit_1", "comp_a"),
        unit_with_parent_id("unit_1", "comp_b"),
    ]);
    let sequence_diagrams = sequence_diagrams(&["unit_1"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains(
        "[Naming] Participant \"unit_1\" from the sequence diagram matches multiple units in the component diagram."
    ));
    assert!(validation_result.failures[0].contains("\"comp_a.unit_1\", \"comp_b.unit_1\""));
}

#[test]
fn reports_alias_missing_from_participants() {
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
fn reports_participant_not_in_aliases() {
    let component_diagrams = component_diagram(vec![unit_without_interfaces("u1")]);
    let sequence_diagrams = sequence_diagrams(&["u1", "orphan"]);

    let validation_result = validate(component_diagrams, sequence_diagrams);
    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("\"orphan\""));
}

#[test]
fn reports_missing_component_alias_and_interface_connection_for_sequence_call() {
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
        message.contains("from the component diagram not found in the sequence diagram")
            && message.contains("\"u2\"")
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

fn validate_sequence(
    component_diagrams: ComponentDiagramInputs,
    participants: Vec<sequence_logic::SequenceParticipant>,
    calls: &[(
        Option<&str>,
        Option<&str>,
        &str,
        sequence_logic::MessageKind,
    )],
) -> ValidationResult {
    validate(component_diagrams, sequence_of(participants, calls))
}

#[test]
fn actor_external_and_gate_messages_are_environment() {
    use sequence_logic::MessageKind::SynchCall;

    let validation_result = validate_sequence(
        component_diagram(vec![unit_without_interfaces("unit_1")]),
        vec![
            actor_participant("app"),
            external_participant("platform"),
            element_participant("unit_1"),
        ],
        &[
            (Some("app"), Some("unit_1"), "Run()", SynchCall),
            (Some("unit_1"), Some("platform"), "Send()", SynchCall),
            (None, Some("unit_1"), "Trigger()", SynchCall),
        ],
    );

    assert!(
        validation_result.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

#[test]
fn reports_actor_that_names_a_unit_without_a_second_missing_unit_error() {
    let validation_result = validate_sequence(
        component_diagram(vec![unit_without_interfaces("unit_1")]),
        vec![actor_participant("unit_1")],
        &[],
    );

    assert_eq!(
        validation_result.failures.len(),
        1,
        "{:?}",
        validation_result.failures
    );
    assert!(validation_result.failures[0].contains(
        "Participant \"unit_1\" is declared as an actor but names a unit of the component diagram"
    ));
}

#[test]
fn reports_external_participant_that_names_a_component() {
    let validation_result = validate_sequence(
        component_diagram(vec![
            component("comp_a"),
            unit_with_parent_id("inner", "comp_a"),
        ]),
        vec![external_participant("comp_a"), element_participant("inner")],
        &[],
    );

    assert_eq!(
        validation_result.failures.len(),
        1,
        "{:?}",
        validation_result.failures
    );
    assert!(validation_result.failures[0].contains(
        "Participant \"comp_a\" is declared as an <<external>> participant but names a component of the component diagram"
    ));
}

#[test]
fn reports_uid_matching_both_a_unit_and_a_component() {
    let validation_result = validate_sequence(
        component_diagram(vec![
            component("comp_a"),
            unit_with_parent_id("shared", "comp_a"),
            component("shared"),
            unit_with_parent_id("inner", "shared"),
        ]),
        vec![element_participant("shared")],
        &[],
    );

    assert!(validation_result.failures.iter().any(|message| message.contains(
        "Participant \"shared\" from the sequence diagram matches multiple elements in the component diagram"
    )));
}

#[test]
fn replies_do_not_require_interface_connections() {
    use sequence_logic::MessageKind::{Reply, SynchCall};

    let validation_result = validate_sequence(
        component_diagram(vec![
            unit("unit_1", &["Iface"], &[]),
            unit("unit_2", &[], &["Iface"]),
            unit_without_interfaces("unit_3"),
            interface("Iface"),
        ]),
        vec![
            element_participant("unit_1"),
            element_participant("unit_2"),
            element_participant("unit_3"),
        ],
        &[
            (Some("unit_1"), Some("unit_2"), "Get()", SynchCall),
            (Some("unit_2"), Some("unit_3"), "result", Reply),
        ],
    );

    assert!(
        validation_result.is_empty(),
        "{:?}",
        validation_result.failures
    );
}

#[test]
fn component_lifeline_is_accepted_but_does_not_cover_its_units() {
    use sequence_logic::MessageKind::SynchCall;

    let validation_result = validate_sequence(
        component_diagram(vec![
            component("comp"),
            unit_with_parent_id("unit_1", "comp"),
            unit_without_interfaces("other"),
        ]),
        vec![element_participant("comp"), element_participant("other")],
        &[(Some("other"), Some("comp"), "Run()", SynchCall)],
    );

    let failures = &validation_result.failures;
    assert!(failures
        .iter()
        .any(|message| message.contains("Unit id \"comp.unit_1\"")));
    assert!(failures
        .iter()
        .all(|message| !message.contains("participant \"comp\"")));
}

#[test]
fn component_call_does_not_cover_the_connected_unit_pair() {
    use sequence_logic::MessageKind::SynchCall;

    let mut provider = unit_with_parent_id("provider", "comp");
    provider.relations = vec![relation(
        "Iface",
        ComponentRelationType::InterfaceBinding,
        EndpointRole::Provided,
    )];

    let validation_result = validate_sequence(
        component_diagram(vec![
            component("comp"),
            provider,
            unit("client", &["Iface"], &[]),
            interface("Iface"),
        ]),
        vec![
            element_participant("client"),
            element_participant("comp"),
            element_participant("comp.provider"),
        ],
        &[(Some("client"), Some("comp"), "Run()", SynchCall)],
    );

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("units \"client\" and \"comp.provider\" have no corresponding function-call"));
}

#[test]
fn reports_ambiguous_component_participant() {
    let nested_component = |parent_id: &str| LogicComponent {
        id: format!("{parent_id}.shared"),
        name: Some("shared".to_string()),
        alias: Some("shared".to_string()),
        parent_id: Some(parent_id.to_string()),
        element_type: ComponentType::Component,
        stereotype: Some("component".to_string()),
        relations: Vec::new(),
        source_location: dummy_source_location(),
    };

    let validation_result = validate(
        component_diagram(vec![
            component("p1"),
            component("p2"),
            nested_component("p1"),
            nested_component("p2"),
            unit_with_parent_id("u1", "p1.shared"),
            unit_with_parent_id("u2", "p2.shared"),
        ]),
        sequence_diagrams(&["shared", "p1.shared.u1", "p2.shared.u2"]),
    );

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("matches multiple components"));
    assert!(validation_result.failures[0].contains("\"p1.shared\", \"p2.shared\""));
}

#[test]
fn component_lifeline_call_needs_shared_interface_with_peer() {
    use sequence_logic::MessageKind::SynchCall;

    let validation_result = validate_sequence(
        component_diagram(vec![
            component("comp"),
            unit_with_parent_id("unit_1", "comp"),
            unit("other", &[], &[]),
        ]),
        vec![
            element_participant("comp"),
            element_participant("other"),
            element_participant("comp.unit_1"),
        ],
        &[
            (Some("other"), Some("comp"), "Run()", SynchCall),
            (Some("comp"), Some("comp.unit_1"), "Inner()", SynchCall),
        ],
    );

    assert_eq!(validation_result.failures.len(), 1);
    assert!(
        validation_result.failures[0].contains("equence-connected elements \"comp\" and \"other\"")
    );
}
