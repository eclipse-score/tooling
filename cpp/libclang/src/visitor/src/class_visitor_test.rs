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
use visit_tu::context::{ParsedClassInfo, ParsedMethodType, ParsedVariableType, VisitContext};
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
