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

//! Validation: compare the indexed Bazel build graph against the indexed PlantUML diagram.
//!
//! [`Validator`] performs a two-way set-difference between a [`BazelArchitecture`]
//! and a [`DiagramArchitecture`]

use crate::models::{BazelArchitecture, BazelInput, DiagramArchitecture, DiagramInputs, Errors};

// ---------------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------------

/// Compares a [`BazelArchitecture`] and a [`DiagramArchitecture`], accumulating mismatches into
/// [`Errors`].
pub struct Validator {
    bazel: BazelArchitecture,
    diagram: DiagramArchitecture,
    errors: Errors,
}

impl Validator {
    /// Create a new `Validator` from pre-built sets and already-accumulated
    /// errors (which may include duplicates detected during set construction).
    pub fn new(bazel: BazelArchitecture, diagram: DiagramArchitecture, errors: Errors) -> Self {
        Self {
            bazel,
            diagram,
            errors,
        }
    }

    /// Run the two-way set-difference comparison and return all accumulated errors.
    ///
    /// The debug log is always built and stored in `errors.debug_output`.
    pub fn run(mut self) -> Errors {
        self.errors.debug_output = self.build_debug_log();
        self.check_seooc();
        self.check_components();
        self.check_units();
        self.errors
    }

    fn build_debug_log(&self) -> String {
        let mut log = String::new();

        log.push_str(&format!(
            "DEBUG: Found {} total diagram entities\n",
            self.diagram.entities.len()
        ));
        for e in &self.diagram.entities {
            log.push_str(&format!(
                "  Entity: id={:?}, alias={:?}, stereotype={:?}\n",
                e.id, e.alias, e.stereotype
            ));
        }
        log.push_str(&format!(
            "DEBUG: Filtered to {} SEooC packages, {} components and {} units\n",
            self.diagram.filtered_seooc_count,
            self.diagram.filtered_component_count,
            self.diagram.filtered_unit_count
        ));
        log.push_str("DEBUG: PlantUML SEooC set:\n");
        for (key, _) in &self.diagram.seooc_set {
            log.push_str(&format!("  {:?}\n", key));
        }
        log.push_str("DEBUG: PlantUML component set:\n");
        for (key, _) in &self.diagram.comp_set {
            log.push_str(&format!("  {:?}\n", key));
        }
        log.push_str("DEBUG: PlantUML unit set:\n");
        for (key, _) in &self.diagram.unit_set {
            log.push_str(&format!("  {:?}\n", key));
        }
        log.push_str("DEBUG: Bazel SEooC set:\n");
        for (key, label) in &self.bazel.seooc_set {
            log.push_str(&format!("  {:?} -> {}\n", key, label));
        }
        log.push_str("DEBUG: Bazel component set:\n");
        for (key, label) in &self.bazel.comp_set {
            log.push_str(&format!("  {:?} -> {}\n", key, label));
        }
        log.push_str("DEBUG: Bazel unit set:\n");
        for (key, label) in &self.bazel.unit_set {
            log.push_str(&format!("  {:?} -> {}\n", key, label));
        }
        log
    }

    fn check_seooc(&mut self) {
        // In Bazel but not in PlantUML → MISSING
        for (key, label) in &self.bazel.seooc_set {
            if !self.diagram.seooc_set.contains_key(key) {
                let (name, _) = key;
                self.errors.push(Self::format_missing(
                    "package",
                    "SEooC",
                    name,
                    "(top-level)",
                    label,
                ));
            }
        }
        // In PlantUML but not in Bazel → EXTRA
        for (key, _) in &self.diagram.seooc_set {
            if !self.bazel.seooc_set.contains_key(key) {
                let (name, _) = key;
                self.errors
                    .push(Self::format_extra("package", name, "(top-level)"));
            }
        }
    }

    fn check_components(&mut self) {
        // In Bazel but not in PlantUML → MISSING
        for (key, label) in &self.bazel.comp_set {
            if !self.diagram.comp_set.contains_key(key) {
                let (name, parent) = key;
                let parent_str = parent
                    .as_ref()
                    .map_or("(top-level)".to_string(), |s| s.clone());
                self.errors.push(Self::format_missing(
                    "component",
                    "component",
                    name,
                    &parent_str,
                    label,
                ));
            }
        }
        // In PlantUML but not in Bazel → EXTRA
        for (key, _) in &self.diagram.comp_set {
            if !self.bazel.comp_set.contains_key(key) {
                let (name, parent) = key;
                let parent_str = parent
                    .as_ref()
                    .map_or("(top-level)".to_string(), |s| s.clone());
                self.errors
                    .push(Self::format_extra("component", name, &parent_str));
            }
        }
    }

