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

use serde::{Deserialize, Serialize};

pub type EntityId = String;

/// Language-level representation of a C++ type after libclang extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolvedType {
    Builtin(String),
    UserDefined(EntityId),
    Template {
        base: EntityId,
        args: Vec<ResolvedType>,
    },
    Function {
        return_type: Box<ResolvedType>,
        parameter_types: Vec<ResolvedType>,
        is_variadic: bool,
    },
    FunctionPointer(Box<ResolvedType>),
    FunctionReference(Box<ResolvedType>),
    Pointer(Box<ResolvedType>),
    Reference(Box<ResolvedType>),
    RValueReference(Box<ResolvedType>),
    Const(Box<ResolvedType>),
    Volatile(Box<ResolvedType>),
    Array {
        element: Box<ResolvedType>,
        size: Option<usize>,
    },
    Unknown(String),
    /// A type that is structurally unresolvable without template instantiation,
    /// e.g. `decltype(some_trait_impl(std::declval<T>()))` inside an
    /// uninstantiated template (the common SFINAE/trait-detection idiom). This is
    /// distinct from `Unknown`: it is an expected, permanent limitation of
    /// AST-only analysis rather than a gap in the resolver, so callers should not
    /// treat it as an error condition.
    Dependent(String),
}

impl ResolvedType {
    /// Returns whether this type should be treated as non-owning for relationship inference.
    ///
    /// Notes:
    /// - Pointer/reference/function-pointer wrappers are always non-owning.
    /// - Qualifiers and containers recurse into the wrapped/contained type.
    /// - A few standard wrappers are modeled as non-owning by policy.
    pub fn is_non_owning(&self) -> bool {
        match self {
            Self::Pointer(_)
            | Self::Reference(_)
            | Self::RValueReference(_)
            | Self::FunctionPointer(_)
            | Self::FunctionReference(_) => true,
            Self::Const(inner) | Self::Volatile(inner) => inner.is_non_owning(),
            Self::Function {
                return_type,
                parameter_types,
                ..
            } => {
                return_type.is_non_owning()
                    || parameter_types.iter().any(ResolvedType::is_non_owning)
            }
            Self::Array { element, .. } => element.is_non_owning(),
            Self::Template { base, args } => {
                matches!(
                    base.trim_start_matches("::"),
                    "std::weak_ptr"
                        | "std::shared_ptr"
                        | "std::reference_wrapper"
                        | "std::observer_ptr"
                ) || args.iter().any(ResolvedType::is_non_owning)
            }
            _ => false,
        }
    }

    /// Extracts a candidate relationship target entity id from a resolved type tree.
    ///
    /// Traversal policy:
    /// - Template prefers first resolvable argument, then falls back to template base.
    ///   This intentionally keeps relationship targets at the template-family
    ///   level for now, even when the model also contains partial-specialization
    ///   entities with more specific ids.
    /// - Function prefers return type, then parameter types.
    /// - Wrapper/qualifier/array nodes delegate to their inner element.
    pub fn relationship_target_entity_id(&self) -> Option<&str> {
        match self {
            Self::Builtin(_) | Self::Unknown(_) | Self::Dependent(_) => None,
            Self::UserDefined(id) => Some(id),
            Self::Template { base, args } => args
                .iter()
                .find_map(ResolvedType::relationship_target_entity_id)
                .or(Some(base)),
            Self::Function {
                return_type,
                parameter_types,
                ..
            } => return_type.relationship_target_entity_id().or_else(|| {
                parameter_types
                    .iter()
                    .find_map(ResolvedType::relationship_target_entity_id)
            }),
            Self::FunctionPointer(inner)
            | Self::FunctionReference(inner)
            | Self::Pointer(inner)
            | Self::Reference(inner)
            | Self::RValueReference(inner)
            | Self::Const(inner)
            | Self::Volatile(inner) => inner.relationship_target_entity_id(),
            Self::Array { element, .. } => element.relationship_target_entity_id(),
        }
    }

