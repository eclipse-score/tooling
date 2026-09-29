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

//! Cross-file `static` merge policy for [`super::ComponentDiagramArchitecture`]:
//! benign re-declaration merging, conflicting re-declaration detection, and
//! single-home decomposition.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::{entity_kind_name, LogicComponent, LogicComponentExt};
use crate::{ErrorBuilder, ErrorCategory, ValidationResult};

/// Merges relations from a repeated declaration into the already-indexed entity.
///
/// Compares structurally, ignoring `source_location` (unlike `PartialEq`), so
/// a relation re-declared in another file isn't treated as new.
pub(super) fn merge_relations(existing: &mut LogicComponent, incoming: &LogicComponent) {
    for relation in &incoming.relations {
        let already_present = existing.relations.iter().any(|r| {
            r.target == relation.target
                && r.relation_type == relation.relation_type
                && r.source_role == relation.source_role
                && r.annotation == relation.annotation
        });
        if !already_present {
            existing.relations.push(relation.clone());
        }
    }
}

/// Returns the first field on which two same-id declarations disagree, or
/// `None` if they may be merged.
///
/// `alias`/`parent` aren't compared: the same `id` already implies both match.
pub(super) fn conflicting_declaration_field(
    prev: &LogicComponent,
    entity: &LogicComponent,
) -> Option<&'static str> {
    if prev.name != entity.name {
        return Some("display name");
    }
    if prev.stereotype != entity.stereotype {
        return Some("stereotype");
    }
    if prev.element_type != entity.element_type {
        return Some("element type");
    }
    None
}

pub(super) fn format_conflicting_declaration_error(
    prev: &LogicComponent,
    entity: &LogicComponent,
    field: &'static str,
) -> String {
    // Order by source location so the message is stable regardless of file order.
    let (first, second) = ordered_declarations(prev, entity);
    let kind = entity_kind_name(first);
    let alias = first.display_key();
    let (source_file, source_line) = first.source_location.display();
    let (conflicting_file, conflicting_line) = second.source_location.display();
    ErrorBuilder::new(ErrorCategory::Design)
        .title(format!(
            "{kind} \"{alias}\" is re-declared with a conflicting {field} in another component diagram file"
        ))
        .field(kind, format!("\"{alias}\""))
        .field("conflicting field", field)
        .field("component source file", format!("\"{source_file}\""))
        .field("component source line", source_line.to_string())
        .field("conflicting source file", format!("\"{conflicting_file}\""))
        .field("conflicting source line", conflicting_line.to_string())
        .fix(format!(
            "make every declaration of \"{alias}\" across all static diagrams agree on {field}, or rename one of the conflicting entities"
        ))
        .build()
}

/// Fails when an entity's children are declared across more than one file
/// with no single file containing the full set.
///
/// Interfaces are exempt since they aren't compared against the Bazel build graph.
pub(super) fn check_single_home_decomposition(
    entities: &[LogicComponent],
    result: &mut ValidationResult,
) {
    // parent id (lowercased) -> file -> child id (lowercased) -> sample entity
    let mut children_by_parent: BTreeMap<
        String,
        BTreeMap<String, BTreeMap<String, &LogicComponent>>,
    > = BTreeMap::new();
    for entity in entities {
        if entity.is_interface() {
            continue;
        }
        let Some(parent_id) = &entity.parent_id else {
            continue;
        };
        let (file, _) = entity.source_location.display();
        children_by_parent
            .entry(parent_id.to_lowercase())
            .or_default()
            .entry(file)
            .or_default()
            .insert(entity.id.to_lowercase(), entity);
    }

    for (parent_key, by_file) in &children_by_parent {
        if by_file.len() <= 1 {
            continue;
        }

        let all_child_count = by_file
            .values()
            .flat_map(|children| children.keys())
            .collect::<BTreeSet<_>>()
            .len();
        let has_single_home_file = by_file
            .values()
            .any(|children| children.len() == all_child_count);
        if has_single_home_file {
            continue;
        }

        let parent_alias = entities
            .iter()
            .find(|entity| entity.id.to_lowercase() == *parent_key)
            .map(LogicComponentExt::display_key)
            .unwrap_or_else(|| parent_key.clone());

        // Report per file so a benign subset re-declaration isn't shown as a duplicate.
        let mut error = ErrorBuilder::new(ErrorCategory::Design)
            .title(format!(
                "entity \"{parent_alias}\" has children declared across more than one component diagram file, with no single file containing all of them"
            ))
            .field("entity", format!("\"{parent_alias}\""));
        for (file, children) in by_file {
            let children_display = children
                .values()
                .map(|child| format!("\"{}\"", child.display_key()))
                .collect::<Vec<_>>()
                .join(", ");
            error = error.field(format!("children in \"{file}\""), children_display);
        }
        result.add_failure(
            error
                .fix(format!(
                    "declare the full decomposition of \"{parent_alias}\" in a single file; other files may re-declare \"{parent_alias}\" with a subset of its already-declared children (or none), but must not add children missing from every other file"
                ))
                .build(),
        );
    }
}

