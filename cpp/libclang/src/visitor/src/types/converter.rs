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

use class_diagram::{QualifiedTypeName, TypeArgument, TypeDescription, TypeExpr, TypeNameSegment};
use cpp_semantics::{ResolvedNameSegment, ResolvedTemplateArgument, ResolvedType};

pub(crate) fn describe_type(raw_text: String, resolved: &ResolvedType) -> TypeDescription {
    TypeDescription {
        raw_text,
        expression: to_type_expr(resolved),
    }
}

fn to_type_expr(resolved: &ResolvedType) -> Option<TypeExpr> {
    Some(match resolved {
        ResolvedType::Builtin(name) => TypeExpr::Builtin(name.clone()),
        ResolvedType::UserDefined(name) | ResolvedType::TemplateTypeParameter(name) => {
            TypeExpr::NamedType(qualified_name(name)?)
        }
        ResolvedType::Template { base, args } => TypeExpr::Template {
            name: qualified_name(base)?,
            arguments: type_arguments(args)?,
        },
        ResolvedType::Qualified { segments, .. } => TypeExpr::NamedType(QualifiedTypeName {
            segments: segments
                .iter()
                .map(|segment| match segment {
                    ResolvedNameSegment::Identifier(name) if is_identifier(name) => {
                        Some(TypeNameSegment::Identifier(name.clone()))
                    }
                    ResolvedNameSegment::Template { name, args } if is_identifier(name) => {
                        Some(TypeNameSegment::Template {
                            name: name.clone(),
                            arguments: type_arguments(args)?,
                        })
                    }
                    _ => None,
                })
                .collect::<Option<_>>()?,
            is_global: false,
        }),
        ResolvedType::Function {
            return_type,
            parameter_types,
            is_variadic,
            is_noexcept,
        } => TypeExpr::Function {
            return_type: Box::new(to_type_expr(return_type)?),
            parameter_types: parameter_types
                .iter()
                .map(to_type_expr)
                .collect::<Option<_>>()?,
            is_variadic: *is_variadic,
            is_noexcept: *is_noexcept,
        },
        ResolvedType::FunctionPointer(inner) | ResolvedType::Pointer(inner) => {
            TypeExpr::Pointer(Box::new(to_type_expr(inner)?))
        }
        ResolvedType::FunctionReference(inner) | ResolvedType::Reference(inner) => {
            TypeExpr::Reference(Box::new(to_type_expr(inner)?))
        }
        ResolvedType::RValueReference(inner) => {
            TypeExpr::RValueReference(Box::new(to_type_expr(inner)?))
        }
        ResolvedType::Const(inner) => TypeExpr::Const(Box::new(to_type_expr(inner)?)),
        ResolvedType::Volatile(inner) => TypeExpr::Volatile(Box::new(to_type_expr(inner)?)),
        ResolvedType::Array { element, size } => TypeExpr::Array {
            element: Box::new(to_type_expr(element)?),
            size: size.map(|size| size.to_string()),
        },
        ResolvedType::Unknown(_) | ResolvedType::Dependent(_) => return None,
    })
}

fn type_arguments(args: &[ResolvedTemplateArgument]) -> Option<Vec<TypeArgument>> {
    args.iter()
        .map(|argument| match argument {
            ResolvedTemplateArgument::Type(ty) => to_type_expr(ty).map(TypeArgument::Type),
            ResolvedTemplateArgument::Value(value) => Some(TypeArgument::Value(value.clone())),
        })
        .collect()
}

fn is_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    characters.next().is_some_and(|first| {
        (first.is_ascii_alphabetic() || first == '_')
            && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
    })
}

