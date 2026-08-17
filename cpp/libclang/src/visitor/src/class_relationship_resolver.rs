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

//! Second-pass relationship inference for extracted class-diagram entities.

use std::collections::HashSet;

use class_diagram::{EntityType, RelationType, Relationship, SimpleEntity, SourceLocation};
use cpp_semantics::ResolvedType;

use crate::context::{ParsedClassInfo, ParsedMethodType, ParsedVariableType, VisitContext};

pub(crate) fn resolve_relationships(ctx: &mut VisitContext) {
    let builders = std::mem::take(&mut ctx.parsed_class_info);
    let known_type_ids: HashSet<String> = ctx.types.keys().cloned().collect();

    for builder in builders {
        build_relationships_for_class(ctx, &builder);
        infer_relationships_from_builder(ctx, &builder, &known_type_ids);
    }
}

fn build_relationships_for_class(ctx: &mut VisitContext, builder: &ParsedClassInfo) {
    for base in &builder.base_classes {
        let Some(resolved_base) = base.resolved_type.referenced_entity_id() else {
            if matches!(base.resolved_type, ResolvedType::Dependent(_)) {
                log::debug!(
                    "unable to resolve base type '{}' for '{}'; \
                     skipping inheritance relationship (dependent/decltype expression)",
                    base.resolved_type.render_for_display(),
                    builder.id
                );
            } else {
                log::warn!(
                    "unable to resolve base type '{}' for '{}'; \
                     skipping inheritance relationship (unexpected unresolved type)",
                    base.resolved_type.render_for_display(),
                    builder.id
                );
            }
            continue;
        };

        let Some(target_class) = ctx.types.get(resolved_base) else {
            log::debug!(
                "base type '{}' not found in type map for '{}'; \
                 skipping inheritance relationship (external dependency)",
                resolved_base,
                builder.id
            );
            continue;
        };

        let relation_type = if target_class.entity_type == EntityType::Interface {
            RelationType::Implementation
        } else {
            RelationType::Inheritance
        };

        let Some(class) = ctx.types.get_mut(&builder.id) else {
            log::warn!(
                "source class '{}' unexpectedly missing from type map; \
                 skipping inheritance relationship to '{}'",
                builder.id,
                resolved_base
            );
            continue;
        };

        add_relationship(
            class,
            resolved_base.to_string(),
            relation_type,
            &base.source_location,
        );
    }
}

fn add_relationship(
    class: &mut SimpleEntity,
    target: String,
    relation_type: RelationType,
    source_location: &SourceLocation,
) {
    if target == class.id {
        return;
    }

    let relationship = Relationship {
        source: class.id.clone(),
        target,
        relation_type,
        source_multiplicity: None,
        target_multiplicity: None,
        source_location: source_location.clone(),
    };

    let duplicate = class.relationships.iter().any(|existing| {
        existing.source == relationship.source
            && existing.target == relationship.target
            && existing.relation_type == relationship.relation_type
            && existing.source_multiplicity == relationship.source_multiplicity
            && existing.target_multiplicity == relationship.target_multiplicity
    });

    if !duplicate {
        class.relationships.push(relationship);
    }
}

fn infer_relationships_from_builder(
    ctx: &mut VisitContext,
    builder: &ParsedClassInfo,
    known_class_ids: &HashSet<String>,
) {
    let Some(class) = ctx.types.get_mut(&builder.id) else {
        log::warn!(
            "source class '{}' unexpectedly missing from type map; \
             skipping inferred relationships",
            builder.id
        );
        return;
    };

    infer_variable_relationships(class, &builder.variable_types, known_class_ids);
    infer_method_relationships(class, &builder.method_types, known_class_ids);
}

fn infer_variable_relationships(
    class: &mut SimpleEntity,
    variable_types: &[ParsedVariableType],
    known_class_ids: &HashSet<String>,
) {
    for variable in variable_types {
        add_relationship_from_resolved_type(
            class,
            &variable.resolved_type,
            known_class_ids,
            RelationType::Aggregation,
            RelationType::Composition,
            &variable.source_location,
        );
    }
}

