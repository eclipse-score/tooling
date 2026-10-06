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

//! Shared cleanup for PlantUML display-name labels.
//!
//! Both the activity diagram's creole normalization and the sequence
//! resolver's participant identity derivation need to recognize the same set
//! of inline style markup tags (`<b>`, `<i>`, `<color:...>`, ...) and, for
//! sequence participants, decode literal `\n` escapes into real line breaks
//! before the first line is inspected. This module is the single source of
//! truth for both.

const KNOWN_TAGS: &[&str] = &[
    "b", "/b", "i", "/i", "u", "/u", "s", "/s", "w", "/w", "img", "/img", "font", "/font",
];
const STYLED_TAGS: &[&str] = &["color", "back", "size"];

/// Whether `tag` (the text between `<` and `>`, without the brackets) is a
/// recognized PlantUML inline style markup tag, e.g. `b`, `/b`, `color:red`
/// or `/color`.
fn is_style_markup_tag(tag: &str) -> bool {
    let tag = tag.trim().to_ascii_lowercase();

    KNOWN_TAGS.contains(&tag.as_str())
        || STYLED_TAGS.iter().any(|styled_tag| {
            tag.strip_prefix('/')
                .is_some_and(|rest| rest == *styled_tag)
                || tag
                    .split_once(':')
                    .is_some_and(|(name, _)| name == *styled_tag)
        })
}

/// Length in bytes (including both `<` and `>`) of a recognized style markup
/// tag at the start of `text`, or `None` if `text` doesn't start with one.
pub fn style_markup_tag_length(text: &str) -> Option<usize> {
    if !text.starts_with('<') {
        return None;
    }

    let end = text.find('>')?;

    is_style_markup_tag(&text[1..end]).then_some(end + 1)
}

/// Strips all recognized inline style markup tags from `text`, leaving
/// everything else (including unrecognized `<...>` sequences) untouched.
pub fn strip_style_markup(text: &str) -> String {
    let mut normalized = String::new();
    let mut index = 0;

    while index < text.len() {
        let remaining = &text[index..];

        if let Some(tag_len) = style_markup_tag_length(remaining) {
            index += tag_len;
            continue;
        }

        let ch = remaining.chars().next().expect("remaining is non-empty");
        normalized.push(ch);
        index += ch.len_utf8();
    }

    normalized
}

/// Decodes literal `\n` escape sequences, as PlantUML authors write them in
/// quoted labels, into real newline characters.
pub fn decode_newline_escapes(text: &str) -> String {
    text.replace("\\n", "\n")
}

/// Identity-label cleanup: decode `\n` escapes into line breaks and strip
/// inline style markup, so the result is ready for first-line inspection.
pub fn normalize_identity_label(text: &str) -> String {
    strip_style_markup(&decode_newline_escapes(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_style_markup_removes_known_tags() {
        assert_eq!(strip_style_markup("<b>bold</b> plain"), "bold plain");
        assert_eq!(
            strip_style_markup("<color:red>red</color> text"),
            "red text"
        );
        assert_eq!(strip_style_markup("<back:yellow>hl</back>"), "hl");
        assert_eq!(strip_style_markup("<size:14>big</size>"), "big");
    }

    #[test]
    fn strip_style_markup_keeps_unknown_tags() {
        assert_eq!(
            strip_style_markup("<unknown>text</unknown>"),
            "<unknown>text</unknown>"
        );
    }

    #[test]
    fn strip_style_markup_is_case_insensitive_and_trims_whitespace() {
        assert_eq!(strip_style_markup("<B>bold</B>"), "bold");
        assert_eq!(strip_style_markup("< b >bold< /b >"), "bold");
        assert_eq!(strip_style_markup("<FONT>text</FONT>"), "text");
    }

    #[test]
    fn strip_style_markup_keeps_non_ascii_text() {
        assert_eq!(strip_style_markup("<b>\u{fc}ber</b>"), "\u{fc}ber");
    }

    #[test]
    fn decode_newline_escapes_turns_backslash_n_into_newline() {
        assert_eq!(decode_newline_escapes("a\\nb"), "a\nb");
    }

    #[test]
    fn normalize_identity_label_decodes_then_strips() {
        assert_eq!(
            normalize_identity_label("<b>Service</b>\\nignored"),
            "Service\nignored"
        );
    }
}