    /// Returns a direct referenced entity id for base-type style lookups.
    ///
    /// Unlike `relationship_target_entity_id`, this intentionally keeps template-base
    /// semantics for inheritance resolution and does not attempt to target a
    /// particular partial specialization entity.
    pub fn referenced_entity_id(&self) -> Option<&str> {
        match self.referenced_entity_root() {
            Self::UserDefined(id) => Some(id),
            Self::Template { base, .. } => Some(base),
            _ => None,
        }
    }

    /// Unwraps qualifiers/wrappers to the core entity-bearing node.
    ///
    /// This helper is used by `referenced_entity_id` so that ownership/indirection
    /// wrappers do not affect base-type lookup.
    fn referenced_entity_root(&self) -> &ResolvedType {
        match self {
            Self::FunctionPointer(inner)
            | Self::FunctionReference(inner)
            | Self::Pointer(inner)
            | Self::Reference(inner)
            | Self::RValueReference(inner)
            | Self::Const(inner)
            | Self::Volatile(inner) => inner.referenced_entity_root(),
            Self::Array { element, .. } => element.referenced_entity_root(),
            _ => self,
        }
    }

    pub fn render_for_display(&self) -> String {
        normalize_pointer_reference_spacing(self.render())
    }

    fn render(&self) -> String {
        match self {
            Self::Builtin(name)
            | Self::UserDefined(name)
            | Self::Unknown(name)
            | Self::Dependent(name) => name.clone(),
            Self::Template { base, args } => format!(
                "{base}<{}>",
                args.iter().map(Self::render).collect::<Vec<_>>().join(", ")
            ),
            Self::Function {
                return_type,
                parameter_types,
                is_variadic,
            } => {
                let mut parameters = parameter_types.iter().map(Self::render).collect::<Vec<_>>();
                if *is_variadic {
                    parameters.push("...".to_string());
                }
                format!("{}({})", return_type.render(), parameters.join(", "))
            }
            Self::FunctionPointer(inner) => render_function_wrapper(inner, "*"),
            Self::FunctionReference(inner) => render_function_wrapper(inner, "&"),
            Self::Pointer(inner) => format!("{}*", inner.render()),
            Self::Reference(inner) => format!("{}&", inner.render()),
            Self::RValueReference(inner) => format!("{}&&", inner.render()),
            Self::Const(inner) => match inner.as_ref() {
                Self::Pointer(pointee) => format!("{}*const", pointee.render()),
                _ => format!("const {}", inner.render()),
            },
            Self::Volatile(inner) => format!("volatile {}", inner.render()),
            Self::Array { element, size } => match size {
                Some(size) => format!("{}[{size}]", element.render()),
                None => format!("{}[]", element.render()),
            },
        }
    }
}

fn render_function_wrapper(inner: &ResolvedType, marker: &str) -> String {
    if let ResolvedType::Function {
        return_type,
        parameter_types,
        is_variadic,
    } = inner
    {
        let mut parameters = parameter_types
            .iter()
            .map(ResolvedType::render)
            .collect::<Vec<_>>();
        if *is_variadic {
            parameters.push("...".to_string());
        }
        format!(
            "{} ({marker})({})",
            return_type.render(),
            parameters.join(", ")
        )
    } else {
        format!("{}{marker}", inner.render())
    }
}

/// Normalizes spacing before pointer and reference markers for display.
///
/// Rules:
/// - Insert one space before the first marker in a consecutive `*` or `&` sequence.
/// - Keep subsequent markers adjacent, preserving `**` and `&&`.
/// - Preserve function pointer/reference markers in `(*)` and `(&)`, whose
///   parentheses already provide separation.
///
/// Examples:
//     `Type*`  -> `Type *`
//     `Type**` -> `Type **`
///    `Type&&` -> `Type &&`
///    `void(*)(T)` -> `void (*)(T)`
fn normalize_pointer_reference_spacing(type_name: String) -> String {
    let chars: Vec<char> = type_name.chars().collect();
    let mut output = String::with_capacity(type_name.len() + 8);

    for (index, character) in chars.iter().copied().enumerate() {
        if is_pointer_or_reference(character) && !is_function_pointer_marker(&chars, index) {
            insert_space_before_pointer_marker(&mut output, &chars, index);
        }
        output.push(character);
    }

    output
}

