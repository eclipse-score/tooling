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

#[cfg(test)]
mod tests {
    use super::{is_identifier_path, join, normalize, normalized_segments, strip_anchor};

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
}
