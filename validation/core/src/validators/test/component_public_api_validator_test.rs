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
};
use crate::ValidationResult;
use class_diagram::ClassDiagram;

fn seooc_with_public_api_relations(alias: &str, relation_targets: &[&str]) -> LogicComponent {
    let mut seooc = unit_without_interfaces(alias);
    seooc.stereotype = Some("SEooC".to_string());
    seooc.relations = relation_targets
        .iter()
        .map(|target| {
            relation(
                target,
                ComponentRelationType::InterfaceBinding,
                EndpointRole::Required,
            )
        })
        .collect();
    seooc
}

fn public_api_index(interfaces: Vec<(&str, Option<&str>)>) -> PublicApiIndex {
    let diagrams = vec![ClassDiagram {
        name: "public_api".to_string(),
        entities: interfaces
            .into_iter()
            .map(|(interface_name, namespace)| class_interface(interface_name, namespace))
            .collect(),
    }];

    PublicApiIndex::build_index(&diagrams)
}

fn validate(
    component_diagrams: ComponentDiagramInputs,
    public_api: &PublicApiIndex,
) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let component_arch = component_diagrams.to_diagram_architecture(&mut setup_result);
    assert!(
        setup_result.is_empty(),
        "test fixture setup failed: {:?}",
        setup_result.failures
    );

    validate_component_public_api(&component_arch, public_api)
}

#[test]
fn passes_when_top_level_public_api_is_declared_and_related_from_seooc() {
    let component_diagrams = component_diagram(vec![
        seooc_with_public_api_relations("sample_seooc", &["SampleLibraryAPI"]),
        interface("SampleLibraryAPI"),
    ]);
    let public_api = public_api_index(vec![("SampleLibraryAPI", Some("sample_seooc"))]);

    let validation_result = validate(component_diagrams, &public_api);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn reports_missing_component_public_api_declaration() {
    let component_diagrams = component_diagram(vec![
        seooc_with_public_api_relations("sample_seooc", &["SampleLibraryAPI"]),
        interface("SampleLibraryAPI"),
    ]);
    let public_api = public_api_index(vec![("OtherAPI", Some("sample_seooc"))]);

    let validation_result = validate(component_diagrams, &public_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains("Missing public API declaration"));
    assert!(validation_result.failures[0].contains("Missing public APIs : \"SampleLibraryAPI\""));
}

#[test]
fn reports_public_api_without_seooc_relationship() {
    let component_diagrams = component_diagram(vec![
        seooc_with_public_api_relations("sample_seooc", &[]),
        interface("SampleLibraryAPI"),
    ]);
    let public_api = public_api_index(vec![("SampleLibraryAPI", Some("sample_seooc"))]);

    let validation_result = validate(component_diagrams, &public_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("Public API interface has no component relationship"));
    assert!(validation_result.failures[0].contains("\"SampleLibraryAPI\""));
}

#[test]
fn ignores_component_relationships_when_checking_public_api() {
    let mut component = unit_without_interfaces("component_example");
    component.stereotype = Some("component".to_string());
    component.relations = vec![relation(
        "SampleLibraryAPI",
        ComponentRelationType::InterfaceBinding,
        EndpointRole::Required,
    )];

    let component_diagrams = component_diagram(vec![
        seooc_with_public_api_relations("sample_seooc", &[]),
        component,
        interface("SampleLibraryAPI"),
    ]);
    let public_api = public_api_index(vec![("SampleLibraryAPI", Some("component_example"))]);

    let validation_result = validate(component_diagrams, &public_api);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("Public API interface has no component relationship"));
    assert!(validation_result.failures[0].contains("\"SampleLibraryAPI\""));
}

#[test]
fn ignores_internal_interface_bindings_when_checking_public_api() {
    let component_diagrams = component_diagram(vec![
        unit("u1", &["component_example.InternalInterface"], &[]),
        unit_without_interfaces("component_example"),
        interface_with_parent_id("InternalInterface", "component_example"),
    ]);
    let public_api = public_api_index(vec![]);

    let validation_result = validate(component_diagrams, &public_api);

    assert!(validation_result.failures.is_empty());
}
