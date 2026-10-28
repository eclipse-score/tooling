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

//! C++ signature text of API items.

use class_diagram::{FunctionArgument, MethodModifier, TemplateParameter};

/// Everything a signature is made of.
pub(crate) struct SignatureParts<'a> {
    /// Name including its scope (`ns::Cls<T>::method`).
    pub qualified_name: &'a str,
    pub return_type: Option<&'a str>,
    pub parameters: &'a [FunctionArgument],
    pub template_parameters: Option<&'a [TemplateParameter]>,
    pub modifiers: &'a [MethodModifier],
}

pub(crate) fn render_signature(parts: &SignatureParts) -> String {
    let mut out = String::new();
    if parts.modifiers.contains(&MethodModifier::Static) {
        out.push_str("static ");
    }
    if let Some(return_type) = parts.return_type {
        out.push_str(return_type);
        out.push(' ');
    }
    out.push_str(parts.qualified_name);
    if let Some(parameters) = parts.template_parameters {
        out.push_str(&render_template_parameters(parameters));
    }
    let rendered: Vec<String> = parts.parameters.iter().map(render_parameter).collect();
    out.push_str(&format!("({})", rendered.join(", ")));
    if parts.modifiers.contains(&MethodModifier::Noexcept) {
        out.push_str(" noexcept");
    }
    out
}

/// `<T, N, Ts...>`: the parameter names, as used in a template-id.
pub(crate) fn render_template_parameters(parameters: &[TemplateParameter]) -> String {
    let rendered: Vec<String> = parameters.iter().map(render_template_parameter).collect();
    format!("<{}>", rendered.join(", "))
}

fn render_template_parameter(parameter: &TemplateParameter) -> String {
    let pack = match parameter {
        TemplateParameter::Type { is_pack, .. }
        | TemplateParameter::NonType { is_pack, .. }
        | TemplateParameter::Template { is_pack, .. } => *is_pack,
    };
    format!("{}{}", parameter.name(), if pack { "..." } else { "" })
}

fn render_parameter(parameter: &FunctionArgument) -> String {
    let mut ty = parameter.param_type.clone().unwrap_or_default();
    if parameter.is_variadic || parameter.is_pack_expansion {
        ty.push_str("...");
    }
    [ty.as_str(), parameter.name.as_str()]
        .iter()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}
