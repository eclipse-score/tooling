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

use std::collections::BTreeMap;

use source_location::SourceLocation;

use crate::models::{BazelArchitecture, ComponentDiagramArchitecture};
use crate::results::{Diagnostics, ErrorBuilder, ErrorCategory};
use crate::validators::shared::{format_source_file, format_source_line};
use crate::ValidationResult;

#[derive(Clone, Debug)]
pub(crate) struct UnitDesignEntity {
    pub id: String,
    pub source_location: SourceLocation,
}

/// Verify that every architectural unit is refined by an eligible entity whose
/// ID is nested below the full architectural unit ID in the bound unit design.
pub fn validate_architectural_unit_class_design(
    bazel: &BazelArchitecture,
    component: &ComponentDiagramArchitecture,
    design_entities_by_unit_label: &BTreeMap<String, Vec<UnitDesignEntity>>,
) -> ValidationResult {
    let mut result = ValidationResult::default();

    for (unit_key, architectural_unit) in &component.unit_set {
        let Some(unit_label) = bazel.unit_set.get(unit_key) else {
            // Bazel-component validation reports this independently.
            continue;
        };

        let entities = design_entities_by_unit_label
            .get(unit_label)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        validate_unit(architectural_unit, unit_label, entities, &mut result);
    }

    result
}

fn validate_unit(
    architectural_unit: &crate::models::LogicComponent,
    unit_label: &str,
    entities: &[UnitDesignEntity],
    result: &mut ValidationResult,
) {
    let refinements: Vec<&UnitDesignEntity> = entities
        .iter()
        .filter(|entity| refines_unit(&entity.id, &architectural_unit.id))
        .collect();

    log_unit_diagnostics(
        &mut result.diagnostics,
        &architectural_unit.id,
        unit_label,
        entities,
        &refinements,
    );

    if refinements.is_empty() {
        result.add_failure(missing_refinement_error(
            &architectural_unit.id,
            unit_label,
            entities,
        ));
    }
}

fn refines_unit(entity_id: &str, unit_id: &str) -> bool {
    entity_id
        .to_lowercase()
        .starts_with(&format!("{}.", unit_id.to_lowercase()))
}

fn missing_refinement_error(
    unit_id: &str,
    unit_label: &str,
    entities: &[UnitDesignEntity],
) -> String {
    let mut error = ErrorBuilder::new(ErrorCategory::Design)
        .title(format!(
            "Architectural unit \"{unit_id}\" has no refining entity in its unit design"
        ))
        .field("architectural unit", unit_id)
        .field("bazel unit", unit_label);

    if entities.is_empty() {
        error = error.field("unit design entities", "<none>");
    } else {
        for entity in entities {
            error = error
                .field("entity", format!("\"{}\"", entity.id))
                .field(
                    "design source file",
                    format!("\"{}\"", format_source_file(&entity.source_location)),
                )
                .field(
                    "design source line",
                    format_source_line(&entity.source_location),
                );
        }
    }

    error
        .fix(format!(
            "add a class, struct, interface, or abstract class whose ID is below the architectural unit ID \"{unit_id}\""
        ))
        .build()
}

fn log_unit_diagnostics(
    diagnostics: &mut Diagnostics,
    unit_id: &str,
    unit_label: &str,
    entities: &[UnitDesignEntity],
    refinements: &[&UnitDesignEntity],
) {
    diagnostics.debug(|| {
        format!(
            "Architectural unit: {}\nBazel unit: {}\nEligible entity identifiers: {}\nSelected refinements: {}",
            unit_id,
            unit_label,
            display_entities(entities),
            display_refinement_ids(refinements),
        )
    });
}

fn display_entities(entities: &[UnitDesignEntity]) -> String {
    display_list(
        entities
            .iter()
            .map(|entity| {
                let (file, line) = entity.source_location.display();
                format!("{} ({}:{})", entity.id, file, line)
            })
            .collect(),
    )
}

fn display_refinement_ids(entities: &[&UnitDesignEntity]) -> String {
    display_list(entities.iter().map(|entity| entity.id.clone()).collect())
}

