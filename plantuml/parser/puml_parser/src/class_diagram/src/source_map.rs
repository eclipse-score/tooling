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
use parser_core::BaseParseError;
use pest::error::{ErrorVariant, InputLocation, LineColLocation};
use std::fmt::Debug;

type SourceCoord = (usize, usize);

#[derive(Clone, Debug, PartialEq, Eq)]
struct NormalizedLineMap {
    column_map: Vec<SourceCoord>,
    fallback: SourceCoord,
}

impl NormalizedLineMap {
    fn map_column(&self, column: usize) -> SourceCoord {
        if column == 0 {
            return self.fallback;
        }

        self.column_map
            .get(column.saturating_sub(1))
            .copied()
            .unwrap_or_else(|| {
                self.column_map
                    .last()
                    .map(|(line, col)| (*line, col + 1))
                    .unwrap_or(self.fallback)
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NormalizedLine {
    text: String,
    column_map: Vec<SourceCoord>,
    fallback: SourceCoord,
}

impl NormalizedLine {
    fn from_raw_line(line: &str, line_number: usize) -> Self {
        let char_count = line.chars().count();
        let column_map = (1..=char_count)
            .map(|column| (line_number, column))
            .collect();

        Self {
            text: line.to_string(),
            column_map,
            fallback: (line_number, char_count + 1),
        }
    }

    fn from_member_line(line: &str, line_number: usize) -> Self {
        let chars: Vec<char> = line.chars().collect();
        let mut text = String::new();
        let mut column_map = Vec::new();
        let mut index = 0;
        let mut column = 1;

        while index < chars.len() {
            if chars[index] == '\\' && chars.get(index + 1) == Some(&'n') {
                text.push(' ');
                column_map.push((line_number, column));
                index += 2;
                column += 2;
                continue;
            }

            text.push(chars[index]);
            column_map.push((line_number, column));
            index += 1;
            column += 1;
        }

        Self {
            text,
            column_map,
            fallback: (line_number, column),
        }
    }

    fn trim_end_whitespace(&mut self) {
        while self
            .text
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
        {
            self.text.pop();
            self.column_map.pop();
        }
    }

    fn trim_whitespace(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        let start = chars.iter().take_while(|ch| ch.is_whitespace()).count();
        let end_trim = chars
            .iter()
            .rev()
            .take_while(|ch| ch.is_whitespace())
            .count();
        let end = chars.len().saturating_sub(end_trim);

        if start == 0 && end == chars.len() {
            return;
        }

        if start >= end {
            self.text.clear();
            self.column_map.clear();
            return;
        }

        self.text = chars[start..end].iter().collect();
        self.column_map = self.column_map[start..end].to_vec();
    }

    fn trim_continuation_marker(&mut self) {
        self.trim_end_whitespace();

        if self.text.ends_with('\\') {
            self.text.pop();
            self.column_map.pop();
        }

        self.trim_end_whitespace();
    }

    fn append_trimmed_continuation(&mut self, mut continuation: Self) {
        continuation.trim_whitespace();

        if continuation.text.is_empty() {
            return;
        }

        if !self.text.ends_with(' ') {
            let join_coord = continuation
                .column_map
                .first()
                .copied()
                .unwrap_or(continuation.fallback);
            self.text.push(' ');
            self.column_map.push(join_coord);
        }

        self.text.push_str(&continuation.text);
        self.column_map.extend(continuation.column_map);
        self.fallback = continuation.fallback;
    }

    fn into_parts(self) -> (String, NormalizedLineMap) {
        (
            self.text,
            NormalizedLineMap {
                column_map: self.column_map,
                fallback: self.fallback,
            },
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NormalizedContent {
    content: String,
    line_maps: Vec<NormalizedLineMap>,
}

impl NormalizedContent {
    pub(crate) fn as_str(&self) -> &str {
        &self.content
    }

    pub(crate) fn map_position(&self, line: usize, column: usize) -> SourceCoord {
        self.line_maps
            .get(line.saturating_sub(1))
            .map(|line_map| line_map.map_column(column))
            .unwrap_or((line, column))
    }
}

fn is_member_declaration_line(line: &str) -> bool {
    let mut trimmed = line.trim_start();

    if trimmed.is_empty() || trimmed.starts_with('"') || trimmed.starts_with('\'') {
        return false;
    }

    while trimmed.starts_with('{') {
        let Some(end) = trimmed.find('}') else {
            break;
        };
        trimmed = trimmed[end + 1..].trim_start();
    }

    matches!(trimmed.chars().next(), Some('+' | '-' | '#' | '~'))
}

pub(crate) fn normalize_multiline_member_signatures(content: &str) -> NormalizedContent {
    let mut normalized_lines = Vec::new();
    let mut pending_member: Option<NormalizedLine> = None;

    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;

        if let Some(mut pending) = pending_member.take() {
            let continuation = NormalizedLine::from_member_line(line, line_number);
            pending.append_trimmed_continuation(continuation);

            if line.trim_end().ends_with('\\') {
                pending.trim_continuation_marker();
                pending_member = Some(pending);
            } else {
                normalized_lines.push(pending);
            }

            continue;
        }

        if is_member_declaration_line(line) {
            let mut normalized = NormalizedLine::from_member_line(line, line_number);

            if line.trim_end().ends_with('\\') {
                normalized.trim_continuation_marker();
                pending_member = Some(normalized);
            } else {
                normalized_lines.push(normalized);
            }
        } else {
            normalized_lines.push(NormalizedLine::from_raw_line(line, line_number));
        }
    }

    if let Some(pending) = pending_member {
        normalized_lines.push(pending);
    }

    let mut content_out = String::new();
    let mut line_maps = Vec::with_capacity(normalized_lines.len());

    for (index, line) in normalized_lines.into_iter().enumerate() {
        if index > 0 {
            content_out.push('\n');
        }

        let (text, line_map) = line.into_parts();
        content_out.push_str(&text);
        line_maps.push(line_map);
    }

    if content.ends_with('\n') {
        content_out.push('\n');
    }

    NormalizedContent {
        content: content_out,
        line_maps,
    }
}

fn original_source_line(content: &str, line: usize) -> String {
    content
        .split_inclusive('\n')
        .nth(line.saturating_sub(1))
        .map(|source_line| source_line.trim_matches(' ').to_string())
        .unwrap_or("<no source line>\n".to_string())
}

fn build_syntax_error_message<Rule>(
    line: usize,
    column: usize,
    cause: Option<&pest::error::Error<Rule>>,
    fallback: String,
) -> String
where
    Rule: Debug,
{
    match cause.map(|error| &error.variant) {
        Some(ErrorVariant::ParsingError {
            positives,
            negatives,
        }) => {
            format!(
                "Parsing error at {:?}, expected {:?}, got {:?}",
                (line, column),
                positives,
                negatives
            )
        }
        Some(ErrorVariant::CustomError { message }) => message.clone(),
        None => fallback,
    }
}

fn source_byte_offset(content: &str, line: usize, column: usize) -> usize {
    let mut offset = 0;

    for (index, source_line) in content.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            let column_offset = source_line
                .chars()
                .take(column.saturating_sub(1))
                .map(char::len_utf8)
                .sum::<usize>();
            return offset + column_offset.min(source_line.len());
        }

        offset += source_line.len();
    }

    content.len()
}

fn remap_pest_cause_to_original_source<Rule>(
    cause: &mut pest::error::Error<Rule>,
    original_content: &str,
    original_line: usize,
    original_column: usize,
) {
    cause.line_col = LineColLocation::Pos((original_line, original_column));
    cause.location = InputLocation::Pos(source_byte_offset(
        original_content,
        original_line,
        original_column,
    ));
}

pub(crate) fn remap_syntax_error_to_original_source<Rule>(
    error: BaseParseError<Rule>,
    original_content: &str,
    normalized_content: &NormalizedContent,
) -> BaseParseError<Rule>
where
    Rule: Debug,
{
    match error {
        BaseParseError::SyntaxError {
            file,
            line,
            column,
            message,
            source_line: _,
            cause,
        } => {
            let (original_line, original_column) = normalized_content.map_position(line, column);
            let mut cause = cause;

            if let Some(cause) = cause.as_deref_mut() {
                remap_pest_cause_to_original_source(
                    cause,
                    original_content,
                    original_line,
                    original_column,
                );
            }

            let message = build_syntax_error_message(
                original_line,
                original_column,
                cause.as_deref(),
                message,
            );

            BaseParseError::SyntaxError {
                file,
                line: original_line,
                column: original_column,
                message,
                source_line: original_source_line(original_content, original_line),
                cause,
            }
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_multiline_member_signatures_preserves_column_mapping() {
        let normalized = normalize_multiline_member_signatures(
            "+ ChangeHandler(amp::pmr::unique_ptr<ExecutorFactory>,\\\n                               \\n  std::shared_ptr<mw::diag::IConversations>)\n",
        );

        assert_eq!(
            normalized.as_str(),
            "+ ChangeHandler(amp::pmr::unique_ptr<ExecutorFactory>, std::shared_ptr<mw::diag::IConversations>)\n"
        );

        let continuation_column = normalized
            .as_str()
            .lines()
            .next()
            .unwrap()
            .find("std::shared_ptr")
            .unwrap()
            + 1;

        assert_eq!(normalized.map_position(1, continuation_column), (2, 36));
    }
}
