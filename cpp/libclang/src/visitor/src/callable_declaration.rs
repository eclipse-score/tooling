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

use clang::{Entity, EntityKind};
use class_diagram::{FunctionArgument, TemplateParameter};

use crate::types::renderer::render_type_for_display;
use crate::types::resolver::resolve_type;

/// Returns callable parameters, including the fallback required for template cursors.
///
/// Normally libclang provides the parameter list via `Entity::get_arguments()`.
/// However, for some cursor kinds (e.g. `FunctionTemplate`) or certain libclang
/// versions, `get_arguments()` may return `None` even though the AST still
/// contains `ParmDecl` child cursors.
pub(crate) fn callable_arguments<'tu>(entity: &Entity<'tu>) -> Vec<Entity<'tu>> {
    // fall back to collecting all direct `ParmDecl` children from
    // the cursor to recover the parameter list.
    entity.get_arguments().unwrap_or_else(|| {
        entity
            .get_children()
            .into_iter()
            .filter(|child| child.get_kind() == EntityKind::ParmDecl)
            .collect()
    })
}

pub(crate) fn parse_function_parameters(entity: &Entity) -> Vec<FunctionArgument> {
    let mut parameters: Vec<FunctionArgument> = callable_arguments(entity)
        .into_iter()
        .map(|argument| {
            let raw_param_type = argument
                .get_type()
                .map(|ty| ty.get_display_name())
                .unwrap_or_default();

            FunctionArgument {
                name: argument.get_name().unwrap_or_default(),
                param_type: Some(normalize_pack_expansion_type(&raw_param_type)),
                is_variadic: false,
                is_pack_expansion: raw_param_type.contains("..."),
            }
        })
        .collect();

    if entity.get_type().is_some_and(|ty| ty.is_variadic()) {
        parameters.push(FunctionArgument {
            name: String::new(),
            param_type: None,
            is_variadic: true,
            is_pack_expansion: false,
        });
    }

    parameters
}

pub(crate) fn parse_callable_return_type(entity: &Entity) -> Option<String> {
    entity.get_result_type().map(|return_type| {
        let resolved_type = resolve_type(&return_type);
        render_type_for_display(&return_type, &resolved_type)
    })
}

pub(crate) fn parse_template_parameters(entity: &Entity) -> Option<Vec<TemplateParameter>> {
    let parameters = entity
        .get_children()
        .into_iter()
        .enumerate()
        .filter_map(|(index, child)| match child.get_kind() {
            // template <typename Foo>  →  "name: Foo, is_pack: False"
            // template <typename, typename> -> "name: T0, is_pack: False", "name: T1, is_pack: False"
            // template <typename... Foo> -> "name: Foo, is_pack: True"
            EntityKind::TemplateTypeParameter => Some(TemplateParameter::Type {
                name: child.get_name().unwrap_or_else(|| format!("T{index}")),
                is_pack: is_template_parameter_pack(&child),
            }),
            // template <int N>  →  "name: N, value_type: int"
            EntityKind::NonTypeTemplateParameter => Some(TemplateParameter::NonType {
                name: child.get_name().unwrap_or_default(),
                value_type: child
                    .get_type()
                    .map(|ty| ty.get_display_name())
                    .unwrap_or_default(),
                is_pack: is_template_parameter_pack(&child),
            }),
            // template <template<...> class C>  → "name: C, parameters: [...], is_pack: False"
            EntityKind::TemplateTemplateParameter => Some(TemplateParameter::Template {
                name: child.get_name().unwrap_or_else(|| format!("T{index}")),
                parameters: parse_template_parameters(&child).unwrap_or_default(),
                is_pack: is_template_parameter_pack(&child),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();

    (!parameters.is_empty()).then_some(parameters)
}

fn normalize_pack_expansion_type(param_type: &str) -> String {
    param_type.replace("...", "").trim().to_string()
}

fn is_template_parameter_pack(entity: &Entity) -> bool {
    entity.get_range().is_some_and(|range| {
        range
            .tokenize()
            .iter()
            .any(|token| token.get_spelling() == "...")
    }) || entity
        .get_display_name()
        .as_deref()
        .is_some_and(|display_name| display_name.contains("..."))
}
