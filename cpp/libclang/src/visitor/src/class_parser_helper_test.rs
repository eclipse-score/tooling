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

#[path = "class_parser_helper.rs"]
mod class_parser_helper;

use class_parser_helper::{collapse_std_internal_namespaces, ResolvedType};

#[test]
fn referenced_entity_through_wrappers_points_to_inner_type() {
    let ty = ResolvedType::Const(Box::new(ResolvedType::Pointer(Box::new(
        ResolvedType::UserDefined("Vehicle::Engine".to_string()),
    ))));

    assert_eq!(ty.referenced_entity_id(), Some("Vehicle::Engine"));
}

#[test]
fn referenced_entity_through_function_pointer_chain() {
    let ty = ResolvedType::FunctionPointer(Box::new(ResolvedType::Const(Box::new(
        ResolvedType::Pointer(Box::new(ResolvedType::UserDefined("Engine".to_string()))),
    ))));

    assert_eq!(ty.referenced_entity_id(), Some("Engine"));
}

#[test]
fn referenced_entity_for_template_uses_template_base() {
    let ty = ResolvedType::Reference(Box::new(ResolvedType::Template {
        base: "std::vector".to_string(),
        args: vec![ResolvedType::UserDefined("Vehicle::Engine".to_string())],
    }));

    assert_eq!(ty.referenced_entity_id(), Some("std::vector"));
}

#[test]
fn relationship_target_prefers_template_argument_over_base() {
    let ty = ResolvedType::Template {
        base: "std::vector".to_string(),
        args: vec![ResolvedType::UserDefined("Vehicle::Engine".to_string())],
    };

    assert_eq!(ty.relationship_target_entity_id(), Some("Vehicle::Engine"));
}

#[test]
fn render_template_pointer_and_array_types() {
    let template = ResolvedType::Template {
        base: "std::vector".to_string(),
        args: vec![ResolvedType::UserDefined("MyNamespace::Engine".to_string())],
    };
    assert_eq!(
        template.render_for_display(),
        "std::vector<MyNamespace::Engine>"
    );

    let ptr = ResolvedType::Pointer(Box::new(ResolvedType::UserDefined(
        "MyNamespace::Engine".to_string(),
    )));
    assert_eq!(ptr.render_for_display(), "MyNamespace::Engine *");

    let arr = ResolvedType::Array {
        element: Box::new(ResolvedType::Builtin("int".to_string())),
        size: Some(8),
    };
    assert_eq!(arr.render_for_display(), "int[8]");
}

#[test]
fn render_unsized_array_type() {
    let arr = ResolvedType::Array {
        element: Box::new(ResolvedType::Builtin("int".to_string())),
        size: None,
    };

    assert_eq!(arr.render_for_display(), "int[]");
}

#[test]
fn render_const_pointer_type() {
    let ty = ResolvedType::Const(Box::new(ResolvedType::Pointer(Box::new(
        ResolvedType::Builtin("int".to_string()),
    ))));

    assert_eq!(ty.render_for_display(), "int *const");
}

#[test]
fn render_volatile_type() {
    let ty = ResolvedType::Volatile(Box::new(ResolvedType::Builtin("int".to_string())));

    assert_eq!(ty.render_for_display(), "volatile int");
}

#[test]
fn render_function_and_function_pointer_types() {
    let function = ResolvedType::Function {
        return_type: Box::new(ResolvedType::Builtin("void".to_string())),
        parameter_types: vec![ResolvedType::UserDefined("Engine".to_string())],
        is_variadic: false,
    };
    assert_eq!(function.render_for_display(), "void(Engine)");

    let fn_ptr = ResolvedType::FunctionPointer(Box::new(function));
    assert_eq!(fn_ptr.render_for_display(), "void (*)(Engine)");
}

#[test]
fn render_function_reference_type() {
    let function = ResolvedType::Function {
        return_type: Box::new(ResolvedType::Builtin("void".to_string())),
        parameter_types: vec![ResolvedType::UserDefined("Engine".to_string())],
        is_variadic: false,
    };

    let fn_ref = ResolvedType::FunctionReference(Box::new(function));
    assert_eq!(fn_ref.render_for_display(), "void (&)(Engine)");
}

#[test]
fn render_variadic_function_return_type_style() {
    let fn_type = ResolvedType::Function {
        return_type: Box::new(ResolvedType::UserDefined(
            "flatbuffers::FlatBufferBuilder".to_string(),
        )),
        parameter_types: vec![ResolvedType::Builtin("int".to_string())],
        is_variadic: true,
    };

    assert_eq!(
        fn_type.render_for_display(),
        "flatbuffers::FlatBufferBuilder(int, ...)"
    );
}

#[test]
fn non_owning_detection_for_pointer_and_const_pointer() {
    let ptr = ResolvedType::Pointer(Box::new(ResolvedType::UserDefined("Engine".to_string())));
    assert!(ptr.is_non_owning());

    let const_ptr = ResolvedType::Const(Box::new(ptr));
    assert!(const_ptr.is_non_owning());
}

#[test]
fn non_owning_detection_for_template_wrappers() {
    let shared_ptr = ResolvedType::Template {
        base: "std::shared_ptr".to_string(),
        args: vec![ResolvedType::UserDefined("Engine".to_string())],
    };
    assert!(shared_ptr.is_non_owning());

    let vector_value = ResolvedType::Template {
        base: "std::vector".to_string(),
        args: vec![ResolvedType::UserDefined("Engine".to_string())],
    };
    assert!(!vector_value.is_non_owning());
}

#[test]
fn collapse_std_internal_namespaces_only_under_std() {
    let parts = vec![
        ("std".to_string(), true),
        ("__1".to_string(), true),
        ("vector".to_string(), false),
    ];

    assert_eq!(
        collapse_std_internal_namespaces(parts),
        vec!["std".to_string(), "vector".to_string()]
    );
}

#[test]
fn collapse_std_internal_namespaces_does_not_drop_non_std_internal_namespaces() {
    let parts = vec![
        ("foo".to_string(), true),
        ("__detail".to_string(), true),
        ("Bar".to_string(), false),
    ];

    assert_eq!(
        collapse_std_internal_namespaces(parts),
        vec!["foo".to_string(), "__detail".to_string(), "Bar".to_string()]
    );
}