/// Orders two declarations by source location, independent of insertion order.
fn ordered_declarations<'a>(
    left: &'a LogicComponent,
    right: &'a LogicComponent,
) -> (&'a LogicComponent, &'a LogicComponent) {
    let left_display = left.source_location.display();
    let right_display = right.source_location.display();

    if (left_display.0.as_str(), left_display.1) <= (right_display.0.as_str(), right_display.1) {
        (left, right)
    } else {
        (right, left)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        ComponentDiagramInputs, ComponentRelationType, ComponentType, EndpointRole, LogicRelation,
    };
    use super::*;
    use crate::validators::fixtures::dummy_source_location;
    use crate::ValidationResult;

    fn relation(target: &str) -> LogicRelation {
        LogicRelation {
            target: target.to_string(),
            annotation: None,
            relation_type: ComponentRelationType::Association,
            source_role: EndpointRole::None,
            source_location: dummy_source_location(),
        }
    }

    fn entity_in_file(
        id: &str,
        alias: Option<&str>,
        parent_id: Option<&str>,
        element_type: ComponentType,
        stereotype: Option<&str>,
        relations: Vec<LogicRelation>,
        file: &str,
    ) -> LogicComponent {
        LogicComponent {
            id: id.to_string(),
            name: alias.map(str::to_string),
            alias: alias.map(str::to_string),
            parent_id: parent_id.map(str::to_string),
            element_type,
            stereotype: stereotype.map(str::to_string),
            relations,
            source_location: source_location::SourceLocation::new(file, 1),
        }
    }

    #[test]
    fn merges_benign_redeclaration_of_same_entity_across_files() {
        let inputs = ComponentDiagramInputs {
            entities: vec![
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    vec![relation("iface_x")],
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    vec![relation("iface_y")],
                    "overview.puml",
                ),
            ],
        };

        let mut result = ValidationResult::default();
        let architecture = inputs.to_diagram_architecture(&mut result);

        assert!(
            result.is_empty(),
            "expected no failures, got {:?}",
            result.failures
        );
        let merged = architecture
            .comp_set
            .get(&("comp_a".to_string(), None))
            .expect("expected merged component entry");
        let mut targets: Vec<&str> = merged
            .relations
            .iter()
            .map(|relation| relation.target.as_str())
            .collect();
        targets.sort_unstable();
        assert_eq!(targets, vec!["iface_x", "iface_y"]);
    }

    #[test]
    fn merges_relation_repeated_across_files_only_once() {
        let inputs = ComponentDiagramInputs {
            entities: vec![
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    vec![relation("iface_x")],
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    vec![relation("iface_x")],
                    "overview.puml",
                ),
            ],
        };

        let mut result = ValidationResult::default();
        let architecture = inputs.to_diagram_architecture(&mut result);

        assert!(
            result.is_empty(),
            "expected no failures, got {:?}",
            result.failures
        );
        let merged = architecture
            .comp_set
            .get(&("comp_a".to_string(), None))
            .expect("expected merged component entry");
        assert_eq!(merged.relations.len(), 1);
    }

    #[test]
    fn reports_conflicting_element_type() {
        let inputs = ComponentDiagramInputs {
            entities: vec![
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Interface,
                    Some("component"),
                    Vec::new(),
                    "overview.puml",
                ),
            ],
        };

        let mut result = ValidationResult::default();
        let _architecture = inputs.to_diagram_architecture(&mut result);

        assert!(
            result
                .failures
                .iter()
                .any(|message| message.contains("is re-declared with a conflicting element type")),
            "Expected conflicting element type error, got: {:?}",
            result.failures
        );
    }

    #[test]
    fn reports_conflicting_stereotype() {
        let inputs = ComponentDiagramInputs {
            entities: vec![
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("unit"),
                    Vec::new(),
                    "overview.puml",
                ),
            ],
        };

        let mut result = ValidationResult::default();
        let _architecture = inputs.to_diagram_architecture(&mut result);

        assert!(
            result
                .failures
                .iter()
                .any(|message| message.contains("is re-declared with a conflicting stereotype")),
            "Expected conflicting stereotype error, got: {:?}",
            result.failures
        );
    }

    #[test]
    fn reports_conflicting_display_name() {
        let mut first = entity_in_file(
            "comp_a",
            Some("comp_a"),
            None,
            ComponentType::Component,
            Some("component"),
            Vec::new(),
            "detail.puml",
        );
        first.name = Some("Component A".to_string());
        let mut second = entity_in_file(
            "comp_a",
            Some("comp_a"),
            None,
            ComponentType::Component,
            Some("component"),
            Vec::new(),
            "overview.puml",
        );
        second.name = Some("Component A (renamed)".to_string());

        let inputs = ComponentDiagramInputs {
            entities: vec![first, second],
        };

        let mut result = ValidationResult::default();
        let _architecture = inputs.to_diagram_architecture(&mut result);

        assert!(
            result
                .failures
                .iter()
                .any(|message| message.contains("is re-declared with a conflicting display name")),
            "Expected conflicting display name error, got: {:?}",
            result.failures
        );
    }

    #[test]
    fn reports_children_declared_across_multiple_files() {
        let inputs = ComponentDiagramInputs {
            entities: vec![
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                    "overview.puml",
                ),
                entity_in_file(
                    "comp_a.unit_1",
                    Some("unit_1"),
                    Some("comp_a"),
                    ComponentType::Component,
                    Some("unit"),
                    Vec::new(),
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a.unit_2",
                    Some("unit_2"),
                    Some("comp_a"),
                    ComponentType::Component,
                    Some("unit"),
                    Vec::new(),
                    "overview.puml",
                ),
            ],
        };

        let mut result = ValidationResult::default();
        let _architecture = inputs.to_diagram_architecture(&mut result);

        assert!(
            result.failures.iter().any(|message| message
                .contains("has children declared across more than one component diagram file")),
            "Expected single-home decomposition error, got: {:?}",
            result.failures
        );
    }

    #[test]
    fn overview_equal_children_does_not_split_decomposition() {
        let inputs = ComponentDiagramInputs {
            entities: vec![
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                    "overview.puml",
                ),
                entity_in_file(
                    "comp_a.unit_1",
                    Some("unit_1"),
                    Some("comp_a"),
                    ComponentType::Component,
                    Some("unit"),
                    Vec::new(),
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a.unit_1",
                    Some("unit_1"),
                    Some("comp_a"),
                    ComponentType::Component,
                    Some("unit"),
                    Vec::new(),
                    "overview.puml",
                ),
            ],
        };

        let mut result = ValidationResult::default();
        let _architecture = inputs.to_diagram_architecture(&mut result);

        assert!(
            result.is_empty(),
            "expected no failures for a benign identical re-declaration, got: {:?}",
            result.failures
        );
    }

    #[test]
    fn overview_strict_subset_of_detail_children_does_not_split_decomposition() {
        let inputs = ComponentDiagramInputs {
            entities: vec![
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a",
                    Some("comp_a"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                    "overview.puml",
                ),
                entity_in_file(
                    "comp_a.unit_1",
                    Some("unit_1"),
                    Some("comp_a"),
                    ComponentType::Component,
                    Some("unit"),
                    Vec::new(),
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a.unit_2",
                    Some("unit_2"),
                    Some("comp_a"),
                    ComponentType::Component,
                    Some("unit"),
                    Vec::new(),
                    "detail.puml",
                ),
                entity_in_file(
                    "comp_a.unit_1",
                    Some("unit_1"),
                    Some("comp_a"),
                    ComponentType::Component,
                    Some("unit"),
                    Vec::new(),
                    "overview.puml",
                ),
            ],
        };

        let mut result = ValidationResult::default();
        let _architecture = inputs.to_diagram_architecture(&mut result);

        assert!(
            result.is_empty(),
            "expected no failures for a strict-subset re-declaration, got: {:?}",
            result.failures
        );
    }

    #[test]
    fn conflicting_declaration_error_is_order_independent() {
        let first = entity_in_file(
            "comp_a",
            Some("comp_a"),
            None,
            ComponentType::Component,
            Some("component"),
            Vec::new(),
            "detail.puml",
        );
        let second = entity_in_file(
            "comp_a",
            Some("comp_a"),
            None,
            ComponentType::Component,
            Some("unit"),
            Vec::new(),
            "overview.puml",
        );

        let mut forward_result = ValidationResult::default();
        let _forward_architecture = ComponentDiagramInputs {
            entities: vec![first.clone(), second.clone()],
        }
        .to_diagram_architecture(&mut forward_result);

        let mut reversed_result = ValidationResult::default();
        let _reversed_architecture = ComponentDiagramInputs {
            entities: vec![second, first],
        }
        .to_diagram_architecture(&mut reversed_result);

        assert_eq!(
            forward_result.failures, reversed_result.failures,
            "expected the conflicting-declaration error to be order-independent"
        );
    }
}
