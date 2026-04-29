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

//! Validation: compare unit names from component diagrams with namespace names
//! found in class diagrams.

use std::collections::BTreeSet;

use crate::models::{ClassDiagramIndex, ComponentDiagramArchitecture, Errors};

/// Run component-vs-class naming validation using prepared architecture/index
/// inputs.
pub fn validate_component_class(
    component_diagram: &ComponentDiagramArchitecture,
    class_diagram: &ClassDiagramIndex,
    errors: Errors,
) -> Errors {
    ComponentClassValidator::new(
        build_expected_unit_names(component_diagram),
        &class_diagram.observed_namespace_names,
        errors,
    )
    .run()
}

/// Verifies naming consistency between component-diagram units and
/// class-diagram namespaces.
pub struct ComponentClassValidator<'a> {
    expected_unit_names: BTreeSet<String>,
    observed_namespace_names: &'a BTreeSet<String>,
    errors: Errors,
}

impl<'a> ComponentClassValidator<'a> {
    fn new(
        expected_unit_names: BTreeSet<String>,
        observed_namespace_names: &'a BTreeSet<String>,
        errors: Errors,
    ) -> Self {
        Self {
            expected_unit_names,
            observed_namespace_names,
            errors,
        }
    }
    /// Run the consistency check and return accumulated errors.
    pub fn run(mut self) -> Errors {
        self.errors.debug_output = self.build_debug_log();
        self.check_unit_naming_consistency();
        self.errors
    }

    fn build_debug_log(&self) -> String {
        let mut log = String::new();

        log.push_str("DEBUG: Expected unit aliases from component diagrams:\n");
        for name in &self.expected_unit_names {
            log.push_str(&format!("  {name}\n"));
        }

        log.push_str("DEBUG: Observed namespace IDs from class diagrams:\n");
        for name in self.observed_namespace_names {
            log.push_str(&format!("  {name}\n"));
        }

        log
    }

    fn check_unit_naming_consistency(&mut self) {
        // Present in component diagrams but missing as namespaces in class
        // diagrams.
        for missing_name in self
            .expected_unit_names
            .difference(&self.observed_namespace_names)
        {
            self.errors.push(format!(
                "Naming consistency violation: missing unit namespace in class diagrams:\n\
                  Expected unit name: \"{}\"\n\
                  Source            : Component diagram unit identifiers\n\
                  Action            : Add/rename class-diagram namespace to match the unit name",
                missing_name
            ));
        }

        // Present as class-diagram namespaces but not declared as component
        // units.
        for extra_name in self
            .observed_namespace_names
            .difference(&self.expected_unit_names)
        {
            self.errors.push(format!(
                "Naming consistency violation: unexpected class-diagram unit namespace:\n\
                  Namespace name    : \"{}\"\n\
                  Source            : Unit class diagrams\n\
                  Action            : Rename namespace to an existing component-diagram unit identifier",
                extra_name
            ));
        }
    }
}

