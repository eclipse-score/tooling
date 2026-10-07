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

/// Matches `uid` against `ids`. An id equal to the uid wins. Otherwise a
/// single-segment uid is a leaf reference to every id whose last segment
/// equals it. A qualified uid never matches by suffix.
pub fn resolve_uid<'a>(uid: &str, ids: impl IntoIterator<Item = &'a str>) -> UidMatch<'a> {
    let uid = normalize(uid);

    if uid.is_empty() {
        return UidMatch::Unresolved;
    }

    let ids: Vec<(&str, String)> = ids.into_iter().map(|id| (id, normalize(id))).collect();

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
    use super::{is_identifier_path, join, normalize, normalized_segments, resolve_uid, UidMatch};

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
    fn resolve_uid_matches_qualified_uid_exactly() {
        let ids = ["unit_1.Controller", "unit_2.Controller"];
        assert_eq!(
            resolve_uid("unit_2.Controller", ids),
            UidMatch::Resolved("unit_2.Controller")
        );
        assert_eq!(
            resolve_uid("unit_2::Controller", ids),
            UidMatch::Resolved("unit_2.Controller")
        );
    }

    #[test]
    fn resolve_uid_does_not_match_qualified_uid_by_suffix() {
        assert_eq!(
            resolve_uid("b.Controller", ["a.b.Controller"]),
            UidMatch::Unresolved
        );
    }

    #[test]
    fn resolve_uid_links_unique_leaf_reference() {
        assert_eq!(
            resolve_uid("Controller", ["unit_1.Controller", "unit_1.Repository"]),
            UidMatch::Resolved("unit_1.Controller")
        );
    }

    #[test]
    fn resolve_uid_reports_ambiguous_leaf_reference_sorted() {
        assert_eq!(
            resolve_uid("Controller", ["unit_2.Controller", "unit_1.Controller"]),
            UidMatch::Ambiguous(vec!["unit_1.Controller", "unit_2.Controller"])
        );
    }

    #[test]
    fn resolve_uid_prefers_exact_id_over_leaf_matches() {
        assert_eq!(
            resolve_uid("Controller", ["unit_1.Controller", "Controller"]),
            UidMatch::Resolved("Controller")
        );
    }

    #[test]
    fn resolve_uid_reports_unresolved_when_nothing_matches() {
        assert_eq!(
            resolve_uid("Missing", ["unit_1.Controller"]),
            UidMatch::Unresolved
        );
        assert_eq!(resolve_uid("", ["unit_1.Controller"]), UidMatch::Unresolved);
    }
}
