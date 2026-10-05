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

pub use uid_utils::{is_identifier_path, join, normalize, normalized_segments};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RootAnchor(Option<String>);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InternalScope(Vec<String>);

impl RootAnchor {
    pub fn new(value: Option<&str>) -> Self {
        Self(
            value
                .map(normalize)
                .filter(|normalized| !normalized.is_empty()),
        )
    }

    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

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

    pub fn resolve_with_leaf(&self, root_anchor: &RootAnchor, leaf: &str) -> String {
        let mut parts = Vec::with_capacity(self.0.len() + 2);

        if let Some(root_anchor) = root_anchor.as_deref() {
            parts.push(root_anchor.to_string());
        }

        parts.extend(self.0.iter().cloned());
        parts.push(normalize(leaf));

        join(parts)
    }

    pub fn resolve(&self, root_anchor: &RootAnchor) -> String {
        join(
            root_anchor
                .as_deref()
                .into_iter()
                .chain(self.0.iter().map(String::as_str)),
        )
    }
}

/// A leading `.` or `::` anchors a name at the root anchor.
fn has_root_marker(name: &str) -> bool {
    name.starts_with('.') || name.starts_with("::")
}

/// `name` without its leading root marker (`.` or `::`).
pub fn strip_root_marker(name: &str) -> &str {
    name.strip_prefix("::")
        .or_else(|| name.strip_prefix('.'))
        .unwrap_or(name)
}

/// `path` below `root_anchor`; the anchor is never stripped from `path`.
pub fn resolve_explicit_path(root_anchor: &RootAnchor, path: &str) -> String {
    InternalScope::from_path(path).resolve(root_anchor)
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
    root_anchor: &RootAnchor,
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
        let rooted = resolve_explicit_path(root_anchor, raw);
        return if rooted_exists(&rooted) {
            Resolution::Resolved(rooted)
        } else {
            Resolution::Unresolved
        };
    }

    if !is_identifier_path(raw) {
        let local = scope.resolve_with_leaf(root_anchor, raw);
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

    let local = scope.child(raw).resolve(root_anchor);
    let rooted = resolve_explicit_path(root_anchor, raw);
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
        leaf_key, resolve_explicit_path, resolve_reference, strip_root_marker, InternalScope,
        Resolution, RootAnchor,
    };

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
    fn root_anchor_normalizes_and_retains_non_empty_values() {
        assert_eq!(
            RootAnchor::new(Some("score::logging")).as_deref(),
            Some("score.logging")
        );
    }

    #[test]
    fn root_anchor_drops_empty_values_after_normalization() {
        assert_eq!(RootAnchor::new(Some("::")).as_deref(), None);
        assert_eq!(RootAnchor::new(None).as_deref(), None);
    }

    #[test]
    fn internal_scope_normalizes_each_segment() {
        let scope = InternalScope::new(["score::logging", "", ".core."]);

        assert_eq!(scope, InternalScope::from_path("score.logging.core"));
    }

    #[test]
    fn internal_scope_resolves_with_leaf_and_root_anchor() {
        let root_anchor = RootAnchor::new(Some("score::logging"));
        let scope = InternalScope::new(["component", "subsystem"]);

        assert_eq!(
            scope.resolve_with_leaf(&root_anchor, "Recorder"),
            "score.logging.component.subsystem.Recorder"
        );
    }

    #[test]
    fn resolve_explicit_path_prefixes_root_anchor() {
        let root_anchor = RootAnchor::new(Some("score::logging"));

        assert_eq!(
            resolve_explicit_path(&root_anchor, "core::Recorder"),
            "score.logging.core.Recorder"
        );
    }

    #[test]
    fn resolve_explicit_path_never_strips_a_pre_existing_anchor_prefix() {
        let root_anchor = RootAnchor::new(Some("score::logging"));

        assert_eq!(
            resolve_explicit_path(&root_anchor, "score::logging::Recorder"),
            "score.logging.score.logging.Recorder"
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
    fn internal_scope_is_empty_reflects_segment_count() {
        assert!(InternalScope::default().is_empty());
        assert!(!InternalScope::from_path("core").is_empty());
    }

    #[test]
    fn resolve_reference_simple_name_prefers_local_scope() {
        let scope = InternalScope::from_path("a");
        let root_anchor = RootAnchor::default();

        let resolution = resolve_reference(
            &scope,
            &root_anchor,
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
        let root_anchor = RootAnchor::default();

        let resolution = resolve_reference(
            &scope,
            &root_anchor,
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
        let root_anchor = RootAnchor::default();

        let resolution = resolve_reference(
            &scope,
            &root_anchor,
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
        let root_anchor = RootAnchor::default();

        let resolution = resolve_reference(
            &scope,
            &root_anchor,
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
        let root_anchor = RootAnchor::default();

        let resolution = resolve_reference(
            &scope,
            &root_anchor,
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
        let root_anchor = RootAnchor::default();

        let resolution =
            resolve_reference(&scope, &root_anchor, ".X", |_| false, |_| false, |_| vec![]);

        assert_eq!(resolution, Resolution::Unresolved);
    }

    #[test]
    fn resolve_reference_double_colon_root_marker_bypasses_local_scope() {
        let scope = InternalScope::from_path("a");
        let root_anchor = RootAnchor::default();

        let resolution = resolve_reference(
            &scope,
            &root_anchor,
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
        let root_anchor = RootAnchor::default();

        let resolution = resolve_reference(
            &scope,
            &root_anchor,
            "::X",
            |_| false,
            |_| false,
            |_| vec![],
        );

        assert_eq!(resolution, Resolution::Unresolved);
    }
}
