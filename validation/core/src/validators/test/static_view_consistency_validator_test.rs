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
use crate::models::{ComponentDiagramInputs, ComponentType, LogicComponent};
use crate::validators::fixtures::dummy_source_location;

fn entity(
    id: &str,
    alias: Option<&str>,
    parent_id: Option<&str>,
    stereotype: Option<&str>,
) -> LogicComponent {
    LogicComponent {
        id: id.to_string(),
        name: alias.map(|s| s.to_string()),
        alias: alias.map(|s| s.to_string()),
        parent_id: parent_id.map(|s| s.to_string()),
        element_type: ComponentType::Component,
        stereotype: stereotype.map(|s| s.to_string()),
        relations: Vec::new(),
        source_location: dummy_source_location(),
    }
}

fn diagram(entities: Vec<LogicComponent>) -> ComponentDiagramInputs {
    ComponentDiagramInputs { entities }
}

fn run(
    static_entities: Vec<LogicComponent>,
    static_view_entities: Vec<LogicComponent>,
) -> ValidationResult {
    let mut result = ValidationResult::default();
    let static_diagram = diagram(static_entities).to_diagram_architecture(&mut result);
    let static_view_diagram =
        diagram(static_view_entities).to_static_view_architecture(&mut result);
    result.merge(validate_static_view_consistency(
        &static_diagram,
        &static_view_diagram,
    ));
    result
}

#[test]
fn static_view_subset_of_static_passes() {
    let static_entities = vec![
        entity("CompA", Some("comp_a"), None, Some("component")),
        entity("CompA.Unit1", Some("unit_1"), Some("CompA"), Some("unit")),
        entity("CompA.Unit2", Some("unit_2"), Some("CompA"), Some("unit")),
    ];
    let static_view_entities = vec![
        entity("CompA", Some("comp_a"), None, Some("component")),
        entity("CompA.Unit1", Some("unit_1"), Some("CompA"), Some("unit")),
    ];

    let result = run(static_entities, static_view_entities);
    assert!(
        result.is_empty(),
        "Expected pass, got: {:?}",
        result.failures
    );
}

#[test]
fn parentless_static_view_unit_matches_unique_static_unit() {
    let static_entities = vec![
        entity("MwCom", Some("mw_com"), None, Some("SEooC")),
        entity(
            "MwCom.BindingFactories",
            Some("binding_factories"),
            Some("MwCom"),
            Some("unit"),
        ),
    ];
    let static_view_entities = vec![entity(
        "BindingFactories",
        Some("binding_factories"),
        None,
        Some("unit"),
    )];

    let result = run(static_entities, static_view_entities);
    assert!(
        result.is_empty(),
        "Expected parentless static-view unit to match unique static unit, got: {:?}",
        result.failures
    );
}

#[test]
fn parentless_static_view_unit_with_ambiguous_static_name_fails() {
    let static_entities = vec![
        entity("CompA", Some("comp_a"), None, Some("component")),
        entity("CompB", Some("comp_b"), None, Some("component")),
        entity(
            "CompA.SharedUnit",
            Some("shared_unit"),
            Some("CompA"),
            Some("unit"),
        ),
        entity(
            "CompB.SharedUnit",
            Some("shared_unit"),
            Some("CompB"),
            Some("unit"),
        ),
    ];
    let static_view_entities = vec![entity(
        "SharedUnit",
        Some("shared_unit"),
        None,
        Some("unit"),
    )];

    let result = run(static_entities, static_view_entities);
    assert!(result.failures.iter().any(|message| message.contains(
        "Unit \"shared_unit\" in the static_view diagram is not defined in the static diagram"
    )));
}

#[test]
fn static_view_component_not_in_static_fails() {
    let static_entities = vec![entity("CompA", Some("comp_a"), None, Some("component"))];
    let static_view_entities = vec![entity("CompB", Some("comp_b"), None, Some("component"))];

    let result = run(static_entities, static_view_entities);
    assert!(result.failures.iter().any(|message| message.contains(
        "Component \"comp_b\" in the static_view diagram is not defined in the static diagram"
    )));
}

#[test]
fn static_view_unit_not_in_static_fails() {
    let static_entities = vec![
        entity("CompA", Some("comp_a"), None, Some("component")),
        entity("CompA.Unit1", Some("unit_1"), Some("CompA"), Some("unit")),
    ];
    let static_view_entities = vec![
        entity("CompA", Some("comp_a"), None, Some("component")),
        entity("CompA.Unit1", Some("unit_1"), Some("CompA"), Some("unit")),
        entity("CompA.Unit2", Some("unit_2"), Some("CompA"), Some("unit")),
    ];

    let result = run(static_entities, static_view_entities);
    assert!(result.failures.iter().any(|message| message.contains(
        "Unit \"unit_2\" in the static_view diagram is not defined in the static diagram"
    )));
}

#[test]
fn static_view_entity_repeated_across_views_is_not_a_duplicate() {
    // Simulates the same entity being declared in two separate static_view
    // diagram files, which are merged before consistency checking.
    let static_entities = vec![
        entity("CompA", Some("comp_a"), None, Some("component")),
        entity("CompA.Unit1", Some("unit_1"), Some("CompA"), Some("unit")),
    ];
    let static_view_entities = vec![
        entity("CompA", Some("comp_a"), None, Some("component")),
        entity("CompA.Unit1", Some("unit_1"), Some("CompA"), Some("unit")),
        entity("CompA", Some("comp_a"), None, Some("component")),
        entity("CompA.Unit1", Some("unit_1"), Some("CompA"), Some("unit")),
    ];

    let result = run(static_entities, static_view_entities);
    assert!(
        result.is_empty(),
        "Expected no duplicate-entity error across static_view diagrams, got: {:?}",
        result.failures
    );
}
