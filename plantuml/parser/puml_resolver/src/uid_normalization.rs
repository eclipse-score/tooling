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

pub use uid_utils::{join, normalize, normalized_segments};

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

    pub fn from_optional_path(value: Option<&str>) -> Self {
        value.map(Self::from_path).unwrap_or_default()
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

pub fn resolve_explicit_path(root_anchor: &RootAnchor, path: &str) -> String {
    let normalized_path = normalize(path);

    internal_scope_from_resolved_path(root_anchor, Some(normalized_path.as_str()))
        .resolve(root_anchor)
}

pub fn internal_scope_from_resolved_path(
    root_anchor: &RootAnchor,
    path: Option<&str>,
) -> InternalScope {
    match path {
        Some(path) => {
            let unrooted_path = root_anchor
                .as_deref()
                .and_then(|root| strip_root_anchor_prefix(path, root))
                .map(|rest| rest.trim_start_matches('.'))
                .unwrap_or(path);

            InternalScope::from_path(unrooted_path)
        }
        None => InternalScope::default(),
    }
}

fn strip_root_anchor_prefix<'a>(path: &'a str, root: &str) -> Option<&'a str> {
    if path == root {
        return Some("");
    }

    path.strip_prefix(root)?.strip_prefix('.')
}

pub fn is_explicit_path(value: &str) -> bool {
    value.contains('.') || value.contains("::")
}

#[cfg(test)]
mod tests {
    use super::{
        internal_scope_from_resolved_path, is_explicit_path, join, normalize, normalized_segments,
        resolve_explicit_path, InternalScope, RootAnchor,
    };

    #[test]
    fn normalize_treats_cpp_and_dot_separators_equally() {
        assert_eq!(
            normalize("score::mw.log::Recorder"),
            "score.mw.log.Recorder"
        );
    }

    #[test]
    fn normalize_drops_leading_and_trailing_dots() {
        assert_eq!(normalize("..score::mw::log.."), "score.mw.log");
    }

    #[test]
    fn normalized_segments_drops_empty_segments_after_normalization() {
        assert_eq!(
            normalized_segments("..score::mw.log::Recorder.."),
            vec!["score", "mw", "log", "Recorder"]
        );
    }

    #[test]
    fn join_skips_empty_segments() {
        assert_eq!(
            join(["root", "", "domain", "Controller"]),
            "root.domain.Controller"
        );
    }

    #[test]
    fn join_returns_empty_string_when_all_segments_are_empty() {
        assert!(join(["", "", ""]).is_empty());
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
    fn resolve_explicit_path_does_not_double_prefix_rooted_paths() {
        let root_anchor = RootAnchor::new(Some("score::logging"));

        assert_eq!(
            resolve_explicit_path(&root_anchor, "score::logging::Recorder"),
            "score.logging.Recorder"
        );
    }

    #[test]
    fn internal_scope_from_resolved_path_strips_root_anchor_prefix() {
        let root_anchor = RootAnchor::new(Some("score::logging"));

        assert_eq!(
            internal_scope_from_resolved_path(
                &root_anchor,
                Some("score.logging.package_a.InternalInterface"),
            ),
            InternalScope::from_path("package_a.InternalInterface")
        );
    }

    #[test]
    fn internal_scope_from_resolved_path_keeps_unrooted_paths() {
        let root_anchor = RootAnchor::new(Some("score::logging"));

        assert_eq!(
            internal_scope_from_resolved_path(&root_anchor, Some("package_a.InternalInterface")),
            InternalScope::from_path("package_a.InternalInterface")
        );
        assert_eq!(
            internal_scope_from_resolved_path(&root_anchor, Some("score.logginging.Component")),
            InternalScope::from_path("score.logginging.Component")
        );
        assert_eq!(
            internal_scope_from_resolved_path(&root_anchor, None),
            InternalScope::default()
        );
    }

    #[test]
    fn is_explicit_path_accepts_cpp_and_dot_qualified_names() {
        assert!(is_explicit_path("score::mw::log::Recorder"));
        assert!(is_explicit_path("score.mw.log.Recorder"));
        assert!(!is_explicit_path("Recorder"));
    }
}
