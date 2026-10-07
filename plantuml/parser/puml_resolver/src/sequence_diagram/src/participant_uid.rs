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

//! Participant uid derivation (spec `element-identifiers.md`, Rule A′).

use uid_normalization::{identity_name, join, normalized_segments, IdentityKind};

/// Derives the uid of a participant from its written name, or the reason it
/// has none. The alias is only a local reference key and plays no part.
pub(crate) fn participant_uid(display_name: &str) -> Result<String, &'static str> {
    let name = identity_name(display_name, IdentityKind::Other).map_err(|error| error.reason())?;

    Ok(join(normalized_segments(&name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uid_normalization::IdentityError;

    #[test]
    fn qualified_name_is_the_path() {
        assert_eq!(participant_uid("a::b::C").as_deref(), Ok("a.b.C"));
        assert_eq!(participant_uid("comp.unit_1").as_deref(), Ok("comp.unit_1"));
    }

    #[test]
    fn leading_root_marker_is_dropped() {
        assert_eq!(participant_uid("::a::X").as_deref(), Ok("a.X"));
        assert_eq!(participant_uid(".a.X").as_deref(), Ok("a.X"));
        assert_eq!(participant_uid("::X").as_deref(), Ok("X"));
    }

    #[test]
    fn bare_name_is_the_uid() {
        assert_eq!(participant_uid("Client").as_deref(), Ok("Client"));
        assert_eq!(participant_uid("<b>Client</b>").as_deref(), Ok("Client"));
        assert_eq!(
            participant_uid("ExternalEndpoint").as_deref(),
            Ok("ExternalEndpoint")
        );
    }

    #[test]
    fn free_text_is_rejected() {
        for label in ["Order Service", "", "backend : a::B", "my-service"] {
            assert_eq!(
                participant_uid(label),
                Err(IdentityError::FreeText.reason()),
                "{label}"
            );
        }
    }

    #[test]
    fn malformed_path_is_rejected() {
        for label in [
            "a.",
            "a..b",
            "a::",
            "a:::b",
            "::",
            ":logging::I",
            "my-pkg::unit",
        ] {
            assert_eq!(
                participant_uid(label),
                Err(IdentityError::MalformedPath.reason()),
                "{label}"
            );
        }
    }

    #[test]
    fn only_the_first_line_of_a_label_counts() {
        assert_eq!(participant_uid("comp::b\\nextra").as_deref(), Ok("comp.b"));
        assert_eq!(participant_uid("\\ncomp::b").as_deref(), Ok("comp.b"));
    }

    #[test]
    fn slash_n_is_text_not_a_line_break() {
        assert_eq!(
            participant_uid("comp::a/nextra"),
            Err(IdentityError::MalformedPath.reason())
        );
    }
}
