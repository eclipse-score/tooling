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

//! Validation: compare the indexed Bazel build graph against the indexed
//! PlantUML component diagram.
//!
//! [`BazelComponentValidator`] performs a two-way set-difference between a
//! [`BazelArchitecture`] and a [`ComponentDiagramArchitecture`].

use std::collections::{BTreeMap, BTreeSet};

use crate::models::{BazelArchitecture, ComponentDiagramArchitecture, LogicComponent};
use crate::results::{ErrorBuilder, ErrorCategory};
use crate::{Diagnostics, ValidationResult};

/// Run bazel-vs-component architecture validation using indexed inputs.
pub fn validate_bazel_component(
    bazel: &BazelArchitecture,
    diagram: &ComponentDiagramArchitecture,
) -> ValidationResult {
    BazelComponentValidator::new().run(bazel, diagram)
}

/// Compares a [`BazelArchitecture`] and a [`ComponentDiagramArchitecture`],
/// accumulating mismatches into [`ValidationResult`].
pub struct BazelComponentValidator {
    result: ValidationResult,
}

impl BazelComponentValidator {
    /// Create a new [`BazelComponentValidator`] from pre-built sets and already
    /// accumulated indexing result.
    pub fn new() -> Self {
        Self {
            result: ValidationResult::default(),
        }
    }

    /// Run the two-way set-difference comparison and return all accumulated
    /// result.
    ///
    pub fn run(
        mut self,
        bazel: &BazelArchitecture,
        diagram: &ComponentDiagramArchitecture,
    ) -> ValidationResult {
        append_debug_log(&mut self.result.diagnostics, bazel, diagram);
        self.check_entity_set(
            &bazel.seooc_set,
            &diagram.seooc_set,
            "package",
            "SEooC",
            "(top-level)",
        );
        self.check_entity_set(
            &bazel.comp_set,
            &diagram.comp_set,
            "component",
            "component",
            "(top-level)",
        );
        self.check_entity_set(
            &bazel.unit_set,
            &diagram.unit_set,
            "unit",
            "unit",
            "(no parent?)",
        );
        self.check_non_empty_composition(bazel);
        self.result
    }

    fn check_entity_set(
        &mut self,
        bazel_set: &BTreeMap<(String, Option<String>), String>,
        diagram_set: &BTreeMap<(String, Option<String>), LogicComponent>,
        display_type: &str,
        stereotype: &str,
        default_parent: &str,
    ) {
        // In Bazel but not in PlantUML -> MISSING.
        for (key, label) in bazel_set {
            if !diagram_set.contains_key(key) {
                let (name, parent) = key;
                let parent_str = Self::parent_display(parent, default_parent);
                self.result.add_failure(Self::format_missing(
                    display_type,
                    stereotype,
                    name,
                    &parent_str,
                    label,
                ));
            }
        }

        // In PlantUML but not in Bazel -> EXTRA.
        for (key, entity) in diagram_set {
            if !bazel_set.contains_key(key) {
                let (name, parent) = key;
                let parent_str = Self::parent_display(parent, default_parent);
                self.result.add_failure(Self::format_extra(
                    display_type,
                    name,
                    &parent_str,
                    entity,
                ));
            }
        }
    }

    fn parent_display(parent: &Option<String>, default_parent: &str) -> String {
        parent
            .as_ref()
            .map_or(default_parent.to_string(), |value| value.clone())
    }

    /// Verify every Bazel `dependable_element`/`component` target decomposes
    /// into at least one nested `component` or `unit`.
    ///
    /// A childless entry (both `units` and `components` empty in the
    /// architecture JSON) can silently agree with an equally childless
    /// PlantUML entity, so the set-difference checks above never flag it.
    /// This scans the Bazel side directly instead, since it reflects what is
    /// actually built.
    fn check_non_empty_composition(&mut self, bazel: &BazelArchitecture) {
        let mut parents_with_children: BTreeSet<String> = BTreeSet::new();
        for (_, parent) in bazel.comp_set.keys() {
            if let Some(parent) = parent {
                parents_with_children.insert(parent.clone());
            }
        }
        for (_, parent) in bazel.unit_set.keys() {
            if let Some(parent) = parent {
                parents_with_children.insert(parent.clone());
            }
        }

        for (key, label) in &bazel.seooc_set {
            let (name, _) = key;
            if !parents_with_children.contains(name) {
                self.result
                    .add_failure(Self::format_empty("dependable element", name, label));
            }
        }
        for (key, label) in &bazel.comp_set {
            let (name, _) = key;
            if !parents_with_children.contains(name) {
                self.result
                    .add_failure(Self::format_empty("component", name, label));
            }
        }
    }

