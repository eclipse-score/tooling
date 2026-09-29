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

//! Helper functions shared by validators.

use std::collections::{BTreeMap, BTreeSet};

use source_location::SourceLocation;
use strsim::jaro_winkler;

pub(in crate::validators) const DEFAULT_SUGGESTION_THRESHOLD: f64 = 0.75;

/// Reduces `(id, source_location)` pairs to one entry per `id`, keeping the
/// earliest location (by file, then line) so the result doesn't depend on
/// iteration order.
pub(in crate::validators) fn earliest_source_by_id(
    entries: impl IntoIterator<Item = (String, SourceLocation)>,
) -> BTreeMap<String, SourceLocation> {
    let mut result: BTreeMap<String, SourceLocation> = BTreeMap::new();
    for (id, location) in entries {
        result
            .entry(id)
            .and_modify(|existing| {
                if location.display() < existing.display() {
                    *existing = location.clone();
                }
            })
            .or_insert(location);
    }
    result
}

pub(in crate::validators) fn extract_method_name(method: &str) -> &str {
    method.split('(').next().unwrap_or(method).trim()
}

pub(in crate::validators) fn intersect_interfaces(
    left_interfaces: &BTreeSet<String>,
    right_interfaces: &BTreeSet<String>,
) -> BTreeSet<String> {
    left_interfaces
        .intersection(right_interfaces)
        .cloned()
        .collect()
}

pub(in crate::validators) fn best_string_suggestion<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let mut best_candidate: Option<&str> = None;
    let mut best_score = 0.0;

    for candidate in candidates {
        let score = jaro_winkler(name, candidate);
        if score > best_score {
            best_score = score;
            best_candidate = Some(candidate);
        }
    }

    if best_score < DEFAULT_SUGGESTION_THRESHOLD {
        return None;
    }

    best_candidate.map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::{best_string_suggestion, earliest_source_by_id, DEFAULT_SUGGESTION_THRESHOLD};
    use source_location::SourceLocation;
    use strsim::jaro_winkler;

    #[test]
    fn best_string_suggestion_returns_none_for_empty_candidates() {
        assert_eq!(best_string_suggestion("Service", std::iter::empty()), None);
    }

    #[test]
    fn best_string_suggestion_accepts_exact_threshold_match() {
        let score = jaro_winkler("a", "baaa");
        assert_eq!(score, DEFAULT_SUGGESTION_THRESHOLD);

        assert_eq!(
            best_string_suggestion("a", ["baaa"]),
            Some("baaa".to_string())
        );
    }

    #[test]
    fn best_string_suggestion_rejects_below_threshold_match() {
        let score = jaro_winkler("abc", "xyz");
        assert!(score < DEFAULT_SUGGESTION_THRESHOLD);

        assert_eq!(best_string_suggestion("abc", ["xyz"]), None);
    }

    #[test]
    fn earliest_source_by_id_keeps_lexicographically_earlier_file() {
        let result = earliest_source_by_id([
            ("id_a".to_string(), SourceLocation::new("overview.puml", 1)),
            ("id_a".to_string(), SourceLocation::new("detail.puml", 1)),
        ]);

        assert_eq!(result["id_a"].display(), ("detail.puml".to_string(), 1));
    }

    #[test]
    fn earliest_source_by_id_keeps_lower_line_in_same_file() {
        let result = earliest_source_by_id([
            ("id_a".to_string(), SourceLocation::new("detail.puml", 10)),
            ("id_a".to_string(), SourceLocation::new("detail.puml", 3)),
        ]);

        assert_eq!(result["id_a"].display(), ("detail.puml".to_string(), 3));
    }

    #[test]
    fn earliest_source_by_id_is_order_independent() {
        let forward = earliest_source_by_id([
            ("id_a".to_string(), SourceLocation::new("detail.puml", 1)),
            ("id_a".to_string(), SourceLocation::new("overview.puml", 1)),
        ]);
        let reversed = earliest_source_by_id([
            ("id_a".to_string(), SourceLocation::new("overview.puml", 1)),
            ("id_a".to_string(), SourceLocation::new("detail.puml", 1)),
        ]);

        assert_eq!(forward["id_a"].display(), reversed["id_a"].display());
    }
}
