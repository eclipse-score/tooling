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

//! Presentation rules for resolved libclang types.

use clang::Type;
use cpp_semantics::ResolvedType;

use crate::clang_adapter::source_filter;

pub(crate) fn render_type_for_display(original: &Type, resolved: &ResolvedType) -> String {
    // Prefer source spelling only in carefully scoped cases (see helper below);
    // otherwise use normalized rendering from semantic type model.
    if should_prefer_source_display_name(original, resolved) {
        original.get_display_name()
    } else {
        resolved.render_for_display()
    }
}

fn should_prefer_source_display_name(ty: &Type, resolved: &ResolvedType) -> bool {
    // Source display names are used for externally declared/system types where
    // canonicalized rendering may be less readable for users.
    if !source_filter::is_declared_in_external_or_system_header(ty)
        || contains_template_type(resolved)
    {
        return false;
    }

    let source_display = ty.get_display_name();
    let rendered = resolved.render_for_display();

    source_display != rendered
}

fn contains_template_type(resolved: &ResolvedType) -> bool {
    match resolved {
        ResolvedType::Template { .. } => true,
        ResolvedType::Function {
            return_type,
            parameter_types,
            ..
        } => {
            contains_template_type(return_type)
                || parameter_types.iter().any(contains_template_type)
        }
        ResolvedType::FunctionPointer(inner)
        | ResolvedType::FunctionReference(inner)
        | ResolvedType::Pointer(inner)
        | ResolvedType::Reference(inner)
        | ResolvedType::RValueReference(inner)
        | ResolvedType::Const(inner)
        | ResolvedType::Volatile(inner) => contains_template_type(inner),
        ResolvedType::Array { element, .. } => contains_template_type(element),
        ResolvedType::Builtin(_)
        | ResolvedType::UserDefined(_)
        | ResolvedType::Unknown(_)
        | ResolvedType::Dependent(_) => false,
    }
}