    fn check_units(&mut self) {
        // In Bazel but not in PlantUML → MISSING
        for (key, label) in &self.bazel.unit_set {
            if !self.diagram.unit_set.contains_key(key) {
                let (name, parent) = key;
                let parent_str = parent
                    .as_ref()
                    .map_or("(no parent?)".to_string(), |s| s.clone());
                self.errors.push(Self::format_missing(
                    "unit",
                    "unit",
                    name,
                    &parent_str,
                    label,
                ));
            }
        }
        // In PlantUML but not in Bazel → EXTRA
        for (key, _) in &self.diagram.unit_set {
            if !self.bazel.unit_set.contains_key(key) {
                let (name, parent) = key;
                let parent_str = parent
                    .as_ref()
                    .map_or("(no parent?)".to_string(), |s| s.clone());
                self.errors
                    .push(Self::format_extra("unit", name, &parent_str));
            }
        }
    }

    fn format_missing(
        display_type: &str,
        stereotype: &str,
        name: &str,
        parent_str: &str,
        label: &str,
    ) -> String {
        format!(
            "Missing {display_type} in PlantUML:\n\
               Alias          : \"{name}\"\n\
               Parent         : {parent_str}\n\
               Bazel label    : {label}\n\
               Required       : Add {display_type} with alias \"{name}\" and stereotype <<{stereotype}>>",
        )
    }

    fn format_extra(entity_type: &str, name: &str, parent_str: &str) -> String {
        format!(
            "Extra {entity_type} in PlantUML not in Bazel:\n\
               Alias          : \"{name}\"\n\
               Parent         : {parent_str}\n\
               Action         : Remove this {entity_type} or add to Bazel",
        )
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Index both sources and run the comparison.
///
/// Errors from index construction (duplicates) and from comparison (mismatches)
/// are all collected into a single [`Errors`] value.
pub fn validate(arch: &BazelInput, diagram: &DiagramInputs) -> Errors {
    let mut errors = Errors::default();
    let bazel = arch.to_bazel_architecture(&mut errors);
    let diag = diagram.to_diagram_architecture(&mut errors);
    Validator::new(bazel, diag, errors).run()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BazelInput, BazelInputEntry, DiagramInput, DiagramInputs};
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
    ) -> DiagramInput {
        DiagramInput {
            id: id.to_string(),
            alias: alias.map(|s| s.to_string()),
            parent_id: parent_id.map(|s| s.to_string()),
            stereotype: stereotype.map(|s| s.to_string()),
        }
    }

    fn diagram(entities: Vec<DiagramInput>) -> DiagramInputs {
        DiagramInputs { entities }
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
        let errs = validate(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.messages);
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
        let errs = validate(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.messages);
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
        let errs = validate(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.messages);
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
        let errs = validate(&arch, &diagram);
        assert!(!errs.is_empty());
        assert!(errs.messages.iter().any(|m| m.contains("Missing unit")));
    }

    #[test]
    fn test_duplicate_bazel_key_detected() {
        let arch = make_arch(vec![
            ("@//pkg1:comp_a", vec![], vec![]),
            ("@//pkg2:comp_a", vec![], vec![]),
        ]);
        let diagram = diagram(vec![entity("CompA", Some("comp_a"), None, Some("SEooC"))]);
        let errs = validate(&arch, &diagram);
        assert!(!errs.is_empty());
        assert!(
            errs.messages.iter().any(|m| m.contains("Duplicate")),
            "Expected duplicate error, got: {:?}",
            errs.messages
        );
    }

    #[test]
    fn test_same_short_name_different_packages_one_child() {
        // pkg1:comp_a is nested under de; pkg2:comp_a is a separate top-level element.
        // Full-label matching in child_labels ensures pkg2:comp_a is never silently
        // suppressed from the top-level (None-parent) slot.
        let arch = make_arch(vec![
            ("de", vec![], vec!["@//pkg1:comp_a"]),
            ("@//pkg1:comp_a", vec![], vec![]),
            ("@//pkg2:comp_a", vec![], vec![]),
        ]);
        // An empty diagram is sufficient to exercise the Bazel-side set building.
        let errs = validate(&arch, &diagram(vec![]));
        // No panic. Both labels are considered; a duplicate or mismatch error is expected.
        let _ = errs;
    }

