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

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

use class_diagram::{Relationship, SimpleEntity};
use source_location::SourceLocation;
use uid_utils::{join, normalized_segments};

pub(crate) fn display_reference_name(reference: &str) -> &str {
    let trimmed = reference.trim();

    trimmed
        .rsplit(['.', ':'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(trimmed)
}

pub(crate) fn display_entity_name(entity: &SimpleEntity) -> &str {
    if entity.name.trim().is_empty() {
        display_reference_name(&entity.id)
    } else {
        entity.name.as_str()
    }
}

pub(crate) fn display_relationship_name(relationship: &Relationship) -> String {
    format!(
        "{} -> {:?} -> {}",
        display_reference_name(&relationship.source),
        relationship.relation_type,
        display_reference_name(&relationship.target)
    )
}

pub(crate) fn display_names<'a>(
    names: impl IntoIterator<Item = &'a str>,
    min_segments: usize,
) -> Vec<String> {
    let segmented = names
        .into_iter()
        .map(normalized_segments)
        .collect::<Vec<_>>();

    segmented
        .iter()
        .enumerate()
        .map(|(index, target_segments)| {
            let min_suffix_length = min_segments.min(target_segments.len());
            let suffix_length = (1..=target_segments.len())
                .find(|length| {
                    let target_suffix = &target_segments[target_segments.len() - length..];

                    segmented
                        .iter()
                        .enumerate()
                        .all(|(other_index, other_segments)| {
                            other_index == index
                                || other_segments.len() < *length
                                || &other_segments[other_segments.len() - length..] != target_suffix
                        })
                })
                .map(|length| length.max(min_suffix_length))
                .unwrap_or(min_suffix_length);

            join(
                target_segments[target_segments.len() - suffix_length..]
                    .iter()
                    .map(String::as_str),
            )
        })
        .collect()
}

pub(crate) fn display_names_without_common_prefix<'a>(
    names: impl IntoIterator<Item = &'a str>,
    min_segments: usize,
) -> Vec<String> {
    let segmented = names
        .into_iter()
        .map(normalized_segments)
        .collect::<Vec<_>>();
    let min_segments = if segmented.iter().all(|segments| segments.len() <= 4) {
        min_segments.min(2)
    } else {
        min_segments
    };
    let shared_prefix_len = common_prefix_len(&segmented);

    let trimmed = segmented
        .iter()
        .map(|segments| {
            let min_segments = min_segments.min(segments.len());
            let removable = segments.len().saturating_sub(min_segments);
            let remove_count = shared_prefix_len.min(removable);
            join(segments[remove_count..].iter().map(String::as_str))
        })
        .collect::<Vec<_>>();

    display_names(trimmed.iter().map(String::as_str), min_segments)
}

pub(crate) fn display_name_from_source_path(
    name: &str,
    source_file: &str,
    min_segments: usize,
) -> String {
    strip_source_path_prefix(name, source_file, min_segments).unwrap_or_else(|| {
        display_names_without_common_prefix([name], min_segments)
            .into_iter()
            .next()
            .unwrap_or_else(|| name.to_string())
    })
}

pub(in crate::validators) fn display_name_from_source_path_in_context<'a>(
    target: &'a str,
    source_file: &str,
    other_names: impl IntoIterator<Item = &'a str>,
) -> String {
    let other_names = other_names.into_iter().collect::<Vec<_>>();
    let source_display = display_name_from_source_path(target, source_file, 1);
    let context_names = std::iter::once(target)
        .chain(other_names.iter().copied())
        .collect::<Vec<_>>();
    let context_display = display_names(context_names, 1);

    let source_segments = normalized_segments(&source_display);
    let source_display_is_unique = other_names.iter().all(|other_name| {
        let other_segments = normalized_segments(other_name);
        if other_segments.len() < source_segments.len() {
            true
        } else {
            other_segments[other_segments.len() - source_segments.len()..] != source_segments[..]
        }
    });

    if source_display_is_unique {
        source_display
    } else {
        context_display
            .into_iter()
            .next()
            .unwrap_or_else(|| target.to_string())
    }
}

pub(in crate::validators) fn display_name_from_sources(
    name: &str,
    sources: &BTreeMap<String, SourceLocation>,
) -> String {
    let source_file = sources
        .get(name)
        .map(|source_location| source_location.display().0);

    display_name_from_source_path_in_context(name, source_file.as_deref().unwrap_or_default(), [])
}

pub(in crate::validators) fn display_names_from_sources(
    names: &BTreeSet<String>,
    sources: &BTreeMap<String, SourceLocation>,
) -> BTreeSet<String> {
    names
        .iter()
        .map(|name| display_name_from_sources(name, sources))
        .collect()
}

