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

//! Conversion from libclang types to the C++ semantic type model.

#![cfg_attr(test, allow(dead_code))]

use clang::{Entity, EntityKind, Type, TypeKind};
use cpp_semantics::ResolvedType;

use crate::clang_adapter::source_filter;

pub(crate) fn resolve_type(original: &Type) -> ResolvedType {
    // Resolve unqualified structural shape first, then re-apply top-level cv-qualifiers.
    // This keeps qualifier placement consistent across all branches.
    let canonical = original.get_canonical_type();
    let mut resolved = resolve_unqualified_type(original, &canonical);

    if original.is_const_qualified() {
        resolved = ResolvedType::Const(Box::new(resolved));
    }
    if original.is_volatile_qualified() {
        resolved = ResolvedType::Volatile(Box::new(resolved));
    }
    resolved
}

fn resolve_unqualified_type(original: &Type, canonical: &Type) -> ResolvedType {
    // Single source of truth for builtin mapping; extend here when adding builtin support.
    if let Some(name) = builtin_name(original.get_kind()) {
        return ResolvedType::Builtin(name.to_string());
    }

    match original.get_kind() {
        // ===== pointer =====
        TypeKind::Pointer => original
            .get_pointee_type()
            .map(|inner| match resolve_type(&inner) {
                function @ ResolvedType::Function { .. } => {
                    ResolvedType::FunctionPointer(Box::new(function))
                }
                inner => ResolvedType::Pointer(Box::new(inner)),
            })
            .unwrap_or_else(|| unknown(original)),

        // ===== reference =====
        TypeKind::LValueReference => original
            .get_pointee_type()
            .map(|inner| match resolve_type(&inner) {
                function @ ResolvedType::Function { .. } => {
                    ResolvedType::FunctionReference(Box::new(function))
                }
                inner => ResolvedType::Reference(Box::new(inner)),
            })
            .unwrap_or_else(|| unknown(original)),
        TypeKind::RValueReference => original
            .get_pointee_type()
            .map(|inner| ResolvedType::RValueReference(Box::new(resolve_type(&inner))))
            .unwrap_or_else(|| unknown(original)),

        // ===== function =====
        TypeKind::FunctionPrototype | TypeKind::FunctionNoPrototype => {
            resolve_function_type(original)
        }

        // ===== arrays =====
        TypeKind::ConstantArray => ResolvedType::Array {
            element: Box::new(
                original
                    .get_element_type()
                    .map(|element| resolve_type(&element))
                    .unwrap_or_else(|| unknown(original)),
            ),
            size: original.get_size(),
        },

        // ===== user-defined / template =====
        // Named types (including aliases/templates) are resolved through decl-aware fallback.
        _ => resolve_named_type(original, canonical),
    }
}

/// Maps clang `TypeKind` builtin kinds to canonical display names used in this model.
fn builtin_name(kind: TypeKind) -> Option<&'static str> {
    match kind {
        TypeKind::Void => Some("void"),
        TypeKind::Bool => Some("bool"),
        TypeKind::CharS | TypeKind::SChar | TypeKind::UChar => Some("char"),
        TypeKind::Short | TypeKind::UShort => Some("short"),
        TypeKind::Int | TypeKind::UInt => Some("int"),
        TypeKind::Long | TypeKind::ULong => Some("long"),
        TypeKind::LongLong | TypeKind::ULongLong => Some("long long"),
        TypeKind::Float => Some("float"),
        TypeKind::Double => Some("double"),
        _ => None,
    }
}

fn resolve_function_type(original: &Type) -> ResolvedType {
    let return_type = original
        .get_result_type()
        .map(|ty| resolve_type(&ty))
        .unwrap_or_else(|| unknown(original));
    let parameter_types = original
        .get_argument_types()
        .unwrap_or_default()
        .into_iter()
        .map(|ty| resolve_type(&ty))
        .collect();

    ResolvedType::Function {
        return_type: Box::new(return_type),
        parameter_types,
        is_variadic: original.is_variadic(),
    }
}

fn resolve_named_type(original: &Type, canonical: &Type) -> ResolvedType {
    let display_name = original.get_display_name();
    let canonical_name = canonical.get_display_name();

    // For typedef/type-alias, canonical declaration usually yields stable target id.
    // Exception: well-known system/STL aliases (e.g. `std::string`) canonicalize into
    // deep, unreadable implementation-detail templates (`basic_string<char, ...>`) that
    // no one writes in a design diagram -- keep just the alias's own name instead,
    // ignoring any (possibly partially-defaulted) template arguments of its target.
    if is_alias_type(original) {
        if source_filter::is_declared_in_external_or_system_header(original) {
            if let Some(declaration) = original.get_declaration() {
                return ResolvedType::UserDefined(entity_id_from_decl(&declaration));
            }
        } else if let Some(resolved) = resolve_decl_based(canonical) {
            return resolved;
        }
    }

    // Heuristic: an unqualified non-alias source name with a qualified canonical name
    // is likely an imported type; prefer the canonical declaration when possible.
    // This runs after alias handling so an external alias cannot be replaced by an
    // implementation-detail canonical type.
    if !display_name.contains("::") && canonical_name.contains("::") {
        if let Some(resolved) = resolve_decl_based(canonical) {
            return resolved;
        }
    }

    // Fallback order matters:
    // 1) source declaration (preserves local spelling when available)
    // 2) canonical declaration (captures normalized identity)
    // 3) dependent-expression heuristic (e.g. `decltype(expr_using<T>)` inside an
    //    uninstantiated template) — structurally unresolvable before instantiation
    // 4) unknown name heuristic
    resolve_decl_based(original)
        .or_else(|| resolve_decl_based(canonical))
        .unwrap_or_else(|| {
            let name = resolve_unknown_name(original, canonical);
            if is_dependent_expression_type(original) {
                log::debug!(
                    "type '{}' is structurally unresolvable before template instantiation",
                    name
                );
                ResolvedType::Dependent(name)
            } else {
                log::debug!("could not resolve type '{}' to a concrete entity id", name);
                ResolvedType::Unknown(name)
            }
        })
}

