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

//! Validation: compare component-diagram unit IDs with enclosing
//! namespace IDs found in class diagrams.

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
        build_expected_unit_ids(component_diagram),
        class_diagram.enclosing_namespace_ids(),
        errors,
    )
    .run()
}

/// Verifies consistency between component-diagram unit IDs and
/// class-diagram enclosing namespace IDs.
struct ComponentClassValidator<'a> {
    expected_unit_ids: BTreeSet<String>,
    observed_namespace_ids: &'a BTreeSet<String>,
    errors: Errors,
}

impl<'a> ComponentClassValidator<'a> {
    fn new(
        expected_unit_ids: BTreeSet<String>,
        observed_namespace_ids: &'a BTreeSet<String>,
        errors: Errors,
    ) -> Self {
        Self {
            expected_unit_ids,
            observed_namespace_ids,
            errors,
        }
    }
    /// Run the consistency check and return accumulated errors.
    pub fn run(mut self) -> Errors {
        self.errors.debug_output.push_str(&self.build_debug_log());
        self.check_unit_naming_consistency();
        self.errors
    }

    fn build_debug_log(&self) -> String {
        let mut log = String::new();

        log.push_str("DEBUG: Expected unit IDs from component diagrams:\n");
        for unit_id in &self.expected_unit_ids {
            log.push_str(&format!("  {unit_id}\n"));
        }

        log.push_str("DEBUG: Observed enclosing namespace IDs from class diagrams:\n");
        for namespace_id in self.observed_namespace_ids {
            log.push_str(&format!("  {namespace_id}\n"));
        }

        log
    }

    fn check_unit_naming_consistency(&mut self) {
        // Every expected unit ID must end with at least one observed namespace
        // ID.
        for expected_unit_id in &self.expected_unit_ids {
            let has_matching_suffix =
                self.observed_namespace_ids
                    .iter()
                    .any(|observed_namespace_id| {
                        has_boundary_suffix(expected_unit_id, observed_namespace_id)
                    });

            if has_matching_suffix {
                continue;
            }

            self.errors.push(format!(
                "Naming consistency violation: no enclosing namespace ID suffix match for component unit ID:\n\
                  Expected unit ID   : \"{}\"\n\
                  Source             : Component diagram unit IDs\n\
                  Action             : Add/rename class-diagram enclosing namespace ID so it matches a suffix of this unit ID",
                expected_unit_id
            ));
        }

        // Every observed namespace ID must be a suffix of at least one
        // expected unit ID.
        for observed_namespace_id in self.observed_namespace_ids {
            let has_matching_suffix = self.expected_unit_ids.iter().any(|expected_unit_id| {
                has_boundary_suffix(expected_unit_id, observed_namespace_id)
            });

            if has_matching_suffix {
                continue;
            }

            self.errors.push(format!(
                "Naming consistency violation: enclosing namespace ID is not a suffix of any component unit ID:\n\
                  Namespace ID       : \"{}\"\n\
                  Source             : Class-diagram enclosing namespace IDs\n\
                  Action             : Rename namespace ID or component unit ID so the namespace ID becomes a suffix of a unit ID",
                observed_namespace_id
            ));
        }
    }
}

fn has_boundary_suffix(full_id: &str, suffix: &str) -> bool {
    full_id == suffix
        || (full_id.len() > suffix.len()
            && full_id.ends_with(suffix)
            && full_id.as_bytes()[full_id.len() - suffix.len() - 1] == b'.')
}

fn build_expected_unit_ids(component_diagram: &ComponentDiagramArchitecture) -> BTreeSet<String> {
    // Unit IDs define expected logical names directly. Parent hierarchy is
    // intentionally ignored.
    component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_unit())
        .map(|entity| entity.id.clone())
        .collect()
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::models::{ClassDiagramInputs, ComponentDiagramInput, ComponentDiagramInputs};
//     use class_diagram::{ClassDiagram, EntityType, SimpleEntity};