pub(in crate::validators) fn display_unit_pair_from_source_paths(
    left: &str,
    left_source_file: &str,
    right: &str,
    right_source_file: &str,
) -> [String; 2] {
    let displayed = [
        display_name_from_source_path(left, left_source_file, unit_min_segments(left)),
        display_name_from_source_path(right, right_source_file, unit_min_segments(right)),
    ];

    if displayed[0] == displayed[1] {
        display_unit_pair(left, right)
    } else {
        displayed
    }
}

pub(in crate::validators) fn display_unit_pair_from_optional_source_paths(
    left: &str,
    left_source_file: Option<&str>,
    right: &str,
    right_source_file: Option<&str>,
) -> [String; 2] {
    match (left_source_file, right_source_file) {
        (Some(left_source_file), Some(right_source_file))
            if !left_source_file.is_empty() && !right_source_file.is_empty() =>
        {
            display_unit_pair_from_source_paths(left, left_source_file, right, right_source_file)
        }
        _ => display_unit_pair(left, right),
    }
}

pub(crate) fn format_display_names<'a>(
    names: impl IntoIterator<Item = &'a str>,
    min_segments: usize,
) -> String {
    let displayed = display_names_without_common_prefix(names, min_segments);

    if displayed.is_empty() {
        return "<none>".to_string();
    }

    displayed
        .into_iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

fn common_prefix_len(segmented_names: &[Vec<String>]) -> usize {
    let Some(first) = segmented_names.first() else {
        return 0;
    };

    first
        .iter()
        .enumerate()
        .take_while(|(index, segment)| {
            segmented_names
                .iter()
                .all(|segments| segments.get(*index) == Some(segment))
        })
        .count()
}

fn strip_source_path_prefix(name: &str, source_file: &str, min_segments: usize) -> Option<String> {
    let name_segments = normalized_segments(name);
    if name_segments.is_empty() {
        return None;
    }

    let source_segments = source_directory_segments(source_file);
    let overlap_len = longest_suffix_prefix_overlap(&source_segments, &name_segments);
    if overlap_len == 0 {
        return None;
    }

    let min_segments = min_segments.min(name_segments.len());
    let removable = name_segments.len().saturating_sub(min_segments);
    let remove_count = overlap_len.min(removable);

    Some(join(
        name_segments[remove_count..].iter().map(String::as_str),
    ))
}

fn source_directory_segments(source_file: &str) -> Vec<String> {
    Path::new(source_file)
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .filter_map(|component| match component {
            Component::Normal(segment) => Some(segment.to_string_lossy().to_string()),
            _ => None,
        })
        .collect()
}

fn longest_suffix_prefix_overlap(source_segments: &[String], name_segments: &[String]) -> usize {
    let max_overlap = source_segments.len().min(name_segments.len());

    (1..=max_overlap)
        .rev()
        .find(|overlap_len| {
            source_segments[source_segments.len() - overlap_len..] == name_segments[..*overlap_len]
        })
        .unwrap_or(0)
}

pub(in crate::validators) fn display_reference_name_set(
    names: &BTreeSet<String>,
) -> BTreeSet<String> {
    names
        .iter()
        .map(|name| display_reference_name(name).to_string())
        .collect()
}

pub(in crate::validators) fn display_unit_pair(left: &str, right: &str) -> [String; 2] {
    let names = [left, right];
    let min_segments = if names
        .iter()
        .map(|name| normalized_segments(name).len())
        .all(|segment_count| segment_count <= 4)
    {
        2
    } else {
        3
    };
    let displayed = display_names_without_common_prefix(names, min_segments);

    [displayed[0].clone(), displayed[1].clone()]
}

fn unit_min_segments(name: &str) -> usize {
    if normalized_segments(name).len() <= 4 {
        2
    } else {
        3
    }
}

pub(in crate::validators) fn format_name_list<T>(names: impl IntoIterator<Item = T>) -> String
where
    T: AsRef<str>,
{
    names
        .into_iter()
        .map(|name| format!("\"{}\"", name.as_ref()))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in crate::validators) fn format_sequence_call(
    caller: &str,
    callee: &str,
    method_name: &str,
) -> String {
    format!("\"{caller}\" -> \"{callee}\" : \"{method_name}\"")
}

#[cfg(test)]
mod tests {
    use super::{
        display_entity_name, display_name_from_source_path,
        display_name_from_source_path_in_context, display_names,
        display_names_without_common_prefix, display_reference_name, display_relationship_name,
        display_unit_pair_from_source_paths, format_display_names,
    };
    use class_diagram::{EntityType, RelationType, Relationship, SimpleEntity};
    use source_location::SourceLocation;

    #[test]
    fn display_reference_name_uses_leaf_for_root_anchored_identifiers() {
        assert_eq!(
            display_reference_name("validation.core.test.Engine"),
            "Engine"
        );
        assert_eq!(
            display_reference_name("validation::core::test::Engine"),
            "Engine"
        );
    }

    #[test]
    fn display_entity_name_prefers_entity_name_over_id() {
        let entity = SimpleEntity {
            id: "validation.core.test.Engine".to_string(),
            name: "Engine".to_string(),
            enclosing_namespace_id: None,
            stereotypes: Vec::new(),
            entity_type: EntityType::Class,
            type_aliases: Vec::new(),
            variables: Vec::new(),
            methods: Vec::new(),
            template_parameters: None,
            enum_literals: Vec::new(),
            relationships: Vec::new(),
            source_location: SourceLocation::new("test.puml", 1),
        };

        assert_eq!(display_entity_name(&entity), "Engine");
    }

    #[test]
    fn display_relationship_name_uses_short_endpoint_names() {
        let relationship = Relationship {
            source: "validation.core.test.Car".to_string(),
            target: "validation::core::test::Wheel".to_string(),
            relation_type: RelationType::Composition,
            source_multiplicity: None,
            target_multiplicity: None,
            source_location: SourceLocation::new("test.puml", 1),
        };

        assert_eq!(
            display_relationship_name(&relationship),
            "Car -> Composition -> Wheel"
        );
    }

    #[test]
    fn display_names_uses_shortest_unique_suffix() {
        let names = [
            "validation.core.example.unit_1.Controller",
            "validation.core.example.unit_2.Controller",
        ];

        assert_eq!(
            display_names(names, 1),
            vec!["unit_1.Controller", "unit_2.Controller"]
        );
    }

    #[test]
    fn display_names_keeps_leaf_when_already_unique() {
        let names = [
            "validation.core.example.Repository",
            "validation.core.example.Service",
        ];

        assert_eq!(display_names(names, 1), vec!["Repository", "Service"]);
    }

    #[test]
    fn display_name_from_source_path_expands_when_context_has_same_display_name() {
        assert_eq!(
            display_name_from_source_path_in_context(
                "validation.core.vehicle.control.SensorUnit",
                "validation/core/vehicle/component.puml",
                ["control.SensorUnit"],
            ),
            "vehicle.control.SensorUnit"
        );
    }

    #[test]
    fn display_names_shortens_single_name() {
        assert_eq!(
            display_names(
                ["validation.core.integration_test.component_sequence.case_a.package_a.InternalInterface"],
                2,
            ),
            vec!["package_a.InternalInterface"]
        );
    }

    #[test]
    fn format_display_names_quotes_names() {
        let names = [
            "validation.core.integration_test.case_a.package_a.InternalInterface",
            "validation.core.integration_test.case_a.package_b.InternalInterface",
        ];

        assert_eq!(
            format_display_names(names, 2),
            "\"package_a.InternalInterface\", \"package_b.InternalInterface\""
        );
    }

    #[test]
    fn format_display_names_handles_empty() {
        assert_eq!(
            format_display_names(std::iter::empty::<&str>(), 2),
            "<none>"
        );
    }

    #[test]
    fn display_names_without_common_prefix_caps_short_names_at_two_segments() {
        let names = [
            "validation.core.integration_test.case_a.package_a.component_a.unit_1",
            "validation.core.integration_test.case_a.package_b.component_a.unit_1",
        ];

        assert_eq!(
            display_names_without_common_prefix(names, 3),
            vec![
                "package_a.component_a.unit_1",
                "package_b.component_a.unit_1"
            ]
        );
    }

    #[test]
    fn display_name_from_source_path_drops_matching_directory_prefix() {
        assert_eq!(
            display_name_from_source_path(
                "validation.core.integration_test.case_a.package_a.InternalInterface",
                "validation/core/integration_test/case_a/component_diagram.puml",
                2,
            ),
            "package_a.InternalInterface"
        );
    }

    #[test]
    fn display_name_from_source_path_falls_back_without_overlap() {
        assert_eq!(
            display_name_from_source_path(
                "validation.core.integration_test.case_a.package_a.component_a.unit_1",
                "other/root/component_diagram.puml",
                3,
            ),
            "package_a.component_a.unit_1"
        );
    }

    #[test]
    fn display_unit_pair_from_source_paths_uses_distinct_trimmed_names() {
        assert_eq!(
            display_unit_pair_from_source_paths(
                "validation.core.case_a.package_a.component_a.Controller",
                "validation/core/case_a/component_diagram_a.puml",
                "validation.core.case_b.package_b.component_b.Controller",
                "validation/core/case_b/component_diagram_b.puml",
            ),
            [
                "package_a.component_a.Controller".to_string(),
                "package_b.component_b.Controller".to_string(),
            ]
        );
    }
}