/// Detects types libclang exposes as `Unexposed` because their meaning depends on
/// an unbound template parameter, e.g. `decltype(is_x_impl(std::declval<T>()))`
/// in a template that is never instantiated in this translation unit. Such types
/// cannot be resolved to a concrete entity id without template instantiation,
/// which is out of scope for AST-only analysis. This is checked only after both
/// declaration-based resolution attempts have already failed, so it never shadows
/// a legitimately resolvable type.
fn is_dependent_expression_type(ty: &Type) -> bool {
    ty.get_kind() == TypeKind::Unexposed
}

fn resolve_unknown_name(original: &Type, canonical: &Type) -> String {
    let display_name = original.get_display_name();
    let canonical_name = canonical.get_display_name();

    // Prefer canonical only when it provides useful qualification and is not an
    // implementation-detail placeholder (std::__*, type-parameter, auto-parameter).
    if !display_name.contains("::")
        && canonical_name.contains("::")
        && !canonical_name.starts_with("std::__")
        && !canonical_name.contains("type-parameter-")
        && !canonical_name.contains("auto-parameter-")
    {
        canonical_name
    } else {
        display_name
    }
}

fn is_alias_type(ty: &Type) -> bool {
    matches!(
        ty.get_declaration()
            .map(|declaration| declaration.get_kind()),
        Some(EntityKind::TypedefDecl | EntityKind::TypeAliasDecl)
    )
}

fn resolve_decl_based(ty: &Type) -> Option<ResolvedType> {
    // Declaration-derived id is the primary identity source for user-defined types.
    // Template arguments are recursively resolved into the same semantic model.
    let declaration = ty.get_declaration()?;
    let base = entity_id_from_decl(&declaration);
    let args = ty
        .get_template_argument_types()
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .map(|argument| resolve_type(&argument))
        .collect::<Vec<_>>();

    (!args.is_empty())
        .then_some(ResolvedType::Template {
            base: base.clone(),
            args,
        })
        .or(Some(ResolvedType::UserDefined(base)))
}

fn unknown(ty: &Type) -> ResolvedType {
    ResolvedType::Unknown(ty.get_display_name())
}

fn entity_id_from_decl(entity: &Entity) -> String {
    if entity.get_kind() == EntityKind::TemplateTemplateParameter {
        return entity.get_name().unwrap_or_default();
    }
    build_fqn_from_entity(entity)
        .trim_start_matches("::")
        .to_string()
}

/// Collapses implementation-detail namespaces such as `std::__1`.
fn collapse_std_internal_namespaces(parts: Vec<(String, bool)>) -> Vec<String> {
    let mut collapsed = Vec::with_capacity(parts.len());
    for (name, is_namespace) in parts {
        let previous = collapsed.last().map(String::as_str);
        let is_std_internal =
            is_namespace && previous == Some("std") && is_std_internal_namespace_segment(&name);
        if !is_std_internal {
            collapsed.push(name);
        }
    }
    collapsed
}

fn is_std_internal_namespace_segment(name: &str) -> bool {
    name.strip_prefix("__")
        .map(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(false)
}

/// Walk semantic parents of an entity to produce `Namespace::Class::Name`.
fn build_fqn_from_entity(entity: &Entity) -> String {
    // Traversal is semantic (not lexical) so aliases/nested constructs resolve to
    // stable ownership hierarchy used by relationship and id matching.
    let mut parts = Vec::new();
    let mut current = Some(*entity);

    while let Some(entity) = current {
        match entity.get_kind() {
            EntityKind::Namespace => {
                if let Some(name) = entity.get_name() {
                    parts.push((name, true));
                }
            }
            EntityKind::ClassTemplatePartialSpecialization => {
                if let Some(name) = entity.get_display_name().or_else(|| entity.get_name()) {
                    parts.push((name, false));
                }
            }
            EntityKind::ClassDecl
            | EntityKind::StructDecl
            | EntityKind::UnionDecl
            | EntityKind::EnumDecl
            | EntityKind::ClassTemplate
            | EntityKind::TemplateTemplateParameter
            | EntityKind::TypedefDecl
            | EntityKind::TypeAliasDecl => {
                if let Some(name) = entity.get_name() {
                    parts.push((name, false));
                }
            }
            _ => break,
        }
        current = entity.get_semantic_parent();
    }
    parts.reverse();
    collapse_std_internal_namespaces(parts).join("::")
}

#[cfg(test)]
mod tests {
    use super::collapse_std_internal_namespaces;

    #[test]
    fn collapses_std_internal_namespaces_only_under_std() {
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
    fn preserves_non_std_internal_namespaces() {
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
}
