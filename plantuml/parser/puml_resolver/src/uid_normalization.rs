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

//! Id construction and reference lookup (spec `element-identifiers.md`, Rule C).

use puml_utils::normalize_identity_label;

pub use uid_utils::{is_identifier_path, join, normalize, normalized_segments};

/// Kind of element a name belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityKind {
    /// class, abstract class, interface, enum, struct: a trailing template
    /// argument list is not part of the name.
    ClassLike,
    Other,
}

/// Why a written name is not an identifier path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityError {
    FreeText,
    MalformedPath,
}

impl IdentityError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::FreeText => {
                "name is free text, expected an identifier path (segments of letters, \
                 digits and `_` separated by `.` or `::`)"
            }
            Self::MalformedPath => {
                "name is not a valid identifier path (non-empty segments of letters, \
                 digits and `_` separated by `.` or `::`)"
            }
        }
    }
}

/// Identity text of a written name (spec `element-identifiers.md`, Rule A').
///
/// The first non-empty line after markup removal, without a trailing template
/// argument list for class-like kinds. It must be an identifier path; a
/// leading root marker is kept.
pub fn identity_name(label: &str, kind: IdentityKind) -> Result<String, IdentityError> {
    let normalized = normalize_identity_label(label);
    let first_line = normalized
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    let text = match kind {
        IdentityKind::ClassLike => strip_template_arguments(first_line),
        IdentityKind::Other => first_line,
    };

    if is_valid_path(text) {
        Ok(text.to_string())
    } else if is_path_like(text) {
        Err(IdentityError::MalformedPath)
    } else {
        Err(IdentityError::FreeText)
    }
}

/// `text` without one trailing balanced `<...>` group.
fn strip_template_arguments(text: &str) -> &str {
    let Some(inner) = text.strip_suffix('>') else {
        return text;
    };

    let mut depth = 1usize;
    for (index, ch) in inner.char_indices().rev() {
        match ch {
            '>' => depth += 1,
            '<' => {
                depth -= 1;
                if depth == 0 {
                    return inner[..index].trim_end();
                }
            }
            _ => {}
        }
    }

    text
}

