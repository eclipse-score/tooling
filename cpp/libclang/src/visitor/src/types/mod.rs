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

//! Conversion and display rules for C++ types.

pub(crate) mod renderer;
pub(crate) mod resolver;

use class_diagram::{TemplateInstantiation, TypeDecorator};
use cpp_semantics::ResolvedType;

/// Extracts only the outer template, without interpreting diagnostic strings.
pub(crate) fn template_instantiation(resolved: &ResolvedType) -> Option<TemplateInstantiation> {
    let mut root = resolved;
    let mut decorators = Vec::new();
    loop {
        let (decorator, inner) = match root {
            ResolvedType::Const(inner) => (TypeDecorator::Const, inner),
            ResolvedType::Volatile(inner) => (TypeDecorator::Volatile, inner),
            ResolvedType::Pointer(inner) => (TypeDecorator::Pointer, inner),
            ResolvedType::Reference(inner) => (TypeDecorator::LValueReference, inner),
            ResolvedType::RValueReference(inner) => (TypeDecorator::RValueReference, inner),
            _ => break,
        };
        decorators.push(decorator);
        root = inner;
    }
    decorators.reverse();
    TypeDecorator::canonicalize(&mut decorators);
    let ResolvedType::Template { base, args } = root else {
        return None;
    };
    args.iter()
        .all(is_supported_argument)
        .then(|| TemplateInstantiation {
            name: base.clone(),
            arguments: args.iter().map(ResolvedType::render_for_display).collect(),
            outer_decorators: Some(decorators),
        })
}

/// Validate the whole argument tree before rendering any metadata.
fn is_supported_argument(resolved: &ResolvedType) -> bool {
    match resolved {
        ResolvedType::Builtin(_) | ResolvedType::UserDefined(_) => true,
        ResolvedType::Template { args, .. } => args.iter().all(is_supported_argument),
        ResolvedType::Const(inner)
        | ResolvedType::Volatile(inner)
        | ResolvedType::Pointer(inner)
        | ResolvedType::Reference(inner)
        | ResolvedType::RValueReference(inner) => is_supported_argument(inner),
        ResolvedType::Unknown(_)
        | ResolvedType::Dependent(_)
        | ResolvedType::Function { .. }
        | ResolvedType::FunctionPointer(_)
        | ResolvedType::FunctionReference(_)
        | ResolvedType::Array { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::template_instantiation;
    use class_diagram::TemplateInstantiation;
    use cpp_semantics::ResolvedType;

    #[test]
    fn emits_no_metadata_for_named_types_without_parsing_names() {
        for resolved in [
            ResolvedType::Builtin("unsigned long".into()),
            ResolvedType::UserDefined("ns::Widget<int, float>".into()),
        ] {
            assert_eq!(template_instantiation(&resolved), None);
        }
    }

    #[test]
    fn records_outer_wrappers_and_preserves_argument_modifiers_and_order() {
        let resolved = ResolvedType::Template {
            base: "ns::Pair".into(),
            args: vec![
                ResolvedType::Const(Box::new(ResolvedType::Pointer(Box::new(
                    ResolvedType::Builtin("int".into()),
                )))),
                ResolvedType::Template {
                    base: "ns::Box".into(),
                    args: vec![ResolvedType::Reference(Box::new(ResolvedType::Volatile(
                        Box::new(ResolvedType::UserDefined("ns::Widget".into())),
                    )))],
                },
            ],
        };
        let expected = Some(TemplateInstantiation {
            name: "ns::Pair".into(),
            arguments: vec!["int *const".into(), "ns::Box<volatile ns::Widget &>".into()],
            outer_decorators: Some(vec![]),
        });
        assert_eq!(template_instantiation(&resolved), expected);
        let wrapped = ResolvedType::Const(Box::new(ResolvedType::Volatile(Box::new(
            ResolvedType::Pointer(Box::new(ResolvedType::Reference(Box::new(
                ResolvedType::RValueReference(Box::new(resolved)),
            )))),
        ))));
        let mut decorated = expected.clone();
        decorated.as_mut().unwrap().outer_decorators = Some(vec![
            class_diagram::TypeDecorator::RValueReference,
            class_diagram::TypeDecorator::LValueReference,
            class_diagram::TypeDecorator::Pointer,
            class_diagram::TypeDecorator::Const,
            class_diagram::TypeDecorator::Volatile,
        ]);
        assert_eq!(template_instantiation(&wrapped), decorated);
        assert_eq!(
            template_instantiation(&ResolvedType::Template {
                base: "Empty".into(),
                args: vec![],
            }),
            Some(TemplateInstantiation {
                name: "Empty".into(),
                arguments: vec![],
                outer_decorators: Some(vec![]),
            })
        );
    }

    #[test]
    fn unsupported_nodes_invalidate_the_entire_tree() {
        let function = ResolvedType::Function {
            return_type: Box::new(ResolvedType::Builtin("void".into())),
            parameter_types: vec![],
            is_variadic: false,
            is_noexcept: false,
        };
        for unsupported in [
            ResolvedType::Unknown("unknown".into()),
            ResolvedType::Dependent("T::type".into()),
            function.clone(),
            ResolvedType::FunctionPointer(Box::new(function.clone())),
            ResolvedType::FunctionReference(Box::new(function)),
            ResolvedType::Array {
                element: Box::new(ResolvedType::Builtin("int".into())),
                size: Some(3),
            },
        ] {
            assert_eq!(template_instantiation(&unsupported), None);
            let wrapped =
                ResolvedType::Const(Box::new(ResolvedType::Pointer(Box::new(unsupported))));
            assert_eq!(template_instantiation(&wrapped), None);
            let nested = ResolvedType::Template {
                base: "Pair".into(),
                args: vec![
                    ResolvedType::Builtin("int".into()),
                    ResolvedType::Template {
                        base: "Box".into(),
                        args: vec![wrapped],
                    },
                ],
            };
            assert_eq!(template_instantiation(&nested), None);
        }
    }
}