fn qualified_name(name: &str) -> Option<QualifiedTypeName> {
    let segments = name.strip_prefix("::").unwrap_or(name).split("::");
    let segments = segments
        .map(|segment| is_identifier(segment).then(|| segment.into()))
        .collect::<Option<_>>()?;
    Some(QualifiedTypeName {
        segments,
        is_global: name.starts_with("::"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_each_template_owner_without_changing_raw_text() {
        let resolved = ResolvedType::Qualified {
            declaration_id: "demo::Owner::Nested".into(),
            segments: vec![
                ResolvedNameSegment::Identifier("demo".into()),
                ResolvedNameSegment::Template {
                    name: "Owner".into(),
                    args: vec![ResolvedTemplateArgument::Type(ResolvedType::Builtin(
                        "double".into(),
                    ))],
                },
                ResolvedNameSegment::Identifier("Nested".into()),
            ],
        };
        let description = describe_type("Owner<double>::Nested".into(), &resolved);
        assert_eq!(description.raw_text, "Owner<double>::Nested");
        assert_eq!(
            description.expression,
            Some(TypeExpr::NamedType(QualifiedTypeName {
                segments: vec![
                    "demo".into(),
                    TypeNameSegment::Template {
                        name: "Owner".into(),
                        arguments: vec![TypeArgument::Type(TypeExpr::Builtin("double".into()))],
                    },
                    "Nested".into(),
                ],
                is_global: false,
            }))
        );
    }

    #[test]
    fn describes_structured_types_without_rendering_raw_text() {
        let builtin = ResolvedType::Builtin("int".into());
        let const_pointer = ResolvedType::Const(Box::new(ResolvedType::Pointer(Box::new(
            ResolvedType::Volatile(Box::new(builtin.clone())),
        ))));
        let function = ResolvedType::Function {
            return_type: Box::new(builtin.clone()),
            parameter_types: vec![ResolvedType::Reference(Box::new(builtin.clone()))],
            is_variadic: true,
            is_noexcept: true,
        };
        let function_expr = TypeExpr::Function {
            return_type: Box::new(TypeExpr::Builtin("int".into())),
            parameter_types: vec![TypeExpr::Reference(Box::new(TypeExpr::Builtin(
                "int".into(),
            )))],
            is_variadic: true,
            is_noexcept: true,
        };
        for (raw_text, resolved, expected) in [
            (
                "volatile int * const",
                const_pointer,
                TypeExpr::Const(Box::new(TypeExpr::Pointer(Box::new(TypeExpr::Volatile(
                    Box::new(TypeExpr::Builtin("int".into())),
                ))))),
            ),
            (
                "int[3][8]",
                ResolvedType::Array {
                    element: Box::new(ResolvedType::Array {
                        element: Box::new(builtin.clone()),
                        size: Some(8),
                    }),
                    size: Some(3),
                },
                TypeExpr::Array {
                    element: Box::new(TypeExpr::Array {
                        element: Box::new(TypeExpr::Builtin("int".into())),
                        size: Some("8".into()),
                    }),
                    size: Some("3".into()),
                },
            ),
            (
                "int (*)(int &, ...) noexcept",
                ResolvedType::FunctionPointer(Box::new(function.clone())),
                TypeExpr::Pointer(Box::new(function_expr.clone())),
            ),
            (
                "int (&)(int &, ...) noexcept",
                ResolvedType::FunctionReference(Box::new(function)),
                TypeExpr::Reference(Box::new(function_expr)),
            ),
            (
                "::vehicle::Buffer<int>",
                ResolvedType::Template {
                    base: "::vehicle::Buffer".into(),
                    args: vec![ResolvedTemplateArgument::Type(builtin)],
                },
                TypeExpr::Template {
                    name: QualifiedTypeName {
                        segments: vec!["vehicle".into(), "Buffer".into()],
                        is_global: true,
                    },
                    arguments: vec![TypeArgument::Type(TypeExpr::Builtin("int".into()))],
                },
            ),
            (
                "Payload &&",
                ResolvedType::RValueReference(Box::new(ResolvedType::UserDefined(
                    "vehicle::Payload".into(),
                ))),
                TypeExpr::RValueReference(Box::new(TypeExpr::NamedType(QualifiedTypeName {
                    segments: vec!["vehicle".into(), "Payload".into()],
                    is_global: false,
                }))),
            ),
            (
                "T",
                ResolvedType::TemplateTypeParameter("T".into()),
                TypeExpr::NamedType(QualifiedTypeName {
                    segments: vec!["T".into()],
                    is_global: false,
                }),
            ),
            (
                "const T &",
                ResolvedType::Reference(Box::new(ResolvedType::Const(Box::new(
                    ResolvedType::TemplateTypeParameter("T".into()),
                )))),
                TypeExpr::Reference(Box::new(TypeExpr::Const(Box::new(TypeExpr::NamedType(
                    QualifiedTypeName {
                        segments: vec!["T".into()],
                        is_global: false,
                    },
                ))))),
            ),
            (
                "Buffer<T>",
                ResolvedType::Template {
                    base: "Buffer".into(),
                    args: vec![ResolvedTemplateArgument::Type(
                        ResolvedType::TemplateTypeParameter("T".into()),
                    )],
                },
                TypeExpr::Template {
                    name: QualifiedTypeName {
                        segments: vec!["Buffer".into()],
                        is_global: false,
                    },
                    arguments: vec![TypeArgument::Type(TypeExpr::NamedType(QualifiedTypeName {
                        segments: vec!["T".into()],
                        is_global: false,
                    }))],
                },
            ),
            (
                "Mixed<int, 3, float>",
                ResolvedType::Template {
                    base: "Mixed".into(),
                    args: vec![
                        ResolvedTemplateArgument::Type(ResolvedType::Builtin("int".into())),
                        ResolvedTemplateArgument::Value("3".into()),
                        ResolvedTemplateArgument::Type(ResolvedType::Builtin("float".into())),
                    ],
                },
                TypeExpr::Template {
                    name: QualifiedTypeName {
                        segments: vec!["Mixed".into()],
                        is_global: false,
                    },
                    arguments: vec![
                        TypeArgument::Type(TypeExpr::Builtin("int".into())),
                        TypeArgument::Value("3".into()),
                        TypeArgument::Type(TypeExpr::Builtin("float".into())),
                    ],
                },
            ),
        ] {
            assert_eq!(
                describe_type(raw_text.into(), &resolved),
                TypeDescription {
                    raw_text: raw_text.into(),
                    expression: Some(expected),
                },
            );
        }
    }

    #[test]
    fn unsupported_type_parts_preserve_only_raw_text() {
        for resolved in [
            ResolvedType::Unknown("Mixed<int, 3, float>".into()),
            ResolvedType::Dependent("T".into()),
            ResolvedType::Dependent("decltype(value)".into()),
            ResolvedType::Dependent("Missing".into()),
            ResolvedType::Pointer(Box::new(ResolvedType::Unknown("Missing".into()))),
            ResolvedType::Template {
                base: "Owner<double>::Mixed".into(),
                args: vec![ResolvedTemplateArgument::Type(ResolvedType::Builtin(
                    "int".into(),
                ))],
            },
            ResolvedType::Template {
                base: "Buffer".into(),
                args: vec![ResolvedTemplateArgument::Type(ResolvedType::Unknown(
                    "Missing".into(),
                ))],
            },
        ] {
            assert_eq!(
                describe_type("original type spelling".into(), &resolved),
                TypeDescription {
                    raw_text: "original type spelling".into(),
                    expression: None,
                },
            );
        }
    }
}