/// Segments of letters, digits and `_`, separated by `.` or `::`, with an
/// optional leading root marker.
fn is_valid_path(text: &str) -> bool {
    strip_root_marker(text)
        .replace("::", ".")
        .split('.')
        .all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

/// A single token with a path separator, so the author meant a path.
fn is_path_like(text: &str) -> bool {
    is_identifier_path(text) && !text.chars().any(char::is_whitespace)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InternalScope(Vec<String>);

impl InternalScope {
    pub fn new<I, S>(parts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self(
            parts
                .into_iter()
                .flat_map(|part| normalized_segments(part.as_ref()))
                .collect(),
        )
    }

    pub fn from_path(value: &str) -> Self {
        Self(normalized_segments(value))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Nested scope under `leaf` (one or more segments).
    pub fn child(&self, leaf: &str) -> Self {
        let mut parts = self.0.clone();
        parts.extend(normalized_segments(leaf));
        Self(parts)
    }

    /// Scope of a declaration named `name`: a leading `.` or `::` is rooted,
    /// anything else nests under this scope.
    pub fn declare(&self, name: &str) -> Self {
        if has_root_marker(name) {
            Self::from_path(name)
        } else {
            self.child(name)
        }
    }

    /// All segments but the last.
    pub fn parent(&self) -> Self {
        let mut parts = self.0.clone();
        parts.pop();
        Self(parts)
    }

    /// Id of this scope.
    pub fn id(&self) -> String {
        join(self.0.iter().map(String::as_str))
    }

    /// Id of `leaf` below this scope; `leaf` must be one segment (no `.` or `::`).
    pub fn id_with_leaf(&self, leaf: &str) -> String {
        debug_assert!(!is_identifier_path(leaf));
        join(self.0.iter().cloned().chain([normalize(leaf)]))
    }
}

/// Where a declaration sits: its id path (what it is) and its reference path
/// (what Rule C resolves against). They differ below an alias: the id path
/// uses the name, the reference path the alias.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DeclarationScope {
    pub id: InternalScope,
    pub reference: InternalScope,
}

impl DeclarationScope {
    /// Scope whose id and reference paths are both `path`.
    pub fn from_path(path: &str) -> Self {
        Self {
            id: InternalScope::from_path(path),
            reference: InternalScope::from_path(path),
        }
    }

    /// Scope of a declaration `name` with an optional `alias`: the id path
    /// follows the name, the reference path the alias (else the name).
    pub fn declare(&self, name: &str, alias: Option<&str>) -> Self {
        Self {
            id: self.id.declare(name),
            reference: self.reference.declare(alias.unwrap_or(name)),
        }
    }
}

/// A leading `.` or `::` roots a name.
fn has_root_marker(name: &str) -> bool {
    name.starts_with('.') || name.starts_with("::")
}

/// `name` without its leading root marker (`.` or `::`).
pub fn strip_root_marker(name: &str) -> &str {
    name.strip_prefix("::")
        .or_else(|| name.strip_prefix('.'))
        .unwrap_or(name)
}

/// Key for the leaf lookup in [`resolve_reference`]: the last id segment of `leaf`.
pub fn leaf_key(leaf: &str) -> String {
    normalized_segments(leaf)
        .pop()
        .unwrap_or_else(|| leaf.to_string())
}

/// Outcome of [`resolve_reference`].
#[derive(Debug, Eq, PartialEq)]
pub enum Resolution {
    Resolved(String),
    Unresolved,
    Ambiguous(Vec<String>),
}

/// Rule C reference lookup. `local_exists` checks the literal `S.r`
/// candidate; `rooted_exists` and `leaf_entries` apply the caller's
/// visibility filter (e.g. declaration order).
pub fn resolve_reference<A, B, L>(
    scope: &InternalScope,
    raw: &str,
    local_exists: A,
    rooted_exists: B,
    leaf_entries: L,
) -> Resolution
where
    A: Fn(&str) -> bool,
    B: Fn(&str) -> bool,
    L: FnOnce(&str) -> Vec<String>,
{
    if has_root_marker(raw) {
        let rooted = InternalScope::from_path(raw).id();
        return if rooted_exists(&rooted) {
            Resolution::Resolved(rooted)
        } else {
            Resolution::Unresolved
        };
    }

    if !is_identifier_path(raw) {
        let local = scope.id_with_leaf(raw);
        if local_exists(&local) {
            return Resolution::Resolved(local);
        }

        let mut candidates = leaf_entries(raw);
        candidates.sort();
        candidates.dedup();

        return match candidates.len() {
            0 => Resolution::Unresolved,
            1 => Resolution::Resolved(candidates.remove(0)),
            _ => Resolution::Ambiguous(candidates),
        };
    }

    let local = scope.child(raw).id();
    let rooted = InternalScope::from_path(raw).id();
    let local_hit = local_exists(&local).then_some(local);
    let rooted_hit = rooted_exists(&rooted).then_some(rooted);

    match (local_hit, rooted_hit) {
        (Some(a), Some(b)) if a != b => {
            let mut candidates = vec![a, b];
            candidates.sort();
            Resolution::Ambiguous(candidates)
        }
        (Some(a), _) => Resolution::Resolved(a),
        (None, Some(b)) => Resolution::Resolved(b),
        (None, None) => Resolution::Unresolved,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        identity_name, leaf_key, resolve_reference, strip_root_marker, DeclarationScope,
        IdentityError, IdentityKind, InternalScope, Resolution,
    };

    fn class_like(label: &str) -> Result<String, IdentityError> {
        identity_name(label, IdentityKind::ClassLike)
    }

    fn other(label: &str) -> Result<String, IdentityError> {
        identity_name(label, IdentityKind::Other)
    }

    #[test]
    fn identity_name_accepts_identifier_paths() {
        assert_eq!(other("Client").as_deref(), Ok("Client"));
        assert_eq!(other("a.b.C").as_deref(), Ok("a.b.C"));
        assert_eq!(other("a::b::C").as_deref(), Ok("a::b::C"));
        assert_eq!(other("a::b.C").as_deref(), Ok("a::b.C"));
        assert_eq!(other("snake_case_1").as_deref(), Ok("snake_case_1"));
    }

    #[test]
    fn identity_name_keeps_the_root_marker() {
        assert_eq!(other("::a::X").as_deref(), Ok("::a::X"));
        assert_eq!(other(".a.X").as_deref(), Ok(".a.X"));
        assert_eq!(other("::X").as_deref(), Ok("::X"));
    }

    #[test]
    fn identity_name_strips_markup() {
        assert_eq!(other("<b>comp::unit</b>").as_deref(), Ok("comp::unit"));
        assert_eq!(other("<color:red>Client</color>").as_deref(), Ok("Client"));
    }

    #[test]
    fn identity_name_uses_the_first_line() {
        assert_eq!(other("comp::b\\nextra").as_deref(), Ok("comp::b"));
        assert_eq!(other("\\ncomp::b").as_deref(), Ok("comp::b"));
        assert_eq!(
            class_like("DummySkeleton\\n{{generated}}").as_deref(),
            Ok("DummySkeleton")
        );
    }

    #[test]
    fn identity_name_slash_n_is_text() {
        assert_eq!(other("comp::a/nextra"), Err(IdentityError::MalformedPath));
        assert_eq!(other("comp a/nextra"), Err(IdentityError::FreeText));
    }

    #[test]
    fn identity_name_class_like_drops_a_trailing_template_list() {
        assert_eq!(
            class_like("ProxyContainer<ProxySpec...>").as_deref(),
            Ok("ProxyContainer")
        );
        assert_eq!(class_like("a::B<C<D>>").as_deref(), Ok("a::B"));
        assert_eq!(class_like("Foo <T>").as_deref(), Ok("Foo"));
    }

    #[test]
    fn identity_name_other_kinds_keep_template_text() {
        assert_eq!(other("Foo<T>"), Err(IdentityError::FreeText));
    }

    #[test]
    fn identity_name_class_like_rejects_unbalanced_templates() {
        assert_eq!(class_like("Foo<T"), Err(IdentityError::FreeText));
        assert_eq!(class_like("Foo>"), Err(IdentityError::FreeText));
        assert_eq!(class_like("<T>"), Err(IdentityError::FreeText));
    }

    #[test]
    fn identity_name_prose_is_free_text() {
        assert_eq!(other("Unit 1"), Err(IdentityError::FreeText));
        assert_eq!(
            class_like("Generated <Name>Proxy"),
            Err(IdentityError::FreeText)
        );
        assert_eq!(
            other("backend : logging::Recorder::Backend"),
            Err(IdentityError::FreeText)
        );
        assert_eq!(
            other(":logging::IBackend"),
            Err(IdentityError::MalformedPath)
        );
    }

    #[test]
    fn identity_name_dash_and_at_are_free_text() {
        assert_eq!(other("my-service"), Err(IdentityError::FreeText));
        assert_eq!(other("user@host"), Err(IdentityError::FreeText));
    }

    #[test]
    fn identity_name_empty_is_free_text() {
        assert_eq!(other(""), Err(IdentityError::FreeText));
        assert_eq!(other("   "), Err(IdentityError::FreeText));
        assert_eq!(other("<b></b>"), Err(IdentityError::FreeText));
    }

    #[test]
    fn identity_name_malformed_paths_are_reported() {
        for label in ["a.", "a..b", "a::", "a:::b", "my-pkg::unit", "::"] {
            assert_eq!(other(label), Err(IdentityError::MalformedPath), "{label}");
        }
    }

    #[test]
    fn strip_root_marker_removes_one_leading_marker() {
        assert_eq!(strip_root_marker("::a::X"), "a::X");
        assert_eq!(strip_root_marker(".a.X"), "a.X");
        assert_eq!(strip_root_marker("::X"), "X");
    }

    #[test]
    fn strip_root_marker_keeps_relative_names() {
        assert_eq!(strip_root_marker("a::X"), "a::X");
        assert_eq!(strip_root_marker("a.X"), "a.X");
        assert_eq!(strip_root_marker("X"), "X");
    }

    #[test]
    fn leaf_key_is_the_last_segment() {
        assert_eq!(leaf_key("Recorder"), "Recorder");
        assert_eq!(leaf_key("core::geometry.Recorder"), "Recorder");
    }

    #[test]
    fn leaf_key_falls_back_to_the_input_without_segments() {
        assert_eq!(leaf_key("::"), "::");
    }

    #[test]
    fn internal_scope_normalizes_each_segment() {
        let scope = InternalScope::new(["score::logging", "", ".core."]);

        assert_eq!(scope, InternalScope::from_path("score.logging.core"));
    }

    #[test]
    fn internal_scope_id_with_leaf_appends_one_segment() {
        let scope = InternalScope::new(["component", "subsystem"]);

        assert_eq!(scope.id(), "component.subsystem");
        assert_eq!(
            scope.id_with_leaf("Recorder"),
            "component.subsystem.Recorder"
        );
    }

    #[test]
    fn internal_scope_child_appends_normalized_segments() {
        let scope = InternalScope::from_path("core").child("geometry::shapes");

        assert_eq!(scope, InternalScope::from_path("core.geometry.shapes"));
    }

    #[test]
    fn declare_plain_name_nests_under_the_scope() {
        let scope = InternalScope::from_path("outer");

        assert_eq!(
            scope.declare("Circle"),
            InternalScope::from_path("outer.Circle")
        );
    }

    #[test]
    fn declare_qualified_name_nests_under_the_scope() {
        let scope = InternalScope::from_path("outer");

        assert_eq!(
            scope.declare("core::geometry::Circle"),
            InternalScope::from_path("outer.core.geometry.Circle")
        );
        assert_eq!(
            scope.declare("core.geometry"),
            InternalScope::from_path("outer.core.geometry")
        );
    }

    #[test]
    fn declare_leading_root_marker_replaces_the_scope() {
        let scope = InternalScope::from_path("outer");

        assert_eq!(scope.declare(".X"), InternalScope::from_path("X"));
        assert_eq!(scope.declare("::X"), InternalScope::from_path("X"));
        assert_eq!(scope.declare("::a::X"), InternalScope::from_path("a.X"));
    }

    #[test]
    fn parent_drops_the_last_segment() {
        assert_eq!(
            InternalScope::from_path("a.b.C").parent(),
            InternalScope::from_path("a.b")
        );
        assert!(InternalScope::from_path("C").parent().is_empty());
        assert!(InternalScope::default().parent().is_empty());
    }

    #[test]
    fn declaration_scope_without_alias_uses_the_name_for_both_paths() {
        let scope = DeclarationScope::from_path("outer").declare("core::Circle", None);

        assert_eq!(scope, DeclarationScope::from_path("outer.core.Circle"));
    }

    #[test]
    fn declaration_scope_alias_only_changes_the_reference_path() {
        let scope = DeclarationScope::from_path("outer").declare("score::x::Y", Some("Y"));

        assert_eq!(scope.id, InternalScope::from_path("outer.score.x.Y"));
        assert_eq!(scope.reference, InternalScope::from_path("outer.Y"));
    }

    #[test]
    fn declaration_scope_nests_both_paths() {
        let outer = DeclarationScope::from_path("").declare("pkg::Outer", Some("O"));
        let inner = outer.declare("Inner", None);

        assert_eq!(inner.id, InternalScope::from_path("pkg.Outer.Inner"));
        assert_eq!(inner.reference, InternalScope::from_path("O.Inner"));
    }

    #[test]
    fn internal_scope_is_empty_reflects_segment_count() {
        assert!(InternalScope::default().is_empty());
        assert!(!InternalScope::from_path("core").is_empty());
    }

    #[test]
    fn resolve_reference_simple_name_prefers_local_scope() {
        let scope = InternalScope::from_path("a");

        let resolution = resolve_reference(
            &scope,
            "X",
            |id| id == "a.X" || id == "X",
            |id| id == "a.X" || id == "X",
            |_| vec!["X".to_string()],
        );

        assert_eq!(resolution, Resolution::Resolved("a.X".to_string()));
    }

    #[test]
    fn resolve_reference_simple_name_falls_back_to_unique_leaf() {
        let scope = InternalScope::from_path("a");

        let resolution = resolve_reference(
            &scope,
            "X",
            |id| id == "p.X",
            |id| id == "p.X",
            |_| vec!["p.X".to_string()],
        );

        assert_eq!(resolution, Resolution::Resolved("p.X".to_string()));
    }

    #[test]
    fn resolve_reference_simple_name_several_leaves_is_ambiguous() {
        let scope = InternalScope::default();

        let resolution = resolve_reference(
            &scope,
            "X",
            |_| false,
            |_| false,
            |_| vec!["p.X".to_string(), "q.X".to_string()],
        );

        assert_eq!(
            resolution,
            Resolution::Ambiguous(vec!["p.X".to_string(), "q.X".to_string()])
        );
    }

    #[test]
    fn resolve_reference_qualified_name_ambiguous_between_local_and_rooted() {
        let scope = InternalScope::from_path("core");

        let resolution = resolve_reference(
            &scope,
            "geometry::User",
            |id| id == "core.geometry.User" || id == "geometry.User",
            |id| id == "core.geometry.User" || id == "geometry.User",
            |_| vec![],
        );

        assert_eq!(
            resolution,
            Resolution::Ambiguous(vec![
                "core.geometry.User".to_string(),
                "geometry.User".to_string()
            ])
        );
    }

    #[test]
    fn resolve_reference_root_marker_bypasses_local_scope() {
        let scope = InternalScope::from_path("a");

        let resolution = resolve_reference(
            &scope,
            ".X",
            |id| id == "X",
            |id| id == "X",
            |_| vec!["a.X".to_string()],
        );

        assert_eq!(resolution, Resolution::Resolved("X".to_string()));
    }

    #[test]
    fn resolve_reference_root_marker_unresolved_when_missing() {
        let scope = InternalScope::from_path("a");

        let resolution = resolve_reference(&scope, ".X", |_| false, |_| false, |_| vec![]);

        assert_eq!(resolution, Resolution::Unresolved);
    }

    #[test]
    fn resolve_reference_double_colon_root_marker_bypasses_local_scope() {
        let scope = InternalScope::from_path("a");

        let resolution = resolve_reference(
            &scope,
            "::X",
            |id| id == "X",
            |id| id == "X",
            |_| vec!["a.X".to_string()],
        );

        assert_eq!(resolution, Resolution::Resolved("X".to_string()));
    }

    #[test]
    fn resolve_reference_double_colon_root_marker_unresolved_when_missing() {
        let scope = InternalScope::from_path("a");

        let resolution = resolve_reference(&scope, "::X", |_| false, |_| false, |_| vec![]);

        assert_eq!(resolution, Resolution::Unresolved);
    }
}
