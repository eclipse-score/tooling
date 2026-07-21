///////////////////////////////////////////////////////////////////////////////////
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
////////////////////////////////////////////////////////////////////////////////////

use class_diagram::{RelationType, SimpleEntity, SourceLocation};
use visit_tu::context::{
    ParsedBaseClass, ParsedClassInfo, ParsedMethodType, ParsedVariableType, VisitContext,
};
use visit_tu::{ClassVisitor, ResolvedType};

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

    ClassVisitor::resolve_relationships(&mut ctx);

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
    ClassVisitor::resolve_relationships(&mut ctx);

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
    ClassVisitor::resolve_relationships(&mut ctx);

    let derived = ctx
        .types
        .get("Derived")
        .expect("Derived must still exist after relationship resolution");
    assert!(
        derived.relationships.is_empty(),
        "unresolvable base class must not produce a relationship"
    );
}
