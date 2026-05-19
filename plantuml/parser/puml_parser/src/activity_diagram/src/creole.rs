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

pub(crate) fn normalize_creole_text(text: &str) -> String {
    let mut normalized_lines = Vec::new();

    for line in text.lines() {
        normalized_lines.push(normalize_creole_line(line));
    }

    while normalized_lines.last().is_some_and(|line| line.is_empty()) {
        normalized_lines.pop();
    }

    normalized_lines.join("\n")
}

fn normalize_creole_line(line: &str) -> String {
    let trimmed = line.trim();

    if trimmed.is_empty() || is_horizontal_rule(trimmed) {
        return String::new();
    }

    if let Some(heading) = strip_heading_markup(trimmed) {
        return normalize_creole_inline(heading.trim());
    }

    if looks_like_table_row(trimmed) {
        return normalize_table_row(trimmed);
    }

    let content = strip_list_marker(trimmed).unwrap_or(trimmed);

    normalize_creole_inline(content)
}

fn normalize_creole_inline(text: &str) -> String {
    const INLINE_MARKERS: [&str; 8] = ["**", "//", "__", "~~", "\"\"", "##", "^^", ",,"];

    let mut normalized = String::new();
    let mut active_marker: Option<&'static str> = None;
    let mut index = 0;

    while index < text.len() {
        let remaining = &text[index..];

        if let Some(content) = extract_bracketed_content(remaining, "[[", "]]") {
            normalized.push_str(&normalize_link_content(&content));
            index += content.len() + 4;
            continue;
        }

        if let Some(content) = extract_bracketed_content(remaining, "{{", "}}") {
            normalized.push_str(&normalize_image_content(&content));
            index += content.len() + 4;
            continue;
        }

        if let Some(tag_len) = creole_tag_length(remaining) {
            index += tag_len;
            continue;
        }

        let mut consumed_marker = false;
        for marker in INLINE_MARKERS {
            if remaining.starts_with(marker) {
                match active_marker {
                    Some(active) if active == marker => {
                        active_marker = None;
                        index += marker.len();
                        consumed_marker = true;
                    }
                    None if should_open_inline_marker(text, index, marker) => {
                        active_marker = Some(marker);
                        index += marker.len();
                        consumed_marker = true;
                    }
                    _ => {}
                }
                if consumed_marker {
                    break;
                }
            }
        }

        if consumed_marker {
            continue;
        }

        if let Some(next) = remaining.strip_prefix('~') {
            if let Some(ch) = next.chars().next() {
                normalized.push(ch);
                index += '~'.len_utf8() + ch.len_utf8();
            } else {
                index += '~'.len_utf8();
            }
            continue;
        }

        let ch = remaining.chars().next().expect("remaining is non-empty");
        normalized.push(ch);
        index += ch.len_utf8();
    }

    collapse_internal_whitespace(&normalized)
}

fn collapse_internal_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn should_open_inline_marker(text: &str, index: usize, marker: &str) -> bool {
    let after_index = index + marker.len();
    let before = text[..index].chars().next_back();
    let after = text[after_index..].chars().next();

    if after.is_none() || after.is_some_and(char::is_whitespace) {
        return false;
    }

    if marker == "//" && before == Some(':') {
        return false;
    }

    text[after_index..].contains(marker)
}

fn extract_bracketed_content(text: &str, open: &str, close: &str) -> Option<String> {
    if !text.starts_with(open) {
        return None;
    }

    let content = &text[open.len()..];
    let end = content.find(close)?;

    Some(content[..end].to_string())
}

fn normalize_link_content(content: &str) -> String {
    let trimmed = content.trim();
    let (target, label) = split_first_unescaped_whitespace(trimmed);

    if let Some(label) = label {
        return normalize_creole_inline(label.trim());
    }

    strip_tooltip(target.trim()).to_string()
}

fn normalize_image_content(content: &str) -> String {
    let trimmed = content.trim();

    if let Some((_, caption)) = trimmed.split_once('|') {
        return normalize_creole_inline(caption.trim());
    }

    String::new()
}

fn split_first_unescaped_whitespace(text: &str) -> (&str, Option<&str>) {
    let mut brace_depth = 0;

    for (index, ch) in text.char_indices() {
        match ch {
            '{' => brace_depth += 1,
            '}' if brace_depth > 0 => brace_depth -= 1,
            _ if brace_depth == 0 && ch.is_whitespace() => {
                let tail = text[index..].trim();
                return (&text[..index], (!tail.is_empty()).then_some(tail));
            }
            _ => {}
        }
    }

    (text, None)
}

fn strip_tooltip(text: &str) -> &str {
    match text.find('{') {
        Some(index) => &text[..index],
        None => text,
    }
}

fn creole_tag_length(text: &str) -> Option<usize> {
    if !text.starts_with('<') {
        return None;
    }

    let end = text.find('>')?;
    let tag = &text[1..end].trim().to_ascii_lowercase();

    let known_tags = [
        "b", "/b", "i", "/i", "u", "/u", "s", "/s", "w", "/w", "img", "/img", "font", "/font",
    ];

    if known_tags.contains(&tag.as_str())
        || tag.starts_with("color:")
        || tag.starts_with("back:")
        || tag.starts_with("size:")
    {
        Some(end + 1)
    } else {
        None
    }
}

fn looks_like_table_row(text: &str) -> bool {
    text.starts_with('|') && text.ends_with('|') && text.len() >= 2
}

fn normalize_table_row(text: &str) -> String {
    text.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().trim_start_matches('=').trim())
        .filter(|cell| !cell.is_empty())
        .map(normalize_creole_inline)
        .collect::<Vec<_>>()
        .join(" | ")
}

fn strip_heading_markup(text: &str) -> Option<&str> {
    let leading = text.chars().take_while(|ch| *ch == '=').count();
    let trailing = text.chars().rev().take_while(|ch| *ch == '=').count();

    if leading == 0 || trailing == 0 || text.len() <= leading + trailing {
        return None;
    }

    Some(&text[leading..text.len() - trailing])
}

fn strip_list_marker(text: &str) -> Option<&str> {
    let marker_len = text
        .chars()
        .take_while(|ch| matches!(ch, '*' | '#'))
        .count();

    if marker_len == 0 {
        return None;
    }

    if !text[marker_len..]
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
    {
        return None;
    }

    let remainder = text[marker_len..].trim_start();
    Some(remainder)
}

fn is_horizontal_rule(text: &str) -> bool {
    text.len() >= 4 && text.chars().all(|ch| ch == '-')
}

#[cfg(test)]
mod tests {
    use super::normalize_creole_text;

    #[test]
    fn normalize_creole_inline_styles_to_plain_text() {
        assert_eq!(
            normalize_creole_text("Hello \"\"code\"\" and **bold** with __underline__ ~~wave~~"),
            "Hello code and bold with underline wave"
        );
    }

    #[test]
    fn normalize_creole_links_images_and_escapes() {
        assert_eq!(
            normalize_creole_text(
                "[[https://example.com/docs{tip} external docs]] {{img.png|preview}} ~*literal"
            ),
            "external docs preview *literal"
        );
    }

    #[test]
    fn normalize_creole_block_syntax_to_plain_text() {
        assert_eq!(
            normalize_creole_text("= Heading =\n* first item\n|=A|B|\n----\n"),
            "Heading\nfirst item\nA | B"
        );
    }

    #[test]
    fn normalize_full_line_bold_text_without_list_stripping() {
        assert_eq!(normalize_creole_text("**action green**"), "action green");
    }
}
