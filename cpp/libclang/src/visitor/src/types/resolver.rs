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

use crate::clang_adapter::exception_specification::has_plain_noexcept;
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

/// Resolves the type of a declaration (field or function), guarding against
/// clang's own error-recovery behavior.
///
/// When `entity` has an invalid declaration -- typically because a
/// `#include` for the type failed to resolve -- `entity.get_type()` may no
/// longer reflect what was written in the source: clang silently substitutes
/// a placeholder type (commonly `int`) so it can keep parsing. Reporting
/// that placeholder as the real type would produce a misleading "type
/// differs between design and implementation" finding instead of surfacing
/// the real problem (the failed `#include`). In that case, this recovers the
/// type as written by re-tokenizing the declaration's own source range
/// instead. If the recovered spelling matches what clang already reports
/// (i.e. this particular declaration parsed fine despite a sibling error),
/// clang's own resolution is kept so builtins still resolve as builtins.
pub(crate) fn resolve_declared_type(entity: &Entity, original: &Type) -> ResolvedType {
    match recover_declared_spelling(entity, original, entity.is_invalid_declaration()) {
        Some(spelled) => ResolvedType::Unknown(spelled),
        None => resolve_type(original),
    }
}

/// Same guard as [`resolve_declared_type`], for a callable's parameter.
///
/// clang's "invalid declaration" bit is not reliably set on an unnamed
/// `ParmDecl` even when the callable it belongs to is invalid (and
/// `Entity::get_semantic_parent()` is not reliably set on parameters either),
/// so the callable's own bit is checked explicitly as well.
pub(crate) fn resolve_declared_argument_type(
    argument: &Entity,
    callable: &Entity,
    original: &Type,
) -> ResolvedType {
    let invalid = argument.is_invalid_declaration() || callable.is_invalid_declaration();
    match recover_declared_spelling(argument, original, invalid) {
        Some(spelled) => ResolvedType::Unknown(spelled),
        None => resolve_type(original),
    }
}

/// Same guard as [`resolve_declared_argument_type`], but for call sites that
/// render a parameter's type via `Type::get_display_name()` directly.
pub(crate) fn declared_argument_display_name(
    argument: &Entity,
    callable: &Entity,
    original: &Type,
) -> String {
    let invalid = argument.is_invalid_declaration() || callable.is_invalid_declaration();
    recover_declared_spelling(argument, original, invalid)
        .unwrap_or_else(|| original.get_display_name())
}

/// Returns the type as written in the source, but only when `invalid` and the
/// recovered spelling differs from what clang itself reports -- i.e. only
/// when clang's reported type is actually the error-recovery placeholder,
/// not just a sibling declaration's unrelated invalidity.
fn recover_declared_spelling(entity: &Entity, original: &Type, invalid: bool) -> Option<String> {
    if !invalid {
        return None;
    }
    let spelled = spelled_type_from_source(entity)?;
    if spelled == original.get_display_name() {
        return None;
    }
    log::debug!(
        "'{}' has an invalid declaration (likely caused by an unresolved #include); \
         using the type as written ('{}') instead of clang's error-recovery type",
        entity.get_name().unwrap_or_default(),
        spelled,
    );
    Some(spelled)
}

/// Recovers a declaration's type as written in the source, by re-tokenizing
/// its own source range and taking everything before the declarator name (or
/// before a top-level `=` default value, for an unnamed declaration). Returns
/// `None` if the entity has no source range or no leading tokens (e.g. the
/// type itself could not be tokenized, such as for a macro-expanded
/// declaration).
///
/// Known limitation: for a parameter that is both unnamed *and* of a fully
/// unresolved type (e.g. `void f(Missing);` where `Missing` is never
/// declared), libclang reports no source range at all for the `ParmDecl`, so
/// recovery is not possible and clang's placeholder type is reported as-is.
/// This combination is rare in practice (unresolved custom types are
/// normally still named for readability); named parameters and unnamed
/// parameters of an otherwise-resolvable type are unaffected.
fn spelled_type_from_source(entity: &Entity) -> Option<String> {
    const SKIP_KEYWORDS: &[&str] = &[
        "static",
        "mutable",
        "inline",
        "constexpr",
        "virtual",
        "explicit",
        "extern",
        "friend",
        "register",
        "thread_local",
    ];

    // Position of the declarator name itself, so tokens making up e.g. an
    // array size, initializer, or nested-name-specifier that *follows* the
    // name are never mistaken for a leading one. Absent for unnamed
    // declarations (e.g. an unnamed parameter).
    let name_offset = entity
        .get_name()
        .filter(|name| !name.is_empty())
        .and_then(|_| entity.get_location())
        .map(|location| location.get_spelling_location().offset);

    let tokens = entity.get_range()?.tokenize();

    let mut type_tokens = Vec::new();
    let mut angle_depth: i32 = 0;
    for token in &tokens {
        let spelling = token.get_spelling();

        match name_offset {
            Some(name_offset) => {
                if token.get_location().get_spelling_location().offset >= name_offset {
                    break;
                }
            }
            // No declarator name to bound the scan (unnamed declaration): stop
            // at a top-level default value instead (e.g. `void f(int = 5)`).
            None if spelling == "=" && angle_depth == 0 => break,
            None => {}
        }

        if spelling == "<" {
            angle_depth += 1;
        } else if is_closing_angles(&spelling) {
            angle_depth -= spelling.len() as i32;
        }

        if !SKIP_KEYWORDS.contains(&spelling.as_str()) {
            type_tokens.push(spelling);
        }
    }

    if type_tokens.is_empty() {
        return None;
    }

    Some(render_type_tokens(&strip_trailing_qualifier(type_tokens)))
}

