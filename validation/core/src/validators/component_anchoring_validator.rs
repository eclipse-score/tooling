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

//! Validation: every `<<component>>` and `<<unit>>` of the merged component
//! diagram is nested directly under a `<<SEooC>>` or a `<<component>>`.
//! An unanchored element means an overview and a detail declaration did not
//! resolve onto one id. Interfaces and unstereotyped elements are exempt.

use std::collections::BTreeMap;

use crate::models::{ComponentDiagramArchitecture, LogicComponent, LogicComponentExt};
use crate::results::{ErrorBuilder, ErrorCategory};
use crate::ValidationResult;

/// Run the component anchoring validation.
pub fn validate_component_anchoring(
    component_diagram: &ComponentDiagramArchitecture,
) -> ValidationResult {
    let entities = &component_diagram.entities;
    let by_id: BTreeMap<String, &LogicComponent> = entities
        .iter()
        .map(|entity| (entity.id.to_lowercase(), entity))
        .collect();

    let mut result = ValidationResult::default();
    for entity in entities {
        if !(entity.is_component() || entity.is_unit()) {
            continue;
        }
        let Some(parent_id) = &entity.parent_id else {
            result.add_failure(format_error(
                entity,
                entities,
                "is not nested under a <<SEooC>> package",
                None,
            ));
            continue;
        };
        // A parent missing from the model is reported when the sets are built.
        let Some(parent) = by_id.get(&parent_id.to_lowercase()) else {
            continue;
        };
        if !(parent.is_seooc_package() || parent.is_component()) {
            result.add_failure(format_error(
                entity,
                entities,
                "is nested under an entity that is neither a <<SEooC>> nor a <<component>>",
                Some(parent),
            ));
        }
    }
    result
}

fn kind_name(entity: &LogicComponent) -> &'static str {
    if entity.is_unit() {
        "unit"
    } else {
        "component"
    }
}

fn format_error(
    entity: &LogicComponent,
    entities: &[LogicComponent],
    reason: &str,
    parent: Option<&LogicComponent>,
) -> String {
    let kind = kind_name(entity);
    let name = entity.match_key();
    let (source_file, source_line) = entity.source_location.display();
    let mut error = ErrorBuilder::new(ErrorCategory::Design)
        .title(format!("{kind} \"{name}\" {reason}"))
        .field(kind, format!("\"{name}\""))
        .field("component source file", format!("\"{source_file}\""))
        .field("component source line", source_line.to_string());
    if let Some(parent) = parent {
        error = error.field("parent", format!("\"{}\"", parent.id));
    }

    // Nearest other declaration of the same name, in a stable order.
    let similar = entities
        .iter()
        .filter(|candidate| candidate.id != entity.id && candidate.match_key() == name)
        .min_by_key(|candidate| candidate.source_location.display());
    if let Some(similar) = similar {
        let (similar_file, similar_line) = similar.source_location.display();
        error = error
            .field("similar declaration", format!("\"{}\"", similar.id))
            .field("similar source file", format!("\"{similar_file}\""))
            .field("similar source line", similar_line.to_string());
    }

    error
        .fix(format!(
            "nest {kind} \"{name}\" under a <<SEooC>> or <<component>>; when it is the detail of a declaration in another static file, give it a body and the same name, element type and stereotype"
        ))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ComponentDiagramInputs, ComponentType};
    use source_location::SourceLocation;

    fn entity(
        id: &str,
        element_type: ComponentType,
        stereotype: Option<&str>,
        file: &str,
        line: u32,
    ) -> LogicComponent {
        let parent_id = id.rsplit_once('.').map(|(parent, _)| parent.to_string());
        LogicComponent {
            id: id.to_string(),
            name: None,
            alias: None,
            parent_id,
            element_type,
            stereotype: stereotype.map(str::to_string),
            relations: Vec::new(),
            source_location: SourceLocation::new(file, line),
        }
    }

    fn component(id: &str, stereotype: &str, file: &str, line: u32) -> LogicComponent {
        entity(id, ComponentType::Component, Some(stereotype), file, line)
    }

    fn run(entities: Vec<LogicComponent>) -> Vec<String> {
        let mut result = ValidationResult::default();
        let architecture = ComponentDiagramInputs { entities }.to_diagram_architecture(&mut result);
        validate_component_anchoring(&architecture).failures
    }

    #[test]
    fn accepts_units_and_components_under_seooc_and_components() {
        let failures = run(vec![
            component("de", "SEooC", "a.puml", 1),
            component("de.comp", "component", "a.puml", 2),
            component("de.comp.unit_a", "unit", "a.puml", 3),
            component("de.unit_b", "unit", "a.puml", 4),
        ]);
        assert!(failures.is_empty(), "unexpected failures: {failures:?}");
    }

    #[test]
    fn exempts_top_level_interfaces_and_unstereotyped_elements() {
        let failures = run(vec![
            component("de", "SEooC", "a.puml", 1),
            entity("PublicApi", ComponentType::Interface, None, "a.puml", 2),
            entity("external", ComponentType::Component, None, "a.puml", 3),
        ]);
        assert!(failures.is_empty(), "unexpected failures: {failures:?}");
    }

    #[test]
    fn reports_top_level_component() {
        let failures = run(vec![component("comp", "component", "detail.puml", 7)]);
        assert_eq!(failures.len(), 1, "got: {failures:?}");
        assert!(failures[0].contains("Component \"comp\" is not nested under a <<SEooC>> package"));
        assert!(failures[0].contains("detail.puml"));
    }

    #[test]
    fn reports_top_level_unit() {
        let failures = run(vec![component("unit_a", "unit", "detail.puml", 3)]);
        assert_eq!(failures.len(), 1, "got: {failures:?}");
        assert!(failures[0].contains("Unit \"unit_a\" is not nested under a <<SEooC>> package"));
    }

    #[test]
    fn reports_component_under_unstereotyped_package() {
        let failures = run(vec![
            entity("pkg", ComponentType::Package, None, "a.puml", 1),
            component("pkg.comp", "component", "a.puml", 2),
        ]);
        assert_eq!(failures.len(), 1, "got: {failures:?}");
        assert!(failures[0].contains("neither a <<SEooC>> nor a <<component>>"));
        assert!(failures[0].contains("\"pkg\""));
    }

    #[test]
    fn reports_component_under_unit() {
        let failures = run(vec![
            component("de", "SEooC", "a.puml", 1),
            component("de.unit_a", "unit", "a.puml", 2),
            component("de.unit_a.comp", "component", "a.puml", 3),
        ]);
        assert_eq!(failures.len(), 1, "got: {failures:?}");
        assert!(failures[0].contains("component \"comp\""));
    }

    #[test]
    fn reports_failed_reparent_once_with_similar_declaration() {
        // Detail did not merge into the overview's nested declaration.
        let failures = run(vec![
            component("de", "SEooC", "overview.puml", 1),
            component("de.comp", "component", "overview.puml", 2),
            component("comp", "component", "detail.puml", 5),
            component("comp.unit_a", "unit", "detail.puml", 6),
        ]);
        assert_eq!(failures.len(), 1, "got: {failures:?}");
        assert!(failures[0].contains("Component \"comp\" is not nested"));
        assert!(failures[0].contains("\"de.comp\""));
        assert!(failures[0].contains("overview.puml"));
    }

    #[test]
    fn leaves_dangling_parent_to_the_model() {
        let failures = run(vec![component("ghost.unit_a", "unit", "a.puml", 1)]);
        assert!(
            failures.iter().all(|f| !f.contains("not nested")),
            "got: {failures:?}"
        );
    }
}
