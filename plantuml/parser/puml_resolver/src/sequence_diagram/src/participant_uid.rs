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

//! Participant uid derivation (spec `element-identifiers.md`, Rule B).

use puml_utils::normalize_identity_label;
use uid_normalization::{
    is_identifier_path, normalized_segments, resolve_explicit_path, strip_root_marker, RootAnchor,
};

/// Reserved participant for actors outside the described architecture (Rule E).
pub(crate) const EXTERNAL_ENDPOINT: &str = "ExternalEndpoint";

pub(crate) const FREE_TEXT_REASON: &str =
    "free-text participant display names require an alias for uid derivation";

pub(crate) const MALFORMED_PATH_REASON: &str = "participant name is not a valid qualified path \
     (non-empty segments of letters, digits and `_` separated by `.` or `::`), \
     fix the path or add an alias";

/// Derives the uid of a participant, or the reason it has none.
///
/// The first non-empty line of the display name is the identity when it is a
/// qualified path. Otherwise the alias is, else that line if it is a plain
/// identifier.
pub(crate) fn participant_uid(
    display_name: &str,
    alias: Option<&str>,
    root_anchor: &RootAnchor,
) -> Result<String, &'static str> {
    let label = normalize_identity_label(display_name);
    let first_line = label
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();

    let identity = if is_qualified_path(first_line) {
        first_line
    } else if let Some(alias) = alias {
        alias
    } else if is_plain_identifier(first_line) {
        first_line
    } else if is_path_like(first_line) {
        return Err(MALFORMED_PATH_REASON);
    } else {
        return Err(FREE_TEXT_REASON);
    };

    if normalized_segments(identity).is_empty() {
        return Err(FREE_TEXT_REASON);
    }

    if identity == EXTERNAL_ENDPOINT {
        return Ok(EXTERNAL_ENDPOINT.to_string());
    }

    Ok(resolve_explicit_path(root_anchor, identity))
}

/// A path with `.` or `::` separators and an optional leading root marker,
/// whose segments are `[A-Za-z0-9_]`.
fn is_qualified_path(text: &str) -> bool {
    if !is_identifier_path(text) {
        return false;
    }

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

/// A name the diagram grammar accepts without quotes.
fn is_plain_identifier(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '@'))
}

/// A single token with a path separator, so the author meant a path.
fn is_path_like(text: &str) -> bool {
    is_identifier_path(text) && !text.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(display_name: &str, alias: Option<&str>) -> Result<String, &'static str> {
        participant_uid(display_name, alias, &RootAnchor::default())
    }

    fn anchored_uid(display_name: &str, alias: Option<&str>) -> Result<String, &'static str> {
        participant_uid(display_name, alias, &RootAnchor::new(Some("score::mw")))
    }

    #[test]
    fn qualified_label_wins_over_alias() {
        assert_eq!(
            uid("comp::unit_1", Some("u1")).as_deref(),
            Ok("comp.unit_1")
        );
        assert_eq!(uid("comp.unit_1", Some("u1")).as_deref(), Ok("comp.unit_1"));
    }

    #[test]
    fn qualified_label_without_alias_is_the_path() {
        assert_eq!(uid("a::b::C", None).as_deref(), Ok("a.b.C"));
    }

    #[test]
    fn leading_root_marker_is_dropped() {
        assert_eq!(uid("::a::X", Some("x")).as_deref(), Ok("a.X"));
        assert_eq!(uid(".a.X", Some("x")).as_deref(), Ok("a.X"));
        assert_eq!(uid("::X", Some("x")).as_deref(), Ok("X"));
    }

    #[test]
    fn prose_label_falls_back_to_the_alias() {
        assert_eq!(uid("Unit 1", Some("unit_1")).as_deref(), Ok("unit_1"));
    }

    #[test]
    fn instance_type_label_is_prose() {
        assert_eq!(
            uid("backend : logging::Recorder::Backend", Some("Backend")).as_deref(),
            Ok("Backend")
        );
        assert_eq!(
            uid(":logging::IBackend", Some("IBackend")).as_deref(),
            Ok("IBackend")
        );
    }

    #[test]
    fn bare_name_without_alias_is_the_uid() {
        assert_eq!(uid("Client", None).as_deref(), Ok("Client"));
        assert_eq!(uid("my-service", None).as_deref(), Ok("my-service"));
        assert_eq!(uid("<b>Client</b>", None).as_deref(), Ok("Client"));
    }

    #[test]
    fn free_text_without_alias_is_rejected() {
        assert_eq!(uid("Order Service", None), Err(FREE_TEXT_REASON));
        assert_eq!(uid("", None), Err(FREE_TEXT_REASON));
        assert_eq!(uid("backend : a::B", None), Err(FREE_TEXT_REASON));
    }

    #[test]
    fn malformed_path_without_alias_is_rejected() {
        for label in ["a.", "a..b", "a::", "a:::b", "my-pkg::unit", ":logging::I"] {
            assert_eq!(uid(label, None), Err(MALFORMED_PATH_REASON), "{label}");
        }
    }

    #[test]
    fn malformed_paths_are_not_paths() {
        assert_eq!(uid("a::", Some("x")).as_deref(), Ok("x"));
        assert_eq!(uid("a..b", Some("x")).as_deref(), Ok("x"));
        assert_eq!(uid("a:::b", Some("x")).as_deref(), Ok("x"));
        assert_eq!(uid("::", Some("x")).as_deref(), Ok("x"));
    }

    #[test]
    fn markup_is_stripped_before_the_path_check() {
        assert_eq!(
            uid("<b>comp::unit</b>", Some("u")).as_deref(),
            Ok("comp.unit")
        );
    }

    #[test]
    fn only_the_first_line_of_a_label_counts() {
        assert_eq!(uid("comp::b\\nextra", Some("y")).as_deref(), Ok("comp.b"));
        assert_eq!(uid("\\ncomp::b", Some("y")).as_deref(), Ok("comp.b"));
    }

    #[test]
    fn slash_n_is_text_not_a_line_break() {
        assert_eq!(uid("comp::a/nextra", Some("x")).as_deref(), Ok("x"));
    }

    #[test]
    fn external_endpoint_is_verbatim() {
        assert_eq!(
            anchored_uid("ExternalEndpoint", None).as_deref(),
            Ok("ExternalEndpoint")
        );
        assert_eq!(
            anchored_uid("Outside", Some("ExternalEndpoint")).as_deref(),
            Ok("ExternalEndpoint")
        );
        assert_eq!(
            anchored_uid("ExternalEndpoint", Some("ext")).as_deref(),
            Ok("score.mw.ext")
        );
    }

    #[test]
    fn root_anchor_prefixes_every_other_uid() {
        assert_eq!(
            anchored_uid("comp::unit", Some("u")).as_deref(),
            Ok("score.mw.comp.unit")
        );
        assert_eq!(anchored_uid("Unit", Some("u")).as_deref(), Ok("score.mw.u"));
        assert_eq!(
            anchored_uid("Client", None).as_deref(),
            Ok("score.mw.Client")
        );
        assert_eq!(
            anchored_uid("x::ExternalEndpoint", None).as_deref(),
            Ok("score.mw.x.ExternalEndpoint")
        );
    }
}