fn display_list(items: Vec<String>) -> String {
    if items.is_empty() {
        "<none>".to_string()
    } else {
        items.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        BazelEntityRef, BazelInput, BazelInputEntry, ComponentDiagramInputs, ComponentType,
        LogicComponent,
    };
    use std::collections::BTreeMap;

    fn source_location() -> source_location::SourceLocation {
        source_location::SourceLocation::new("test.puml", 1)
    }

    fn component_entity(id: &str, parent_id: Option<&str>, stereotype: &str) -> LogicComponent {
        LogicComponent {
            id: id.to_string(),
            name: id.rsplit('.').next().map(str::to_string),
            alias: None,
            parent_id: parent_id.map(str::to_string),
            element_type: if stereotype == "SEooC" {
                ComponentType::Package
            } else {
                ComponentType::Component
            },
            stereotype: Some(stereotype.to_string()),
            relations: Vec::new(),
            source_location: source_location(),
        }
    }

    fn bazel_ref(label: &str, design_name: &str) -> BazelEntityRef {
        BazelEntityRef {
            label: label.to_string(),
            design_name: design_name.to_string(),
        }
    }

    fn inputs(
        class_ids: &[&str],
    ) -> (
        BazelArchitecture,
        ComponentDiagramArchitecture,
        BTreeMap<String, Vec<UnitDesignEntity>>,
    ) {
        let mut components = BTreeMap::new();
        components.insert(
            "@//:system".to_string(),
            BazelInputEntry {
                units: Vec::new(),
                components: vec![bazel_ref("@//:control", "control")],
            },
        );
        components.insert(
            "@//:control".to_string(),
            BazelInputEntry {
                units: vec![bazel_ref("@//:filter", "Filter")],
                components: Vec::new(),
            },
        );

        let mut result = ValidationResult::default();
        let bazel = BazelInput { components }.to_bazel_architecture(&mut result);
        let component = ComponentDiagramInputs {
            entities: vec![
                component_entity("system", None, "SEooC"),
                component_entity("system.control", Some("system"), "component"),
                component_entity("system.control.Filter", Some("system.control"), "unit"),
            ],
        }
        .to_diagram_architecture(&mut result);
        assert!(
            result.is_empty(),
            "fixture setup failed: {:?}",
            result.failures
        );

        let classes = BTreeMap::from([(
            "@//:filter".to_string(),
            class_ids
                .iter()
                .map(|id| UnitDesignEntity {
                    id: (*id).to_string(),
                    source_location: source_location(),
                })
                .collect(),
        )]);

        (bazel, component, classes)
    }

    #[test]
    fn accepts_a_class_in_the_bound_unit_design_namespace() {
        let (bazel, component, classes) = inputs(&["system.control.Filter.FilterService"]);

        let result = validate_architectural_unit_class_design(&bazel, &component, &classes);

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
    }

    #[test]
    fn rejects_a_class_from_another_namespace() {
        let (bazel, component, classes) = inputs(&["system.control.OtherUnit.Service"]);

        let result = validate_architectural_unit_class_design(&bazel, &component, &classes);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains(
            "Architectural unit \"system.control.Filter\" has no refining entity in its unit design."
        ));
        assert!(result.failures[0].contains("Design source file : \"test.puml\""));
        assert!(result.failures[0].contains("Design source line : 1"));
    }

    #[test]
    fn rejects_a_class_with_the_same_unit_name_under_another_parent() {
        let (bazel, component, classes) = inputs(&["system.other.Filter.FilterService"]);

        let result = validate_architectural_unit_class_design(&bazel, &component, &classes);

        assert_eq!(result.failures.len(), 1);
    }

    #[test]
    fn rejects_a_lexical_but_not_segment_prefix() {
        let (bazel, component, classes) = inputs(&["system.control.Filtering.FilterService"]);

        let result = validate_architectural_unit_class_design(&bazel, &component, &classes);

        assert_eq!(result.failures.len(), 1);
    }
}
