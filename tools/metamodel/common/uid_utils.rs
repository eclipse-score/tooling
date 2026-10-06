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

/// Normalizes `::` and `.` separators to `.` and trims leading/trailing `.`.
/// Interior empty segments (e.g. `"a..b"`) are kept, and whitespace is not
/// trimmed; callers that need either normalize the input themselves.
pub fn normalize(value: &str) -> String {
    value.replace("::", ".").trim_matches('.').to_string()
}

pub fn normalized_segments(value: &str) -> Vec<String> {
    normalize(value)
        .split('.')
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn join<I, S>(parts: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parts
        .into_iter()
        .filter(|part| !part.as_ref().is_empty())
        .map(|part| part.as_ref().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Returns `true` if `value` looks like a qualified identifier path (contains
/// a `.` or `::` separator) rather than a single unqualified name.
pub fn is_identifier_path(value: &str) -> bool {
    value.contains('.') || value.contains("::")
}

/// Strips a leading `anchor` prefix from `value`, returning the remainder
/// (with any leading separator removed). If `anchor` is `None`, empty, or is
/// not a prefix of `value`, the normalized `value` is returned unchanged.
pub fn strip_anchor(value: &str, anchor: Option<&str>) -> String {
    let normalized_value = normalize(value);
    let Some(anchor) = anchor.filter(|anchor| !anchor.is_empty()) else {
        return normalized_value;
    };
    let value_segments = normalized_segments(value);
    let anchor_segments = normalized_segments(anchor);

    if !value_segments.starts_with(&anchor_segments) {
        return normalized_value;
    }

    join(&value_segments[anchor_segments.len()..])
}

/// Outcome of matching a uid against a set of element ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UidMatch<'a> {
    /// Exactly one id matches.
    Resolved(&'a str),
    /// A leaf reference matches several ids (sorted).
    Ambiguous(Vec<&'a str>),
    /// No id matches.
    Unresolved,
}

/// Matches `uid` against `ids` after stripping `anchor` from both. An id equal
/// to the uid wins. Otherwise a single-segment uid is a leaf reference to every
/// id whose last segment equals it. A qualified uid never matches by suffix.
pub fn resolve_uid<'a>(
    uid: &str,
    anchor: Option<&str>,
    ids: impl IntoIterator<Item = &'a str>,
) -> UidMatch<'a> {
    let uid = strip_anchor(uid, anchor);

    if uid.is_empty() {
        return UidMatch::Unresolved;
    }

    let ids: Vec<(&str, String)> = ids
        .into_iter()
        .map(|id| (id, strip_anchor(id, anchor)))
        .collect();

    if let Some((id, _)) = ids.iter().find(|(_, local)| *local == uid) {
        return UidMatch::Resolved(id);
    }

    if is_identifier_path(&uid) {
        return UidMatch::Unresolved;
    }

    let mut leaf_matches: Vec<&str> = ids
        .into_iter()
        .filter(|(_, local)| local.rsplit('.').next() == Some(uid.as_str()))
        .map(|(id, _)| id)
        .collect();
    leaf_matches.sort_unstable();
    leaf_matches.dedup();

    match leaf_matches.as_slice() {
        [] => UidMatch::Unresolved,
        [id] => UidMatch::Resolved(id),
        _ => UidMatch::Ambiguous(leaf_matches),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_identifier_path, join, normalize, normalized_segments, resolve_uid, strip_anchor,
        UidMatch,
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
    fn is_identifier_path_accepts_cpp_and_dot_qualified_names() {
        assert!(is_identifier_path("score::mw::log::Recorder"));
        assert!(is_identifier_path("score.mw.log.Recorder"));
        assert!(!is_identifier_path("Recorder"));
    }

    #[test]
    fn strip_anchor_strips_matching_prefix() {
        assert_eq!(
            strip_anchor(
                "score.logging.package_a.InternalInterface",
                Some("score::logging")
            ),
            "package_a.InternalInterface"
        );
    }

    #[test]
    fn strip_anchor_returns_empty_string_when_value_equals_anchor() {
        assert_eq!(strip_anchor("score::logging", Some("score.logging")), "");
    }

    #[test]
    fn strip_anchor_keeps_unrooted_values_unchanged() {
        assert_eq!(
            strip_anchor("package_a.InternalInterface", Some("score::logging")),
            "package_a.InternalInterface"
        );
        assert_eq!(
            strip_anchor("score.logginging.Component", Some("score::logging")),
            "score.logginging.Component"
        );
    }

    #[test]
    fn strip_anchor_skips_interior_empty_segments() {
        assert_eq!(strip_anchor("score.logging..x", Some("score.logging")), "x");
    }

    #[test]
    fn strip_anchor_returns_normalized_value_when_anchor_is_absent() {
        assert_eq!(
            strip_anchor("score::logging::Recorder", None),
            "score.logging.Recorder"
        );
    }

    #[test]
    fn resolve_uid_matches_qualified_uid_exactly() {
        let ids = ["unit_1.Controller", "unit_2.Controller"];
        assert_eq!(
            resolve_uid("unit_2.Controller", None, ids),
            UidMatch::Resolved("unit_2.Controller")
        );
        assert_eq!(
            resolve_uid("unit_2::Controller", None, ids),
            UidMatch::Resolved("unit_2.Controller")
        );
    }

    #[test]
    fn resolve_uid_does_not_match_qualified_uid_by_suffix() {
        assert_eq!(
            resolve_uid("b.Controller", None, ["a.b.Controller"]),
            UidMatch::Unresolved
        );
    }

    #[test]
    fn resolve_uid_links_unique_leaf_reference() {
        assert_eq!(
            resolve_uid(
                "Controller",
                None,
                ["unit_1.Controller", "unit_1.Repository"]
            ),
            UidMatch::Resolved("unit_1.Controller")
        );
    }

    #[test]
    fn resolve_uid_reports_ambiguous_leaf_reference_sorted() {
        assert_eq!(
            resolve_uid(
                "Controller",
                None,
                ["unit_2.Controller", "unit_1.Controller"]
            ),
            UidMatch::Ambiguous(vec!["unit_1.Controller", "unit_2.Controller"])
        );
    }

    #[test]
    fn resolve_uid_prefers_exact_id_over_leaf_matches() {
        assert_eq!(
            resolve_uid("Controller", None, ["unit_1.Controller", "Controller"]),
            UidMatch::Resolved("Controller")
        );
    }

    #[test]
    fn resolve_uid_reports_unresolved_when_nothing_matches() {
        assert_eq!(
            resolve_uid("Missing", None, ["unit_1.Controller"]),
            UidMatch::Unresolved
        );
        assert_eq!(
            resolve_uid("", None, ["unit_1.Controller"]),
            UidMatch::Unresolved
        );
    }

    #[test]
    fn resolve_uid_strips_anchor_from_uid_and_ids() {
        let ids = ["score.mw.unit_1.Controller", "score.mw.unit_2.Controller"];
        assert_eq!(
            resolve_uid("unit_2.Controller", Some("score::mw"), ids),
            UidMatch::Resolved("score.mw.unit_2.Controller")
        );
        assert_eq!(
            resolve_uid("score.mw.unit_2.Controller", Some("score.mw"), ids),
            UidMatch::Resolved("score.mw.unit_2.Controller")
        );
    }

    #[test]
    fn resolve_uid_treats_anchor_stripped_single_segment_as_leaf() {
        assert_eq!(
            resolve_uid(
                "score.mw.Controller",
                Some("score.mw"),
                ["score.mw.a.Controller"]
            ),
            UidMatch::Resolved("score.mw.a.Controller")
        );
    }

    #[test]
    fn resolve_uid_reports_unresolved_for_the_anchor_itself() {
        assert_eq!(
            resolve_uid("score.mw", Some("score.mw"), ["score.mw.Controller"]),
            UidMatch::Unresolved
        );
    }
}