fn infer_method_relationships(
    class: &mut SimpleEntity,
    method_types: &[ParsedMethodType],
    known_class_ids: &HashSet<String>,
) {
    for method in method_types {
        add_relationship_from_resolved_type(
            class,
            &method.return_type,
            known_class_ids,
            RelationType::Dependency,
            RelationType::Association,
            &method.source_location,
        );

        for parameter_type in &method.parameter_types {
            add_relationship_from_resolved_type(
                class,
                parameter_type,
                known_class_ids,
                RelationType::Dependency,
                RelationType::Association,
                &method.source_location,
            );
        }
    }
}

fn add_relationship_from_resolved_type(
    class: &mut SimpleEntity,
    resolved_type: &ResolvedType,
    known_class_ids: &HashSet<String>,
    non_owning_relation: RelationType,
    owning_relation: RelationType,
    source_location: &SourceLocation,
) {
    let Some(raw_target) = resolved_type.relationship_target_entity_id() else {
        return;
    };

    let Some(target) = resolve_in_model_target(class, raw_target, known_class_ids) else {
        return;
    };

    let relation_type = if resolved_type.is_non_owning() {
        non_owning_relation
    } else {
        owning_relation
    };

    add_relationship(class, target, relation_type, source_location);
}

fn resolve_in_model_target(
    source_class: &SimpleEntity,
    raw_target: &str,
    known_class_ids: &HashSet<String>,
) -> Option<String> {
    if known_class_ids.contains(raw_target) {
        return Some(raw_target.to_string());
    }

    if !raw_target.contains("::") {
        if let Some(namespace) = source_class.enclosing_namespace_id.as_deref() {
            let mut current_namespace: Option<&str> = Some(namespace);
            while let Some(current_namespace_id) = current_namespace {
                let candidate = format!("{current_namespace_id}::{raw_target}");
                if known_class_ids.contains(&candidate) {
                    return Some(candidate);
                }
                current_namespace = current_namespace_id
                    .rsplit_once("::")
                    .map(|(parent, _)| parent);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use class_diagram::{RelationType, SimpleEntity, SourceLocation};
    use cpp_semantics::ResolvedType;

    use super::resolve_relationships;
    use crate::context::{
        ParsedBaseClass, ParsedClassInfo, ParsedMethodType, ParsedVariableType, VisitContext,
    };

    #[test]
    fn resolve_relationships_uses_variable_and_method_source_locations() {
        let source_file = "unit_source.cpp";

        let mut ctx = VisitContext::default();
        ctx.types.insert(
            "Engine".to_string(),
            SimpleEntity {
                id: "Engine".to_string(),
                name: "Engine".to_string(),
                source_location: SourceLocation::new(source_file, 1),
                ..Default::default()
            },
        );
        ctx.types.insert(
            "Car".to_string(),
            SimpleEntity {
                id: "Car".to_string(),
                name: "Car".to_string(),
                source_location: SourceLocation::new(source_file, 3),
                ..Default::default()
            },
        );

        ctx.parsed_class_info.push(ParsedClassInfo {
            id: "Car".to_string(),
            base_classes: vec![],
            variable_types: vec![ParsedVariableType {
                name: "engine".to_string(),
                resolved_type: ResolvedType::UserDefined("Engine".to_string()),
                source_location: SourceLocation::new(source_file, 5),
            }],
            method_types: vec![ParsedMethodType {
                name: "buildEngine".to_string(),
                return_type: ResolvedType::UserDefined("Engine".to_string()),
                parameter_types: vec![],
                source_location: SourceLocation::new(source_file, 6),
            }],
        });

        resolve_relationships(&mut ctx);

        let car = ctx
            .types
            .get("Car")
            .expect("Car must still exist after relationship resolution");

        // Class source location should not be modified by relationship resolution.
        assert_eq!(car.source_location, SourceLocation::new(source_file, 3));

        let variable_relationship = car
            .relationships
            .iter()
            .find(|relationship| {
                relationship.target == "Engine"
                    && relationship.relation_type == RelationType::Composition
            })
            .expect("Expected a composition relationship inferred from member variable type");
        assert_eq!(
            variable_relationship.source_location,
            SourceLocation::new(source_file, 5)
        );

        let method_relationship = car
            .relationships
            .iter()
            .find(|relationship| {
                relationship.target == "Engine"
                    && relationship.relation_type == RelationType::Association
            })
            .expect("Expected an association relationship inferred from method return type");
        assert_eq!(
            method_relationship.source_location,
            SourceLocation::new(source_file, 6)
        );
    }

    /// Regression test for a real crash: a base class like
    /// `struct is_maplike_container : decltype(is_maplike_container_impl(std::declval<T>())) {};`
    /// resolves to `ResolvedType::Dependent(..)` because `decltype(...)` of a
    /// dependent expression cannot be tied to a concrete entity id without template
    /// instantiation. `resolve_relationships` must not panic on this: it should skip
    /// only the unresolvable base while still building relationships for any other,
    /// resolvable base classes on the same type.
    #[test]
    fn resolve_relationships_skips_dependent_base_class_without_panicking() {
        let source_file = "is_maplike_container.hpp";
        let mut ctx = VisitContext::default();
        ctx.types.insert(
            "amp::detail::is_container_base".to_string(),
            SimpleEntity {
                id: "amp::detail::is_container_base".to_string(),
                name: "is_container_base".to_string(),
                source_location: SourceLocation::new(source_file, 1),
                ..Default::default()
            },
        );
        ctx.types.insert(
            "amp::detail::is_maplike_container".to_string(),
            SimpleEntity {
                id: "amp::detail::is_maplike_container".to_string(),
                name: "is_maplike_container".to_string(),
                source_location: SourceLocation::new(source_file, 5),
                ..Default::default()
            },
        );
        ctx.parsed_class_info.push(ParsedClassInfo {
            id: "amp::detail::is_maplike_container".to_string(),
            base_classes: vec![
                // Unresolvable dependent expression — must be skipped, not panic.
                ParsedBaseClass {
                    resolved_type: ResolvedType::Dependent(
                        "decltype(is_maplike_container_impl(std::declval<T>()))".to_string(),
                    ),
                    source_location: SourceLocation::new(source_file, 5),
                },
                // A normal, resolvable base class alongside the dependent one.
                ParsedBaseClass {
                    resolved_type: ResolvedType::UserDefined(
                        "amp::detail::is_container_base".to_string(),
                    ),
                    source_location: SourceLocation::new(source_file, 5),
                },
            ],
            variable_types: vec![],
            method_types: vec![],
        });

        // Must not panic.
        resolve_relationships(&mut ctx);

        let is_maplike_container = ctx
            .types
            .get("amp::detail::is_maplike_container")
            .expect("is_maplike_container must still exist after relationship resolution");

        // No relationship should have been created for the unresolvable dependent base.
        assert!(
            !is_maplike_container
                .relationships
                .iter()
                .any(|relationship| relationship.relation_type == RelationType::Implementation),
            "dependent base class must not produce a relationship"
        );

        // The sibling resolvable base class must still be processed correctly.
        let inheritance_relationship = is_maplike_container
            .relationships
            .iter()
            .find(|relationship| {
                relationship.target == "amp::detail::is_container_base"
                    && relationship.relation_type == RelationType::Inheritance
            })
            .expect("Expected an inheritance relationship for the resolvable base class");
        assert_eq!(
            inheritance_relationship.source_location,
            SourceLocation::new(source_file, 5)
        );
    }

    /// An unresolved base type that is *not* `ResolvedType::Dependent` (e.g.
    /// `Unknown`) is unexpected and gets a `log::warn!`, but must still never
    /// abort the parser — when in doubt, warn and skip rather than crash.
    #[test]
    fn resolve_relationships_warns_and_skips_unexpected_unresolved_base() {
        let source_file = "unit_source.cpp";
        let mut ctx = VisitContext::default();
        ctx.types.insert(
            "Derived".to_string(),
            SimpleEntity {
                id: "Derived".to_string(),
                name: "Derived".to_string(),
                source_location: SourceLocation::new(source_file, 1),
                ..Default::default()
            },
        );
        ctx.parsed_class_info.push(ParsedClassInfo {
            id: "Derived".to_string(),
            base_classes: vec![ParsedBaseClass {
                // Not `Dependent`: an unexpected, unresolvable base type.
                resolved_type: ResolvedType::Unknown("SomeWeirdType".to_string()),
                source_location: SourceLocation::new(source_file, 1),
            }],
            variable_types: vec![],
            method_types: vec![],
        });

        // Must not panic.
        resolve_relationships(&mut ctx);

        let derived = ctx
            .types
            .get("Derived")
            .expect("Derived must still exist after relationship resolution");
        assert!(
            derived.relationships.is_empty(),
            "unresolvable base class must not produce a relationship"
        );
    }
}