//     fn component_diagrams(units: &[&str]) -> ComponentDiagramInputs {
//         ComponentDiagramInputs {
//             entities: units
//                 .iter()
//                 .map(|name| ComponentDiagramInput {
//                     id: (*name).to_string(),
//                     alias: Some((*name).to_string()),
//                     parent_id: None,
//                     stereotype: Some("unit".to_string()),
//                 })
//                 .collect(),
//         }
//     }

//     fn component_diagrams_with_hierarchy(
//         entities: &[(&str, Option<&str>, Option<&str>, &str)],
//     ) -> ComponentDiagramInputs {
//         ComponentDiagramInputs {
//             entities: entities
//                 .iter()
//                 .map(|(id, alias, parent_id, stereotype)| ComponentDiagramInput {
//                     id: (*id).to_string(),
//                     alias: alias.map(str::to_string),
//                     parent_id: parent_id.map(str::to_string),
//                     stereotype: Some((*stereotype).to_string()),
//                 })
//                 .collect(),
//         }
//     }

//     fn class_diagrams(namespaces: &[&str]) -> ClassDiagramInputs {
//         vec![ClassDiagram {
//             name: "diagram".to_string(),
//             entities: namespaces
//                 .iter()
//                 .enumerate()
//                 .map(|(index, namespace_id)| SimpleEntity {
//                     id: format!("entity_{index}"),
//                     name: format!("entity_{index}"),
//                     enclosing_namespace_id: Some((*namespace_id).to_string()),
//                     entity_type: EntityType::Class,
//                     type_aliases: Vec::new(),
//                     variables: Vec::new(),
//                     methods: Vec::new(),
//                     template_parameters: None,
//                     enum_literals: Vec::new(),
//                     relationships: Vec::new(),
//                     source_file: None,
//                     source_line: None,
//                 })
//                 .collect(),
//             relationships: Vec::new(),
//             source_files: Vec::new(),
//             version: None,
//         }]
//     }

//     fn run_component_class_validation(
//         component_diagrams: &ComponentDiagramInputs,
//         class_diagrams: &ClassDiagramInputs,
//     ) -> Errors {
//         let mut errors = Errors::default();
//         let component_arch = component_diagrams.to_diagram_architecture(&mut errors);
//         let class_index = ClassDiagramIndex::build_index(class_diagrams.as_slice(), &mut errors);

//         validate_component_class(&component_arch, &class_index, errors)
//     }

//     #[test]
//     fn naming_consistency_passes_for_exact_match() {
//         let component_diagrams = component_diagrams(&["unit_1", "Unit_2"]);
//         let class_diagrams = class_diagrams(&["unit_1", "Unit_2"]);

//         let errors = run_component_class_validation(&component_diagrams, &class_diagrams);

//         assert!(errors.is_empty());
//     }

//     #[test]
//     fn naming_consistency_reports_missing_and_extra() {
//         let component_diagrams = component_diagrams(&["unit_1", "unit_2", "unit_3"]);
//         let class_diagrams = class_diagrams(&["unit_2", "Unit_3"]);

//         let errors = run_component_class_validation(&component_diagrams, &class_diagrams);

//         assert!(!errors.is_empty());
//         assert_eq!(errors.messages.len(), 3);

//         let missing_count = errors
//             .messages
//             .iter()
//             .filter(|message| {
//                 message.contains("no enclosing namespace ID suffix match for component unit ID")
//             })
//             .count();
//         let unexpected_count = errors
//             .messages
//             .iter()
//             .filter(|message| message.contains("is not a suffix of any component unit ID"))
//             .count();

//         assert_eq!(missing_count, 2);
//         assert_eq!(unexpected_count, 1);
//     }

//     #[test]
//     fn entity_enclosing_namespace_ids_are_used_as_observed_namespaces() {
//         let component_diagrams = component_diagrams(&["unit_1"]);
//         let class_diagrams = class_diagrams(&["unit_1"]);

