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
        .map(|part| part.as_ref().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::{join, normalize, normalized_segments};

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
}
