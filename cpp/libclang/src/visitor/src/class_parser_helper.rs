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
#![cfg_attr(test, allow(dead_code))]

use clang::{Entity, EntityKind, Type, TypeKind};
use serde::{Deserialize, Serialize};

pub type EntityId = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            ResolvedType::Pointer(_)
            | ResolvedType::Reference(_)
            | ResolvedType::RValueReference(_)
            | ResolvedType::FunctionPointer(_)
            | ResolvedType::FunctionReference(_) => true,
            ResolvedType::Const(inner) | ResolvedType::Volatile(inner) => inner.is_non_owning(),
            ResolvedType::Function {
                return_type,
                parameter_types,
                ..
            } => {
                return_type.is_non_owning()
                    || parameter_types.iter().any(ResolvedType::is_non_owning)
            }
            ResolvedType::Array { element, .. } => element.is_non_owning(),
            ResolvedType::Template { base, args } => {
                let normalized_base = base.trim_start_matches("::");
                matches!(
                    normalized_base,
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
    /// - Function prefers return type, then parameter types.
    /// - Wrapper/qualifier/array nodes delegate to their inner element.
    pub fn relationship_target_entity_id(&self) -> Option<&str> {
        match self {
            ResolvedType::Builtin(_) | ResolvedType::Unknown(_) | ResolvedType::Dependent(_) => {
                None
            }
            ResolvedType::UserDefined(id) => Some(id),
            ResolvedType::Template { base, args } => args
                .iter()
                .find_map(ResolvedType::relationship_target_entity_id)
                .or_else(|| Some(base)),
            ResolvedType::Function {
                return_type,
                parameter_types,
                ..
            } => return_type.relationship_target_entity_id().or_else(|| {
                parameter_types
                    .iter()
                    .find_map(ResolvedType::relationship_target_entity_id)
            }),
            ResolvedType::FunctionPointer(inner)
            | ResolvedType::FunctionReference(inner)
            | ResolvedType::Pointer(inner)
            | ResolvedType::Reference(inner)
            | ResolvedType::RValueReference(inner)
            | ResolvedType::Const(inner)
            | ResolvedType::Volatile(inner) => inner.relationship_target_entity_id(),
            ResolvedType::Array { element, .. } => element.relationship_target_entity_id(),
        }
    }

    /// Returns a direct referenced entity id for base-type style lookups.
    ///
    /// Unlike `relationship_target_entity_id`, this intentionally keeps template-base
    /// semantics for inheritance resolution.
    pub fn referenced_entity_id(&self) -> Option<&str> {
        match self.referenced_entity_root() {
            ResolvedType::UserDefined(id) => Some(id),
            ResolvedType::Template { base, .. } => Some(base),
            _ => None,
        }
    }

    /// Unwraps qualifiers/wrappers to the core entity-bearing node.
    ///
    /// This helper is used by `referenced_entity_id` so that ownership/indirection
    /// wrappers do not affect base-type lookup.
    fn referenced_entity_root(&self) -> &ResolvedType {
        match self {
            ResolvedType::FunctionPointer(inner)
            | ResolvedType::FunctionReference(inner)
            | ResolvedType::Pointer(inner)
            | ResolvedType::Reference(inner)
            | ResolvedType::RValueReference(inner)
            | ResolvedType::Const(inner)
            | ResolvedType::Volatile(inner) => inner.referenced_entity_root(),

            ResolvedType::Array { element, .. } => element.referenced_entity_root(),

            _ => self,
        }
    }

    fn render(&self) -> String {
        match self {
            ResolvedType::Builtin(name) => name.clone(),

            ResolvedType::UserDefined(id) => id.clone(),

            ResolvedType::Template { base, args } => {
                let args = args
                    .iter()
                    .map(|arg| arg.render())
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{base}<{args}>")
            }

            ResolvedType::Function {
                return_type,
                parameter_types,
                is_variadic,
            } => {
                let mut params = parameter_types
                    .iter()
                    .map(|p| p.render())
                    .collect::<Vec<_>>();
                if *is_variadic {
                    params.push("...".to_string());
                }
                format!("{}({})", return_type.render(), params.join(", "))
            }

            ResolvedType::FunctionPointer(inner) => {
                if let ResolvedType::Function {
                    return_type,
                    parameter_types,
                    is_variadic,
                } = inner.as_ref()
                {
                    let mut params = parameter_types
                        .iter()
                        .map(|p| p.render())
                        .collect::<Vec<_>>();
                    if *is_variadic {
                        params.push("...".to_string());
                    }
                    format!("{} (*)({})", return_type.render(), params.join(", "))
                } else {
                    format!("{}*", inner.render())
                }
            }

            ResolvedType::FunctionReference(inner) => {
                if let ResolvedType::Function {
                    return_type,
                    parameter_types,
                    is_variadic,
                } = inner.as_ref()
                {
                    let mut params = parameter_types
                        .iter()
                        .map(|p| p.render())
                        .collect::<Vec<_>>();
                    if *is_variadic {
                        params.push("...".to_string());
                    }
                    format!("{} (&)({})", return_type.render(), params.join(", "))
                } else {
                    format!("{}&", inner.render())
                }
            }

            ResolvedType::Pointer(inner) => {
                format!("{}*", inner.render())
            }

            ResolvedType::Reference(inner) => {
                format!("{}&", inner.render())
            }

            ResolvedType::RValueReference(inner) => {
                format!("{}&&", inner.render())
            }

            ResolvedType::Const(inner) => match inner.as_ref() {
                ResolvedType::Pointer(pointee) => format!("{}*const", pointee.render()),
                _ => format!("const {}", inner.render()),
            },

            ResolvedType::Volatile(inner) => {
                format!("volatile {}", inner.render())
            }

            ResolvedType::Array { element, size } => match size {
                Some(n) => format!("{}[{}]", element.render(), n),
                None => format!("{}[]", element.render()),
            },

            ResolvedType::Unknown(s) => s.clone(),

            ResolvedType::Dependent(s) => s.clone(),
        }
    }

    pub fn render_for_display(&self) -> String {
        normalize_pointer_reference_spacing(self.render())
    }
}

pub fn render_type_for_display(original: &Type, resolved: &ResolvedType) -> String {
    // Prefer source spelling only in carefully scoped cases (see helper below);
    // otherwise use normalized rendering from semantic type model.
    if should_prefer_source_display_name(original, resolved) {
        original.get_display_name()
    } else {
        resolved.render_for_display()
    }
}

pub(crate) fn should_prefer_source_display_name(ty: &Type, resolved: &ResolvedType) -> bool {
    // Source display names are used for externally declared/system types where
    // canonicalized rendering may be less readable for users.
    if !is_declared_in_external_or_system_header(ty) {
        return false;
    }

    // Keep template output stable/normalized via `ResolvedType` renderer.
    if contains_template_type(resolved) {
        return false;
    }

    let source_display = ty.get_display_name();
    let rendered = resolved.render();

    source_display != rendered
}

fn is_declared_in_external_or_system_header(ty: &Type) -> bool {
    ty.get_declaration()
        .and_then(|decl| decl.get_location())
        .map(|location| {
            let (path, _, _) = location.get_presumed_location();
            is_external_or_system_path(&path)
        })
        .unwrap_or(false)
}

fn is_external_or_system_path(path: &str) -> bool {
    is_system_header_path(path)
        || path.contains("/external/")
        || path.contains("external/")
        || path.contains("_virtual_includes/")
        || (path.contains("bazel-out/") && path.contains("/external/"))
}

fn is_system_header_path(path: &str) -> bool {
    path.starts_with("/usr/include")
        || path.starts_with("/usr/local/include")
        || path.starts_with("/opt/")
        || path.contains("/gcc/")
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

fn normalize_pointer_reference_spacing(type_name: String) -> String {
    let chars: Vec<char> = type_name.chars().collect();
    let mut out = String::with_capacity(type_name.len() + 8);

    for (idx, ch) in chars.iter().copied().enumerate() {
        if is_pointer_or_reference(ch) && !is_function_pointer_marker(&chars, idx) {
            insert_space_before_pointer_marker(&mut out, &chars, idx);
        }
        out.push(ch);
    }

    out
}

fn is_pointer_or_reference(ch: char) -> bool {
    ch == '*' || ch == '&'
}

fn is_function_pointer_marker(chars: &[char], idx: usize) -> bool {
    matches!(chars.get(idx), Some('*' | '&'))
        && matches!(
            (
                idx.checked_sub(1).and_then(|i| chars.get(i)),
                chars.get(idx + 1)
            ),
            (Some('('), Some(')'))
        )
}

fn insert_space_before_pointer_marker(out: &mut String, chars: &[char], idx: usize) {
    let prev_input = idx.checked_sub(1).and_then(|i| chars.get(i)).copied();

    let is_first_pointer_in_sequence = prev_input != Some('*') && prev_input != Some('&');

    if is_first_pointer_in_sequence && !out.ends_with(' ') && !out.ends_with('(') {
        out.push(' ');
    }
}

pub fn resolve_type(original: &Type) -> ResolvedType {
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
    let kind = original.get_kind();

    // Single source of truth for builtin mapping; extend here when adding builtin support.
    if let Some(name) = builtin_name_from_type_kind(kind) {
        return ResolvedType::Builtin(name.to_string());
    }

    match kind {
        // ===== pointer =====
        TypeKind::Pointer => {
            if let Some(inner) = original.get_pointee_type() {
                let resolved_inner = resolve_type(&inner);
                if matches!(resolved_inner, ResolvedType::Function { .. }) {
                    ResolvedType::FunctionPointer(Box::new(resolved_inner))
                } else {
                    ResolvedType::Pointer(Box::new(resolved_inner))
                }
            } else {
                unknown(original)
            }
        }

        // ===== reference =====
        TypeKind::LValueReference => {
            if let Some(inner) = original.get_pointee_type() {
                let resolved_inner = resolve_type(&inner);
                if matches!(resolved_inner, ResolvedType::Function { .. }) {
                    ResolvedType::FunctionReference(Box::new(resolved_inner))
                } else {
                    ResolvedType::Reference(Box::new(resolved_inner))
                }
            } else {
                unknown(original)
            }
        }

        TypeKind::RValueReference => {
            if let Some(inner) = original.get_pointee_type() {
                let resolved_inner = resolve_type(&inner);
                ResolvedType::RValueReference(Box::new(resolved_inner))
            } else {
                unknown(original)
            }
        }

        // ===== function =====
        TypeKind::FunctionPrototype | TypeKind::FunctionNoPrototype => {
            resolve_function_type(original)
        }

        // ===== arrays =====
        TypeKind::ConstantArray => {
            let element = original
                .get_element_type()
                .map(|t| resolve_type(&t))
                .unwrap_or_else(|| unknown(original));

            ResolvedType::Array {
                element: Box::new(element),
                size: original.get_size(),
            }
        }

        // ===== user-defined / template =====
        // Named types (including aliases/templates) are resolved through decl-aware fallback.
        _ => resolve_named_type(original, canonical),
    }
}

/// Maps clang `TypeKind` builtin kinds to canonical display names used in this model.
fn builtin_name_from_type_kind(kind: TypeKind) -> Option<&'static str> {
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
        .map(|t| resolve_type(&t))
        .unwrap_or_else(|| unknown(original));

    let parameter_types = original
        .get_argument_types()
        .unwrap_or_default()
        .into_iter()
        .map(|arg| resolve_type(&arg))
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

    // Heuristic: unqualified source name but qualified canonical name likely indicates
    // alias/imported type; prefer canonical declaration path when possible.
    if !display_name.contains("::") && canonical_name.contains("::") {
        if let Some(resolved) = resolve_decl_based(canonical) {
            return resolved;
        }
    }

    // For typedef/type-alias, canonical declaration usually yields stable target id.
    if is_alias_type(original) {
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
    if let Some(resolved) = resolve_decl_based(original) {
        return resolved;
    }

    if let Some(resolved) = resolve_decl_based(canonical) {
        return resolved;
    }

    if is_dependent_expression_type(original) {
        let name = resolve_unknown_name(original, canonical);
        log::debug!(
            "type '{}' is structurally unresolvable before template instantiation \
             (dependent/decltype expression)",
            name
        );
        return ResolvedType::Dependent(name);
    }

    let name = resolve_unknown_name(original, canonical);
    log::debug!("could not resolve type '{}' to a concrete entity id", name);
    ResolvedType::Unknown(name)
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
        ty.get_declaration().map(|decl| decl.get_kind()),
        Some(EntityKind::TypedefDecl | EntityKind::TypeAliasDecl)
    )
}

fn resolve_decl_based(ty: &Type) -> Option<ResolvedType> {
    // Declaration-derived id is the primary identity source for user-defined types.
    // Template arguments are recursively resolved into the same semantic model.
    let decl = ty.get_declaration()?;
    let entity_id = build_entity_id_from_decl(&decl);

    let args: Vec<ResolvedType> = ty
        .get_template_argument_types()
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .map(|arg_ty| resolve_type(&arg_ty))
        .collect();

    if !args.is_empty() {
        return Some(ResolvedType::Template {
            base: entity_id,
            args,
        });
    }

    Some(ResolvedType::UserDefined(entity_id))
}

fn unknown(ty: &Type) -> ResolvedType {
    ResolvedType::Unknown(ty.get_display_name())
}

fn build_entity_id_from_decl(entity: &Entity) -> String {
    if entity.get_kind() == EntityKind::TemplateTemplateParameter {
        return entity.get_name().unwrap_or_default();
    }

    strip_global_scope_prefix(&build_fqn_from_entity(entity))
}

fn strip_global_scope_prefix(type_name: &str) -> String {
    type_name.trim_start_matches("::").to_string()
}

pub(crate) fn collapse_std_internal_namespaces(parts: Vec<(String, bool)>) -> Vec<String> {
    let mut collapsed: Vec<String> = Vec::with_capacity(parts.len());

    for (name, is_namespace) in parts {
        let prev = collapsed.last().map(|s| s.as_str());
        if should_skip_std_internal_namespace(is_namespace, &name, prev) {
            continue;
        }
        collapsed.push(name);
    }

    collapsed
}

fn should_skip_std_internal_namespace(
    is_namespace: bool,
    name: &str,
    previous_segment: Option<&str>,
) -> bool {
    is_namespace && previous_segment == Some("std") && is_std_internal_namespace_segment(name)
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
    let mut parts: Vec<(String, bool)> = Vec::new();
    let mut current = Some(*entity);

    while let Some(entity) = current {
        match entity.get_kind() {
            EntityKind::Namespace => {
                if let Some(name) = entity.get_name() {
                    parts.push((name, true));
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
            // Stop at TranslationUnit, unexposed decls, or anything else
            _ => break,
        }
        current = entity.get_semantic_parent();
    }

    parts.reverse();
    collapse_std_internal_namespaces(parts).join("::")
}