//         let errors = run_component_class_validation(&component_diagrams, &class_diagrams);
//         assert!(
//             errors.is_empty(),
//             "Expected pass when entity parent IDs match unit aliases, got: {:?}",
//             errors.messages
//         );
//     }

//     #[test]
//     fn parent_unit_aliases_are_not_prefixed_into_expected_names() {
//         let component_diagrams = component_diagrams_with_hierarchy(&[
//             ("component_1", Some("component_1"), None, "component"),
//             (
//                 "component_1.parent",
//                 Some("parent"),
//                 Some("component_1"),
//                 "unit",
//             ),
//             (
//                 "component_1.parent.child",
//                 Some("child"),
//                 Some("component_1.parent"),
//                 "unit",
//             ),
//             (
//                 "component_1.parent.child.leaf",
//                 Some("leaf"),
//                 Some("component_1.parent.child"),
//                 "unit",
//             ),
//         ]);
//         let class_diagrams = class_diagrams(&["parent", "child", "leaf"]);

//         let errors = run_component_class_validation(&component_diagrams, &class_diagrams);

//         assert!(
//             errors.is_empty(),
//             "Expected pass when namespace IDs match unit ID suffixes on boundaries, got: {:?}",
//             errors.messages
//         );
//     }

//     #[test]
//     fn suffix_matching_passes_when_namespace_ids_match_unit_id_suffixes() {
//         let component_diagrams = component_diagrams_with_hierarchy(&[
//             ("module_a.subsystem.unit_1", Some("u1"), None, "unit"),
//             ("module_b.unit_2", Some("u2"), None, "unit"),
//         ]);
//         let class_diagrams = class_diagrams(&["unit_1", "unit_2"]);

//         let errors = run_component_class_validation(&component_diagrams, &class_diagrams);
//         assert!(
//             errors.is_empty(),
//             "Expected pass when namespace IDs are suffixes of unit IDs, got: {:?}",
//             errors.messages
//         );
//     }

//     #[test]
//     fn reports_missing_when_expected_unit_id_has_no_suffix_match() {
//         let component_diagrams = component_diagrams_with_hierarchy(&[(
//             "module_a.subsystem.unit_1",
//             Some("u1"),
//             None,
//             "unit",
//         )]);
//         let class_diagrams = class_diagrams(&["unit_2"]);

//         let errors = run_component_class_validation(&component_diagrams, &class_diagrams);
//         assert!(!errors.is_empty());
//         assert_eq!(errors.messages.len(), 2);
//         assert!(errors.messages.iter().any(|message| {
//             message.contains("no enclosing namespace ID suffix match for component unit ID")
//                 && message.contains("module_a.subsystem.unit_1")
//         }));
//     }

//     #[test]
//     fn reports_unexpected_when_namespace_is_not_suffix_of_any_unit_id() {
//         let component_diagrams = component_diagrams_with_hierarchy(&[(
//             "module_a.subsystem.unit_1",
//             Some("u1"),
//             None,
//             "unit",
//         )]);
//         let class_diagrams = class_diagrams(&["unit_1", "orphan"]);

//         let errors = run_component_class_validation(&component_diagrams, &class_diagrams);
//         assert!(!errors.is_empty());
//         assert_eq!(errors.messages.len(), 1);
//         assert!(errors.messages.iter().any(|message| {
//             message.contains("is not a suffix of any component unit ID")
//                 && message.contains("Namespace ID       : \"orphan\"")
//         }));
//     }

//     #[test]
//     fn partial_suffix_without_namespace_boundary_does_not_match() {
//         let component_diagrams = component_diagrams_with_hierarchy(&[(
//             "module_a.subsystem.unit_1",
//             Some("u1"),
//             None,
//             "unit",
//         )]);
//         let class_diagrams = class_diagrams(&["it_1"]);

//         let errors = run_component_class_validation(&component_diagrams, &class_diagrams);
//         assert!(!errors.is_empty());
//         assert_eq!(errors.messages.len(), 2);
//     }
// }