    fn format_empty(display_type: &str, name: &str, label: &str) -> String {
        format!(
            "Empty {display_type} in Bazel build graph:\n\
               Alias          : \"{name}\"\n\
               Bazel label    : {label}\n\
               Required       : A {display_type} must decompose into at least one nested unit or component; add one or remove the empty {display_type}",
        )
    }

    fn format_missing(
        display_type: &str,
        stereotype: &str,
        name: &str,
        parent_str: &str,
        label: &str,
    ) -> String {
        ErrorBuilder::new(ErrorCategory::Naming)
            .title(format!(
                "{display_type} \"{name}\" from Bazel not found in the PlantUML component diagram"
            ))
            .field("alias", format!("\"{name}\""))
            .field("parent", parent_str)
            .field("stereotype", format!("<<{stereotype}>>"))
            .field("bazel label", label)
            .fix(format!(
                "add {display_type} \"{name}\" with stereotype <<{stereotype}>> in the PlantUML component diagram, or remove it from Bazel"
            ))
            .build()
    }

    fn format_extra(
        entity_type: &str,
        name: &str,
        parent_str: &str,
        entity: &LogicComponent,
    ) -> String {
        let (source_file, source_line) = entity.source_location.display();

        ErrorBuilder::new(ErrorCategory::Naming)
            .title(format!(
                "{entity_type} \"{name}\" from the PlantUML component diagram not found in Bazel"
            ))
            .field("alias", format!("\"{name}\""))
            .field("parent", parent_str)
            .field("component source file", format!("\"{source_file}\""))
            .field("component source line", source_line.to_string())
            .fix(format!(
                "add the corresponding Bazel {entity_type} definition for \"{name}\", or remove it from the PlantUML component diagram"
            ))
            .build()
    }
}

