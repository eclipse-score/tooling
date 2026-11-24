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

//! Normalization of C++ type spellings and method identities, shared by the
//! class declaration merge and the design-vs-implementation validator.

use class_diagram::{
    FunctionArgument, Method, MethodModifier, TemplateInstantiation, TemplateParameter,
};
use source_location::SourceLocation;

pub(crate) fn method_key(method: &Method) -> String {
    let parameter_types = parameter_signature(&method.parameters);
    let const_qualifier = if method.modifiers.contains(&MethodModifier::Const) {
        " const"
    } else {
        ""
    };
    format!("{}({}){}", method.name, parameter_types, const_qualifier)
}

pub(crate) fn parameter_signature(parameters: &[FunctionArgument]) -> String {
    parameters
        .iter()
        .map(|parameter| {
            let variadic = if parameter.is_variadic || parameter.is_pack_expansion {
                "..."
            } else {
                ""
            };
            format!(
                "{}{}",
                parameter
                    .param_type
                    .as_deref()
                    .map(render_type_name)
                    .unwrap_or_default(),
                variadic
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn contains_literal(type_name: &str) -> bool {
    type_name.contains('"') || type_name.contains('\'')
}

pub(crate) fn render_type_name(type_name: &str) -> String {
    if contains_literal(type_name) {
        type_name.to_string()
    } else {
        normalize_type_name(type_name)
    }
}

pub(crate) fn normalized_method(method: &Method) -> Method {
    let mut normalized = method.clone();
    normalized.source_location = SourceLocation::new("", 0);
    normalized.conversion_target = normalized
        .conversion_target
        .as_deref()
        .map(render_type_name);
    if let Some(target) = &normalized.conversion_target {
        normalized.name = format!("operator {target}");
    }
    normalize_callable_types(
        &mut normalized.return_type,
        &mut normalized.return_template_instantiation,
        &mut normalized.parameters,
        &mut normalized.template_parameters,
    );
    normalized
}

pub(crate) fn normalize_callable_types(
    return_type: &mut Option<String>,
    return_metadata: &mut Option<TemplateInstantiation>,
    parameters: &mut [FunctionArgument],
    template_parameters: &mut Option<Vec<TemplateParameter>>,
) {
    *return_type = return_type.as_deref().map(render_type_name);
    normalize_template_metadata(return_metadata);
    for parameter in parameters {
        parameter.param_type = parameter.param_type.as_deref().map(render_type_name);
        normalize_template_metadata(&mut parameter.template_instantiation);
    }
    if let Some(parameters) = template_parameters {
        normalize_template_declarations(parameters);
    }
}

fn normalize_template_metadata(metadata: &mut Option<TemplateInstantiation>) {
    if let Some(metadata) = metadata {
        metadata.name = render_type_name(&metadata.name);
        for argument in &mut metadata.arguments {
            *argument = render_type_name(argument);
        }
        if let Some(decorators) = &mut metadata.outer_decorators {
            class_diagram::TypeDecorator::canonicalize(decorators);
        }
    }
}

fn normalize_template_declarations(parameters: &mut [TemplateParameter]) {
    for parameter in parameters {
        match parameter {
            TemplateParameter::NonType { value_type, .. } => {
                *value_type = normalize_non_type_template_parameter_type(value_type);
            }
            TemplateParameter::Template { parameters, .. } => {
                normalize_template_declarations(parameters);
            }
            TemplateParameter::Type { .. } => {}
        }
    }
}

pub(crate) fn normalize_parameter_type(type_name: &str) -> String {
    strip_top_level_const(&normalize_type_name(type_name))
}

// Top-level const on a by-value parameter is not part of the C++ signature.
fn strip_top_level_const(type_name: &str) -> String {
    if let Some(pointer) = type_name.strip_suffix("*const") {
        return format!("{pointer}*");
    }
    match type_name.strip_prefix("const") {
        Some(rest) if rest.starts_with(char::is_whitespace) && !has_top_level_declarator(rest) => {
            rest.trim_start().to_string()
        }
        _ => type_name.to_string(),
    }
}

// A `*`, `&`, `[` or `(` outside template arguments makes a leading const low-level.
fn has_top_level_declarator(type_name: &str) -> bool {
    let mut depth = 0usize;
    type_name.chars().any(|character| match character {
        '<' => {
            depth += 1;
            false
        }
        '>' => {
            depth = depth.saturating_sub(1);
            false
        }
        '*' | '&' | '[' | '(' => depth == 0,
        _ => false,
    })
}

// `T* const` and `T *const` both become `T*const`.
fn tighten_pointer_const(type_name: &str) -> String {
    let mut tightened = String::with_capacity(type_name.len());
    let mut rest = type_name;
    while let Some(star) = rest.find('*') {
        tightened.push_str(&rest[..=star]);
        rest = &rest[star + 1..];
        let trimmed = rest.trim_start();
        let is_const_keyword = trimmed.strip_prefix("const").is_some_and(|tail| {
            !tail.starts_with(|character: char| character.is_alphanumeric() || character == '_')
        });
        if is_const_keyword {
            rest = trimmed;
        }
    }
    tightened.push_str(rest);
    tightened
}

pub(crate) fn normalize_non_type_template_parameter_type(type_name: &str) -> String {
    // Preserve existing behavior for potentially literal-bearing expressions.
    // Whitespace within literals is significant and must not be collapsed.
    if contains_literal(type_name) {
        return type_name.to_string();
    }

    normalize_type_name(&type_name.split_whitespace().collect::<Vec<_>>().join(" "))
}

pub(crate) fn normalize_type_name(type_name: &str) -> String {
    normalize_qualified_name(&tighten_pointer_const(
        &type_name
            .trim()
            .trim_start_matches("::")
            .replace("std::", "")
            .replace(" *", "*")
            .replace(" &", "&"),
    ))
}

pub(crate) fn normalize_qualified_name(reference: &str) -> String {
    // TODO: Remove this workaround once class diagram and implementation parser UIDs use the same namespace separator.
    reference.trim().replace('.', "::")
}