/// Removes a trailing nested-name-specifier (e.g. `Ns::Class::` or a
/// templated `Class<T>::`) from the end of a declaration's leading tokens.
/// This is the qualifier of an out-of-line definition's declarator name
/// (`ReturnType Class::method()`), not part of the return type itself.
fn strip_trailing_qualifier(mut tokens: Vec<String>) -> Vec<String> {
    while tokens.last().map(String::as_str) == Some("::") {
        tokens.pop();
        match tokens.pop() {
            Some(closing) if is_closing_angles(&closing) => {
                let mut depth = closing.len() as i32;
                while depth > 0 {
                    match tokens.pop() {
                        Some(inner) if inner == "<" => depth -= 1,
                        Some(inner) if is_closing_angles(&inner) => depth += inner.len() as i32,
                        Some(_) => {}
                        None => break,
                    }
                }
                tokens.pop(); // the template name preceding '<'
            }
            _ => {} // a plain identifier qualifier, already popped
        }
    }
    tokens
}

/// A token consisting solely of `>` characters (clang's raw lexer emits `>>`
/// and `>>>` as single tokens; it does not perform the C++11 "maximal munch"
/// split into separate `>` tokens that a template-aware parser would).
fn is_closing_angles(token: &str) -> bool {
    !token.is_empty() && token.chars().all(|c| c == '>')
}

/// Joins spelled-out declaration tokens back into a single type string,
/// keeping template/scope punctuation tight (`std::vector<int>`) while still
/// spacing out keywords and declarators (`const T &`).
fn render_type_tokens(tokens: &[String]) -> String {
    fn no_space_before(token: &str) -> bool {
        token == "::" || token == "," || token == "<" || is_closing_angles(token)
    }
    fn no_space_after(token: &str) -> bool {
        token == "::" || token == "<"
    }

    let mut rendered = String::new();
    for (index, token) in tokens.iter().enumerate() {
        let previous = index.checked_sub(1).map(|i| tokens[i].as_str());
        let needs_space = !rendered.is_empty()
            && !no_space_before(token)
            && !previous.is_some_and(no_space_after);
        if needs_space {
            rendered.push(' ');
        }
        rendered.push_str(token);
    }
    rendered
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
        TypeKind::IncompleteArray => ResolvedType::Array {
            element: Box::new(
                original
                    .get_element_type()
                    .map(|element| resolve_type(&element))
                    .unwrap_or_else(|| unknown(original)),
            ),
            size: None,
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
        TypeKind::CharS => Some("char"),
        TypeKind::SChar => Some("signed char"),
        TypeKind::UChar => Some("unsigned char"),
        TypeKind::Short => Some("short"),
        TypeKind::UShort => Some("unsigned short"),
        TypeKind::Int => Some("int"),
        TypeKind::UInt => Some("unsigned int"),
        TypeKind::Long => Some("long"),
        TypeKind::ULong => Some("unsigned long"),
        TypeKind::LongLong => Some("long long"),
        TypeKind::ULongLong => Some("unsigned long long"),
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
        is_noexcept: has_plain_noexcept(original.get_exception_specification()),
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
    use super::{collapse_std_internal_namespaces, render_type_tokens, strip_trailing_qualifier};

    fn tokens(spellings: &[&str]) -> Vec<String> {
        spellings.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn renders_simple_name() {
        assert_eq!(render_type_tokens(&tokens(&["Missing"])), "Missing");
    }

    #[test]
    fn renders_reference_with_leading_space() {
        assert_eq!(
            render_type_tokens(&tokens(&["const", "Missing", "&"])),
            "const Missing &"
        );
    }

    #[test]
    fn renders_nested_template_without_stray_spaces() {
        let nested = tokens(&[
            "std", "::", "map", "<", "int", ",", "std", "::", "vector", "<", "Missing", ">>",
        ]);
        assert_eq!(
            render_type_tokens(&nested),
            "std::map<int, std::vector<Missing>>"
        );
    }

    #[test]
    fn renders_single_level_template() {
        let single = tokens(&["std", "::", "vector", "<", "Missing", ">"]);
        assert_eq!(render_type_tokens(&single), "std::vector<Missing>");
    }

    #[test]
    fn strips_simple_out_of_line_qualifier() {
        let with_qualifier = tokens(&["Missing", "S", "::"]);
        assert_eq!(
            strip_trailing_qualifier(with_qualifier),
            tokens(&["Missing"])
        );
    }

    #[test]
    fn strips_namespaced_out_of_line_qualifier() {
        let with_qualifier = tokens(&["Missing", "ns", "::", "S", "::"]);
        assert_eq!(
            strip_trailing_qualifier(with_qualifier),
            tokens(&["Missing"])
        );
    }

    #[test]
    fn strips_templated_out_of_line_qualifier() {
        let with_qualifier = tokens(&["Missing", "Foo", "<", "T", ">", "::"]);
        assert_eq!(
            strip_trailing_qualifier(with_qualifier),
            tokens(&["Missing"])
        );
    }

    #[test]
    fn leaves_trailing_template_close_untouched() {
        // A type's own trailing '>' (not a qualifier) must survive unstripped.
        let plain_template = tokens(&["std", "::", "vector", "<", "Missing", ">"]);
        assert_eq!(
            strip_trailing_qualifier(plain_template.clone()),
            plain_template
        );
    }

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