fn append_debug_log(
    diagnostics: &mut Diagnostics,
    bazel: &BazelArchitecture,
    diagram: &ComponentDiagramArchitecture,
) {
    diagnostics.debug(|| format!("Found {} total diagram entities", diagram.entities.len()));
    for entity in &diagram.entities {
        diagnostics.debug(|| {
            format!(
                "Entity: id={:?}, alias={:?}, stereotype={:?}",
                entity.id, entity.alias, entity.stereotype
            )
        });
    }
    diagnostics.debug(|| {
        format!(
            "Filtered to {} SEooC packages, {} components and {} units",
            diagram.filtered_seooc_count,
            diagram.filtered_component_count,
            diagram.filtered_unit_count
        )
    });
    diagnostics.debug(|| "PlantUML SEooC set:".to_string());
    for key in diagram.seooc_set.keys() {
        diagnostics.debug(|| format!("  {:?}", key));
    }
    diagnostics.debug(|| "PlantUML component set:".to_string());
    for key in diagram.comp_set.keys() {
        diagnostics.debug(|| format!("  {:?}", key));
    }
    diagnostics.debug(|| "PlantUML unit set:".to_string());
    for key in diagram.unit_set.keys() {
        diagnostics.debug(|| format!("  {:?}", key));
    }
    diagnostics.debug(|| "Bazel SEooC set:".to_string());
    for (key, label) in &bazel.seooc_set {
        diagnostics.debug(|| format!("  {:?} -> {}", key, label));
    }
    diagnostics.debug(|| "Bazel component set:".to_string());
    for (key, label) in &bazel.comp_set {
        diagnostics.debug(|| format!("  {:?} -> {}", key, label));
    }
    diagnostics.debug(|| "Bazel unit set:".to_string());
    for (key, label) in &bazel.unit_set {
        diagnostics.debug(|| format!("  {:?} -> {}", key, label));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        BazelInput, BazelInputEntry, ComponentDiagramInputs, ComponentType, LogicComponent,
    };
    use crate::validators::fixtures::dummy_source_location;
    use std::collections::BTreeMap;

    fn make_arch(entries: Vec<(&str, Vec<&str>, Vec<&str>)>) -> BazelInput {
        let mut components = BTreeMap::new();
        for (label, units, nested) in entries {
            components.insert(
                label.to_string(),
                BazelInputEntry {
                    units: units.into_iter().map(|s| s.to_string()).collect(),
                    components: nested.into_iter().map(|s| s.to_string()).collect(),
                },
            );
        }
        BazelInput { components }
    }

    fn entity(
        id: &str,
        alias: Option<&str>,
        parent_id: Option<&str>,
        stereotype: Option<&str>,
    ) -> LogicComponent {
        let element_type = if stereotype == Some("SEooC") {
            ComponentType::Package
        } else {
            ComponentType::Component
        };

        LogicComponent {
            id: id.to_string(),
            name: alias.map(|s| s.to_string()),
            alias: alias.map(|s| s.to_string()),
            parent_id: parent_id.map(|s| s.to_string()),
            element_type,
            stereotype: stereotype.map(|s| s.to_string()),
            relations: Vec::new(),
            source_location: dummy_source_location(),
        }
    }

    fn diagram(entities: Vec<LogicComponent>) -> ComponentDiagramInputs {
        ComponentDiagramInputs { entities }
    }

    fn run_arch_validation(
        arch: &BazelInput,
        diagram: &ComponentDiagramInputs,
    ) -> ValidationResult {
        let mut result = ValidationResult::default();
        let bazel = arch.to_bazel_architecture(&mut result);
        let diag = diagram.to_diagram_architecture(&mut result);
        result.merge(validate_bazel_component(&bazel, &diag));
        result
    }

    #[test]
    fn test_component_and_unit_match() {
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            ("@//pkg:comp_a", vec!["@//pkg/u1:unit_1"], vec![]),
        ]);
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity("CompA", Some("comp_a"), Some("MyDE"), Some("component")),
            entity("CompA.Unit1", Some("unit_1"), Some("CompA"), Some("unit")),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.failures);
    }

    #[test]
    fn test_seooc_package_matches_dependable_element() {
        let arch = make_arch(vec![
            (
                "safety_software_seooc_example",
                vec![],
                vec!["@//bazel/rules/rules_score/examples/seooc:component_example"],
            ),
            (
                "@//bazel/rules/rules_score/examples/seooc:component_example",
                vec![
                    "@//bazel/rules/rules_score/examples/seooc/unit_1:unit_1",
                    "@//bazel/rules/rules_score/examples/seooc/unit_2:unit_2",
                ],
                vec![],
            ),
        ]);
        let diagram = diagram(vec![
            entity(
                "SampleSeooc",
                Some("safety_software_seooc_example"),
                None,
                Some("SEooC"),
            ),
            entity(
                "ComponentExample",
                Some("component_example"),
                Some("SampleSeooc"),
                Some("component"),
            ),
            entity(
                "Unit1",
                Some("unit_1"),
                Some("ComponentExample"),
                Some("unit"),
            ),
            entity(
                "Unit2",
                Some("unit_2"),
                Some("ComponentExample"),
                Some("unit"),
            ),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.failures);
    }

    #[test]
    fn test_units_with_unique_names() {
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            (
                "@//pkg:comp_a",
                vec!["@//pkg/u1:unit_1", "@//pkg/u2:unit_2"],
                vec![],
            ),
        ]);
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity("CompA", Some("comp_a"), Some("MyDE"), Some("component")),
            entity("CompA.Unit1", Some("unit_1"), Some("CompA"), Some("unit")),
            entity("CompA.Unit2", Some("unit_2"), Some("CompA"), Some("unit")),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.failures);
    }

    #[test]
    fn test_missing_unit_detected() {
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            (
                "@//pkg:comp_a",
                vec!["@//pkg/u1:unit_1", "@//pkg/u2:unit_2"],
                vec![],
            ),
        ]);
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity("CompA", Some("comp_a"), Some("MyDE"), Some("component")),
            entity("CompA.Unit1", Some("unit_1"), Some("CompA"), Some("unit")),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(!errs.is_empty());
        assert!(errs.failures.iter().any(|m| {
            m.contains("Unit \"unit_2\" from Bazel not found in the PlantUML component diagram.")
        }));
    }

    #[test]
    fn test_same_short_name_different_packages_one_child() {
        let arch = make_arch(vec![
            ("de", vec![], vec!["@//pkg1:comp_a"]),
            ("@//pkg1:comp_a", vec![], vec![]),
            ("@//pkg2:comp_a", vec![], vec![]),
        ]);
        let errs = run_arch_validation(&arch, &diagram(vec![]));
        let _ = errs;
    }

    #[test]
    fn test_missing_seooc_error_mentions_seooc_stereotype() {
        let arch = make_arch(vec![("my_de", vec![], vec![])]);
        let errs = run_arch_validation(&arch, &diagram(vec![]));
        assert!(!errs.is_empty());
        let msg = &errs.failures[0];
        assert!(
            msg.contains("SEooC"),
            "Expected error to mention SEooC stereotype, got: {msg}"
        );
    }

    #[test]
    fn test_extra_component_in_plantuml_detected() {
        let arch = make_arch(vec![("my_de", vec![], vec![])]);
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity(
                "ExtraComp",
                Some("extra_comp"),
                Some("MyDE"),
                Some("component"),
            ),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(!errs.is_empty());
        assert!(
            errs.failures.iter().any(|m| m.contains(
                "Component \"extra_comp\" from the PlantUML component diagram not found in Bazel."
            )),
            "Expected extra component error, got: {:?}",
            errs.failures
        );
    }

    #[test]
    fn test_extra_unit_in_plantuml_detected() {
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            ("@//pkg:comp_a", vec![], vec![]),
        ]);
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity("CompA", Some("comp_a"), Some("MyDE"), Some("component")),
            entity("ExtraUnit", Some("extra_unit"), Some("CompA"), Some("unit")),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(!errs.is_empty());
        assert!(
            errs.failures.iter().any(|m| m.contains(
                "Unit \"extra_unit\" from the PlantUML component diagram not found in Bazel."
            )),
            "Expected extra unit error, got: {:?}",
            errs.failures
        );
    }

    #[test]
    fn test_component_with_wrong_stereotype_rejected() {
        let arch = make_arch(vec![("my_de", vec![], vec![])]);
        let diagram = diagram(vec![entity("MyDE", Some("my_de"), None, Some("component"))]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(
            !errs.is_empty(),
            "<<component>> should not satisfy <<SEooC>> requirement"
        );
        assert!(
            errs.failures.iter().any(|m| m.contains(
                "Package \"my_de\" from Bazel not found in the PlantUML component diagram."
            )),
            "Expected missing package error, got: {:?}",
            errs.failures
        );
    }

    #[test]
    fn test_seooc_where_component_expected_rejected() {
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            ("@//pkg:comp_a", vec![], vec![]),
        ]);
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity("CompA", Some("comp_a"), Some("MyDE"), Some("SEooC")),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(
            !errs.is_empty(),
            "<<SEooC>> should not satisfy <<component>> requirement"
        );
        assert!(
            errs.failures.iter().any(|m| m.contains(
                "Component \"comp_a\" from Bazel not found in the PlantUML component diagram."
            )),
            "Expected missing component error, got: {:?}",
            errs.failures
        );
    }

    #[test]
    fn test_empty_diagram_reports_all_missing() {
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            ("@//pkg:comp_a", vec!["@//pkg/u1:unit_1"], vec![]),
        ]);
        let errs = run_arch_validation(&arch, &diagram(vec![]));
        assert_eq!(
            errs.failures.len(),
            3,
            "Expected 3 errors (seooc + comp + unit), got: {:?}",
            errs.failures
        );
    }

    #[test]
    fn test_case_insensitive_matching() {
        let arch = make_arch(vec![
            ("My_DE", vec![], vec!["@//pkg:Comp_A"]),
            ("@//pkg:Comp_A", vec!["@//pkg/u1:Unit_1"], vec![]),
        ]);
        let diagram = diagram(vec![
            entity("MyDE", Some("MY_DE"), None, Some("SEooC")),
            entity("CompA", Some("COMP_A"), Some("MyDE"), Some("component")),
            entity("Unit1", Some("UNIT_1"), Some("CompA"), Some("unit")),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.failures);
    }

    #[test]
    fn test_entity_without_alias_uses_id_as_key() {
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            ("@//pkg:comp_a", vec!["@//pkg/u1:unit_1"], vec![]),
        ]);
        let diagram = diagram(vec![
            entity("my_de", None, None, Some("SEooC")),
            entity("comp_a", None, Some("my_de"), Some("component")),
            entity("unit_1", None, Some("comp_a"), Some("unit")),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.failures);
    }

    #[test]
    fn test_empty_component_detected() {
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            ("@//pkg:comp_a", vec![], vec![]),
        ]);
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity("CompA", Some("comp_a"), Some("MyDE"), Some("component")),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(
            errs.failures.iter().any(|m| m.contains("Empty component")),
            "Expected empty component error, got: {:?}",
            errs.failures
        );
    }

    #[test]
    fn test_empty_dependable_element_detected() {
        let arch = make_arch(vec![("my_de", vec![], vec![])]);
        let diagram = diagram(vec![entity("MyDE", Some("my_de"), None, Some("SEooC"))]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(
            errs.failures
                .iter()
                .any(|m| m.contains("Empty dependable element")),
            "Expected empty dependable element error, got: {:?}",
            errs.failures
        );
    }

    #[test]
    fn test_component_with_nested_component_not_flagged_empty() {
        // "bindings" has no direct units but decomposes into a nested
        // component ("lola_component") - it must not be reported as empty.
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:bindings"]),
            ("@//pkg:bindings", vec![], vec!["@//pkg:lola_component"]),
            ("@//pkg:lola_component", vec!["@//pkg/u1:unit_1"], vec![]),
        ]);
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity(
                "Bindings",
                Some("bindings"),
                Some("MyDE"),
                Some("component"),
            ),
            entity(
                "LolaComponent",
                Some("lola_component"),
                Some("Bindings"),
                Some("component"),
            ),
            entity("Unit1", Some("unit_1"), Some("LolaComponent"), Some("unit")),
        ]);
        let errs = run_arch_validation(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.failures);
    }
}