fn is_pointer_or_reference(character: char) -> bool {
    matches!(character, '*' | '&')
}

fn is_function_pointer_marker(characters: &[char], index: usize) -> bool {
    matches!(characters.get(index), Some('*' | '&'))
        && matches!(
            (
                index.checked_sub(1).and_then(|i| characters.get(i)),
                characters.get(index + 1)
            ),
            (Some('('), Some(')'))
        )
}

/// Adds a separator before the first marker unless one is already present.
fn insert_space_before_pointer_marker(output: &mut String, characters: &[char], index: usize) {
    let previous_input = index
        .checked_sub(1)
        .and_then(|i| characters.get(i))
        .copied();

    let is_first_marker = !matches!(previous_input, Some('*' | '&'));
    if is_first_marker && !output.ends_with(' ') && !output.ends_with('(') {
        output.push(' ');
    }
}

#[cfg(test)]
mod tests {
    use super::ResolvedType;

    #[test]
    fn resolves_referenced_entities_through_wrappers() {
        let wrapped = ResolvedType::Const(Box::new(ResolvedType::Pointer(Box::new(
            ResolvedType::UserDefined("Vehicle::Engine".to_string()),
        ))));
        assert_eq!(wrapped.referenced_entity_id(), Some("Vehicle::Engine"));

        let function_pointer =
            ResolvedType::FunctionPointer(Box::new(ResolvedType::Const(Box::new(
                ResolvedType::Pointer(Box::new(ResolvedType::UserDefined("Engine".to_string()))),
            ))));
        assert_eq!(function_pointer.referenced_entity_id(), Some("Engine"));

        let template = ResolvedType::Reference(Box::new(ResolvedType::Template {
            base: "std::vector".to_string(),
            args: vec![ResolvedType::UserDefined("Vehicle::Engine".to_string())],
        }));
        assert_eq!(template.referenced_entity_id(), Some("std::vector"));
    }

    #[test]
    fn relationship_target_prefers_template_argument() {
        let ty = ResolvedType::Template {
            base: "std::vector".to_string(),
            args: vec![ResolvedType::UserDefined("Vehicle::Engine".to_string())],
        };
        assert_eq!(ty.relationship_target_entity_id(), Some("Vehicle::Engine"));
    }

    #[test]
    fn renders_composite_types() {
        let pointer = ResolvedType::Pointer(Box::new(ResolvedType::UserDefined(
            "MyNamespace::Engine".to_string(),
        )));
        assert_eq!(pointer.render_for_display(), "MyNamespace::Engine *");

        let array = ResolvedType::Array {
            element: Box::new(ResolvedType::Builtin("int".to_string())),
            size: Some(8),
        };
        assert_eq!(array.render_for_display(), "int[8]");
    }

    #[test]
    fn renders_qualified_and_callable_types() {
        let const_pointer = ResolvedType::Const(Box::new(ResolvedType::Pointer(Box::new(
            ResolvedType::Builtin("int".to_string()),
        ))));
        assert_eq!(const_pointer.render_for_display(), "int *const");

        let function = ResolvedType::Function {
            return_type: Box::new(ResolvedType::Builtin("void".to_string())),
            parameter_types: vec![ResolvedType::UserDefined("Engine".to_string())],
            is_variadic: false,
        };
        assert_eq!(
            ResolvedType::FunctionPointer(Box::new(function)).render_for_display(),
            "void (*)(Engine)"
        );
    }

    #[test]
    fn identifies_non_owning_and_dependent_types() {
        let shared_ptr = ResolvedType::Template {
            base: "std::shared_ptr".to_string(),
            args: vec![ResolvedType::UserDefined("Engine".to_string())],
        };
        assert!(shared_ptr.is_non_owning());

        let dependent = ResolvedType::Dependent("decltype(foo(std::declval<T>()))".to_string());
        assert_eq!(dependent.referenced_entity_id(), None);
        assert_eq!(dependent.relationship_target_entity_id(), None);
        assert!(!dependent.is_non_owning());
        assert_eq!(
            dependent.render_for_display(),
            "decltype(foo(std::declval<T>()))"
        );
    }
}
