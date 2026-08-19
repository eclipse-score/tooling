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
use crate::models::{ClassDiagramInputs, ClassEntityIndex, SequenceDiagramInputs};
use crate::ValidationResult;
use class_diagram::{ClassDiagram, RelationType, Relationship};

fn validate(
    design_classes: ClassDiagramInputs,
    sequence_diagrams: SequenceDiagramInputs,
) -> ValidationResult {
    let mut setup_result = ValidationResult::default();
    let design_index = ClassEntityIndex::build_index(&design_classes, &mut setup_result);
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(
        setup_result.is_empty(),
        "test fixture setup failed: {:?}",
        setup_result.failures
    );

    validate_class_design_sequence(&design_index, &sequence_index)
}

fn class_diagrams(entities: Vec<class_diagram::SimpleEntity>) -> ClassDiagramInputs {
    vec![ClassDiagram {
        name: "class_design".to_string(),
        entities,
    }]
}

fn class_entity(id: &str, namespace: Option<&str>) -> class_diagram::SimpleEntity {
    let mut entity = class_interface(id, namespace);
    entity.entity_type = class_diagram::EntityType::Class;
    entity
}

#[test]
fn passes_when_all_sequence_participants_match_design_classes() {
    let design_classes = class_diagrams(vec![
        class_entity("Controller", None),
        class_entity("Repository", None),
    ]);
    let sequence_diagrams = sequence_diagrams(&["Controller", "Repository"]);

    let validation_result = validate(design_classes, sequence_diagrams);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn reports_sequence_participant_missing_from_design_classes() {
    let design_classes = class_diagrams(vec![class_entity("Controller", None)]);
    let sequence_diagrams = sequence_calls(&[("Controller", "Repository", "FindById()")]);

    let validation_result = validate(design_classes, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains(
        "[Class] Sequence participant \"Repository\" has no matching class in the class diagram."
    ));
    assert!(validation_result.failures[0].contains("\"Repository\""));
}

#[test]
fn reports_sequence_participant_missing_with_fuzzy_class_suggestion() {
    let design_classes = class_diagrams(vec![class_entity("Repository", None)]);
    let sequence_diagrams = sequence_diagrams(&["Repositry"]);

    let validation_result = validate(design_classes, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("Suggestion for \"Repositry\" : Did you mean class \"Repository\"?"));
}

#[test]
fn matches_sequence_participant_against_fully_qualified_class_id() {
    let design_classes = class_diagrams(vec![class_entity("Controller", Some("unit_1"))]);
    let sequence_diagrams = sequence_diagrams(&["unit_1::Controller"]);

    let validation_result = validate(design_classes, sequence_diagrams);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn matches_sequence_participant_against_unique_short_name() {
    let design_classes = class_diagrams(vec![class_entity("Controller", Some("unit_1"))]);
    let sequence_diagrams = sequence_diagrams(&["Controller"]);

    let validation_result = validate(design_classes, sequence_diagrams);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn reports_sequence_participant_with_ambiguous_short_name() {
    let design_classes = class_diagrams(vec![
        class_entity("Controller", Some("unit_1")),
        class_entity("Controller", Some("unit_2")),
    ]);
    let sequence_diagrams = sequence_diagrams(&["Controller"]);

    let validation_result = validate(design_classes, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains(
        "[Class] Sequence participant \"Controller\" matches multiple classes in the class diagram."
    ));
    assert!(validation_result.failures[0].contains("\"unit_1.Controller\""));
    assert!(validation_result.failures[0].contains("\"unit_2.Controller\""));
}

#[test]
fn passes_when_sequence_call_targets_existing_method_on_callee_class() {
    let mut repository = class_entity("Repository", None);
    repository.methods = vec![method("FindById")];

    let design_classes = class_diagrams(vec![class_entity("Controller", None), repository]);
    let sequence_diagrams = sequence_calls(&[("Controller", "Repository", "FindById()")]);

    let validation_result = validate(design_classes, sequence_diagrams);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn reports_sequence_call_method_missing_from_callee_class() {
    let mut repository = class_entity("Repository", None);
    repository.methods = vec![method("Store")];

    let design_classes = class_diagrams(vec![class_entity("Controller", None), repository]);
    let sequence_diagrams = sequence_calls(&[("Controller", "Repository", "FindById()")]);

    let validation_result = validate(design_classes, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0].contains(
        "[Method] Sequence function \"FindById\" from sequence call \"Controller\" -> \"Repository\" : \"FindById\" not found on target class \"Repository\" or its accessible inherited types in the class diagram."
    ));
    assert!(validation_result.failures[0].contains("\"Repository\""));
}

#[test]
fn reports_sequence_call_method_missing_with_inherited_fuzzy_suggestion() {
    let mut repository_base = class_entity("RepositoryBase", None);
    repository_base.methods = vec![method("FindById")];

    let mut repository = class_entity("Repository", None);
    repository.relationships = vec![relationship(
        "Repository",
        "RepositoryBase",
        RelationType::Inheritance,
    )];

    let design_classes = class_diagrams(vec![
        class_entity("Controller", None),
        repository,
        repository_base,
    ]);
    let sequence_diagrams = sequence_calls(&[("Controller", "Repository", "FindByIds()")]);

    let validation_result = validate(design_classes, sequence_diagrams);

    assert_eq!(validation_result.failures.len(), 1);
    assert!(validation_result.failures[0]
        .contains("Suggestion for \"FindByIds\" : Did you mean method \"FindById\"?"));
}

#[test]
fn passes_when_sequence_self_call_targets_existing_method() {
    let mut controller = class_entity("Controller", None);
    controller.methods = vec![method("Validate")];

    let design_classes = class_diagrams(vec![controller]);
    let sequence_diagrams = sequence_calls(&[("Controller", "Controller", "Validate()")]);

    let validation_result = validate(design_classes, sequence_diagrams);

    assert!(validation_result.failures.is_empty());
}

#[test]
fn extracts_ignored_special_display_suffix() {
    let mut sequence_diagrams = sequence_diagrams(&["help"]);
    sequence_diagrams.diagrams[0].participants[0].display_name =
        ":Process/nara::com user".to_string();
    sequence_diagrams.diagrams[0].participants[0].alias = Some("help".to_string());

    let mut setup_result = ValidationResult::default();
    let sequence_index = sequence_diagrams.to_sequence_diagram_index(&mut setup_result);
    assert!(
        setup_result.is_empty(),
        "test fixture setup failed: {:?}",
        setup_result.failures
    );

    let participant_info = sequence_index.participant_info("help").unwrap();

    assert_eq!(
        ignored_special_display_suffix(participant_info),
        Some("ara::com user".to_string())
    );
}

fn relationship(source: &str, target: &str, relation_type: RelationType) -> Relationship {
    Relationship {
        source: source.to_string(),
        target: target.to_string(),
        relation_type,
        source_multiplicity: None,
        target_multiplicity: None,
        source_location: dummy_source_location(),
    }
}