    #[test]
    fn test_missing_seooc_error_mentions_seooc_stereotype() {
        let arch = make_arch(vec![("my_de", vec![], vec![])]);
        let errs = validate(&arch, &diagram(vec![]));
        assert!(!errs.is_empty());
        let msg = &errs.messages[0];
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
        let errs = validate(&arch, &diagram);
        assert!(!errs.is_empty());
        assert!(
            errs.messages.iter().any(|m| m.contains("Extra component")),
            "Expected extra component error, got: {:?}",
            errs.messages
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
        let errs = validate(&arch, &diagram);
        assert!(!errs.is_empty());
        assert!(
            errs.messages.iter().any(|m| m.contains("Extra unit")),
            "Expected extra unit error, got: {:?}",
            errs.messages
        );
    }

    #[test]
    fn test_component_with_wrong_stereotype_rejected() {
        // A <<component>> where <<SEooC>> is expected must not pass.
        let arch = make_arch(vec![("my_de", vec![], vec![])]);
        let diagram = diagram(vec![entity("MyDE", Some("my_de"), None, Some("component"))]);
        let errs = validate(&arch, &diagram);
        assert!(
            !errs.is_empty(),
            "<<component>> should not satisfy <<SEooC>> requirement"
        );
        assert!(
            errs.messages.iter().any(|m| m.contains("Missing package")),
            "Expected missing package error, got: {:?}",
            errs.messages
        );
    }

    #[test]
    fn test_seooc_where_component_expected_rejected() {
        // A <<SEooC>> where <<component>> is expected must not pass.
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            ("@//pkg:comp_a", vec![], vec![]),
        ]);
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity("CompA", Some("comp_a"), Some("MyDE"), Some("SEooC")),
        ]);
        let errs = validate(&arch, &diagram);
        assert!(
            !errs.is_empty(),
            "<<SEooC>> should not satisfy <<component>> requirement"
        );
        assert!(
            errs.messages
                .iter()
                .any(|m| m.contains("Missing component")),
            "Expected missing component error, got: {:?}",
            errs.messages
        );
    }

    #[test]
    fn test_empty_diagram_reports_all_missing() {
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            ("@//pkg:comp_a", vec!["@//pkg/u1:unit_1"], vec![]),
        ]);
        let errs = validate(&arch, &diagram(vec![]));
        assert_eq!(
            errs.messages.len(),
            3,
            "Expected 3 errors (seooc + comp + unit), got: {:?}",
            errs.messages
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
        let errs = validate(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.messages);
    }

    #[test]
    fn test_entity_without_alias_uses_id_as_key() {
        let arch = make_arch(vec![
            ("my_de", vec![], vec!["@//pkg:comp_a"]),
            ("@//pkg:comp_a", vec![], vec![]),
        ]);
        // No alias set — match_key() should fall back to id (lowercased).
        let diagram = diagram(vec![
            entity("my_de", None, None, Some("SEooC")),
            entity("comp_a", None, Some("my_de"), Some("component")),
        ]);
        let errs = validate(&arch, &diagram);
        assert!(errs.is_empty(), "Expected pass, got: {:?}", errs.messages);
    }

    #[test]
    fn test_duplicate_diagram_id_detected() {
        let arch = make_arch(vec![("my_de", vec![], vec![])]);
        // Two entities with the same id (case-insensitive).
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity("myDE", Some("other_alias"), None, Some("component")),
        ]);
        let errs = validate(&arch, &diagram);
        assert!(
            errs.messages
                .iter()
                .any(|m| m.contains("Duplicate entity ID")),
            "Expected duplicate ID error, got: {:?}",
            errs.messages
        );
    }

    #[test]
    fn test_orphaned_parent_id_detected() {
        let arch = make_arch(vec![("my_de", vec![], vec![])]);
        // Entity references a parent_id that doesn't exist in the diagram.
        let diagram = diagram(vec![
            entity("MyDE", Some("my_de"), None, Some("SEooC")),
            entity(
                "CompA",
                Some("comp_a"),
                Some("NonExistent"),
                Some("component"),
            ),
        ]);
        let errs = validate(&arch, &diagram);
        assert!(
            errs.messages
                .iter()
                .any(|m| m.contains("Unresolved parent_id")),
            "Expected unresolved parent error, got: {:?}",
            errs.messages
        );
    }
}