fn build_expected_unit_names(component_diagram: &ComponentDiagramArchitecture) -> BTreeSet<String> {
    // Unit aliases define expected logical names directly. Parent hierarchy is
    // intentionally ignored.
    component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_unit())
        .filter_map(|entity| entity.alias.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ClassDiagramEntityInput, ClassDiagramInput, ClassDiagramInputs, ComponentDiagramInput,
        ComponentDiagramInputs,
    };

    fn component_diagrams(units: &[&str]) -> ComponentDiagramInputs {
        ComponentDiagramInputs {
            entities: units
                .iter()
                .map(|name| ComponentDiagramInput {
                    id: (*name).to_string(),
                    alias: Some((*name).to_string()),
                    parent_id: None,
                    stereotype: Some("unit".to_string()),
                })
                .collect(),
        }
    }

    fn component_diagrams_with_hierarchy(
        entities: &[(&str, Option<&str>, Option<&str>, &str)],
    ) -> ComponentDiagramInputs {
        ComponentDiagramInputs {
            entities: entities
                .iter()
                .map(|(id, alias, parent_id, stereotype)| ComponentDiagramInput {
                    id: (*id).to_string(),
                    alias: alias.map(str::to_string),
                    parent_id: parent_id.map(str::to_string),
                    stereotype: Some((*stereotype).to_string()),
                })
                .collect(),
        }
    }

    fn class_diagrams(namespaces: &[&str]) -> ClassDiagramInputs {
        ClassDiagramInputs {
            diagrams: vec![ClassDiagramInput {
                name: "diagram".to_string(),
                entities: namespaces
                    .iter()
                    .enumerate()
                    .map(|(index, parent_id)| ClassDiagramEntityInput {
                        id: format!("entity_{index}"),
                        name: Some(format!("entity_{index}")),
                        alias: None,
                        parent_id: Some((*parent_id).to_string()),
                        entity_type: "Class".to_string(),
                        stereotypes: Vec::new(),
                        template_params: Vec::new(),
                        source_file: None,
                        source_line: 0,
                    })
                    .collect(),
                relationships: Vec::new(),
                source_files: Vec::new(),
                version: None,
            }],
        }
    }

    fn run_component_class_validation(
        component_diagrams: &ComponentDiagramInputs,
        class_diagrams: &ClassDiagramInputs,
    ) -> Errors {
        let mut errors = Errors::default();
        let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
        let class_index = class_diagrams.to_class_diagram_index(&mut errors);

        validate_component_class(&component_arch, &class_index, errors)
    }

    fn class_diagrams_from_entity_parent_ids(parent_ids: &[&str]) -> ClassDiagramInputs {
        ClassDiagramInputs {
            diagrams: vec![ClassDiagramInput {
                name: "diagram".to_string(),
                entities: parent_ids
                    .iter()
                    .enumerate()
                    .map(|(index, parent_id)| ClassDiagramEntityInput {
                        id: format!("entity_{index}"),
                        name: Some(format!("entity_{index}")),
                        alias: None,
                        parent_id: Some((*parent_id).to_string()),
                        entity_type: "Class".to_string(),
                        stereotypes: Vec::new(),
                        template_params: Vec::new(),
                        source_file: None,
                        source_line: 0,
                    })
                    .collect(),
                relationships: Vec::new(),
                source_files: Vec::new(),
                version: None,
            }],
        }
    }

    #[test]
    fn naming_consistency_passes_for_exact_match() {
        let component_diagrams = component_diagrams(&["unit_1", "Unit_2"]);
        let class_diagrams = class_diagrams(&["unit_1", "Unit_2"]);

        let errors = run_component_class_validation(&component_diagrams, &class_diagrams);

        assert!(errors.is_empty());
    }

    #[test]
    fn naming_consistency_reports_missing_and_extra() {
        let component_diagrams = component_diagrams(&["unit_1", "unit_2", "unit_3"]);
        let class_diagrams = class_diagrams(&["unit_2", "Unit_3"]);

        let errors = run_component_class_validation(&component_diagrams, &class_diagrams);

        assert!(!errors.is_empty());
        assert_eq!(errors.messages.len(), 3);

        let missing_count = errors
            .messages
            .iter()
            .filter(|message| message.contains("missing unit namespace in class diagrams"))
            .count();
        let unexpected_count = errors
            .messages
            .iter()
            .filter(|message| message.contains("unexpected class-diagram unit namespace"))
            .count();

        assert_eq!(missing_count, 2);
        assert_eq!(unexpected_count, 1);
    }

    #[test]
    fn units_without_alias_are_skipped() {
        let component_diagrams = ComponentDiagramInputs {
            entities: vec![
                ComponentDiagramInput {
                    id: "unit_with_alias".to_string(),
                    alias: Some("unit_with_alias".to_string()),
                    parent_id: None,
                    stereotype: Some("unit".to_string()),
                },
                ComponentDiagramInput {
                    id: "unit_without_alias".to_string(),
                    alias: None,
                    parent_id: None,
                    stereotype: Some("unit".to_string()),
                },
            ],
        };

        let class_diagrams = class_diagrams(&["unit_with_alias"]);

        let errors = run_component_class_validation(&component_diagrams, &class_diagrams);
        assert!(
            errors.is_empty(),
            "Expected pass when unit without alias is ignored, got: {:?}",
            errors.messages
        );
    }

    #[test]
    fn entity_parent_ids_are_used_as_observed_namespaces() {
        let component_diagrams = component_diagrams(&["unit_1"]);
        let class_diagrams = class_diagrams_from_entity_parent_ids(&["unit_1"]);

        let errors = run_component_class_validation(&component_diagrams, &class_diagrams);
        assert!(
            errors.is_empty(),
            "Expected pass when entity parent IDs match unit aliases, got: {:?}",
            errors.messages
        );
    }

    #[test]
    fn parent_unit_aliases_are_not_prefixed_into_expected_names() {
        let component_diagrams = component_diagrams_with_hierarchy(&[
            ("component_1", Some("component_1"), None, "component"),
            ("unit_parent", Some("parent"), Some("component_1"), "unit"),
            ("unit_child", Some("child"), Some("unit_parent"), "unit"),
            ("unit_leaf", Some("leaf"), Some("unit_child"), "unit"),
        ]);
        let class_diagrams = class_diagrams_from_entity_parent_ids(&["parent", "child", "leaf"]);

        let errors = run_component_class_validation(&component_diagrams, &class_diagrams);

        assert!(
            errors.is_empty(),
            "Expected pass when only direct unit aliases are compared, got: {:?}",
            errors.messages
        );
    }
}
