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

//! Shared error message formatting helpers for validation results.

use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Design,
    Class,
    Member,
    Implementation,
    Naming,
    Interface,
    Method,
    Coverage,
}

impl ErrorCategory {
    fn as_tag(self) -> &'static str {
        match self {
            Self::Design => "Design",
            Self::Class => "Class",
            Self::Member => "Member",
            Self::Implementation => "Implementation",
            Self::Naming => "Naming",
            Self::Interface => "Interface",
            Self::Method => "Method",
            Self::Coverage => "Coverage",
        }
    }
}

fn uppercase_first_char(value: impl Into<String>) -> String {
    let mut value = value.into();

    if let Some((index, first_char)) = value.char_indices().next() {
        let upper = first_char.to_uppercase().to_string();
        value.replace_range(index..index + first_char.len_utf8(), &upper);
    }

    value
}

fn normalize_sentence_case(value: impl Into<String>) -> String {
    let mut value = uppercase_first_char(value);

    if !value.ends_with('.') {
        value.push('.');
    }

    value
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ErrorContent {
    Field(String, String),
    Section(ErrorSection),
    Table(Vec<Vec<String>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorSection {
    title: String,
    content: Vec<ErrorContent>,
}

impl ErrorSection {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: Vec::new(),
        }
    }

    pub fn field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.content
            .push(ErrorContent::Field(uppercase_first_char(key), value.into()));
        self
    }

    pub fn section(mut self, section: ErrorSection) -> Self {
        self.content.push(ErrorContent::Section(section));
        self
    }

    pub fn table<const N: usize>(
        mut self,
        headers: [&str; N],
        rows: impl IntoIterator<Item = [String; N]>,
    ) -> Self {
        self.content.push(table_content(headers, rows));
        self
    }
}

fn table_content<const N: usize>(
    headers: [&str; N],
    rows: impl IntoIterator<Item = [String; N]>,
) -> ErrorContent {
    ErrorContent::Table(
        std::iter::once(headers.map(str::to_string).to_vec())
            .chain(rows.into_iter().map(|row| row.to_vec()))
            .collect(),
    )
}

fn render_content(message: &mut String, content: &[ErrorContent], indent: usize) {
    let label_width = content
        .iter()
        .filter_map(|item| match item {
            ErrorContent::Field(key, _) => Some(key.len()),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let mut previous_group = false;
    for (index, item) in content.iter().enumerate() {
        let group = !matches!(item, ErrorContent::Field(..));
        if index > 0 && (group || previous_group) {
            message.push('\n');
        }
        match item {
            ErrorContent::Field(key, value) => {
                write!(message, "\n{:indent$}{key:label_width$} : {value}", "")
                    .expect("writing to String cannot fail");
            }
            ErrorContent::Section(section) => {
                write!(message, "\n{:indent$}{}", "", section.title)
                    .expect("writing to String cannot fail");
                render_content(message, &section.content, indent + 2);
            }
            ErrorContent::Table(rows) => {
                let widths = (0..rows[0].len())
                    .map(|column| {
                        rows.iter()
                            .map(|row| row[column].chars().count())
                            .max()
                            .unwrap_or(0)
                    })
                    .collect::<Vec<_>>();
                for row in rows {
                    write!(message, "\n{:indent$}", "").expect("writing to String cannot fail");
                    for (column, value) in row.iter().enumerate() {
                        if column > 0 {
                            message.push_str(" | ");
                        }
                        if column + 1 == row.len() {
                            message.push_str(value);
                        } else {
                            write!(message, "{value:width$}", width = widths[column])
                                .expect("writing to String cannot fail");
                        }
                    }
                }
            }
        }
        previous_group = group;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorBuilder {
    category: ErrorCategory,
    title: Option<String>,
    content: Vec<ErrorContent>,
    fix: Option<String>,
}

impl ErrorBuilder {
    pub fn new(category: ErrorCategory) -> Self {
        Self {
            category,
            title: None,
            content: Vec::new(),
            fix: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(normalize_sentence_case(title));
        self
    }

    pub fn field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.content
            .push(ErrorContent::Field(uppercase_first_char(key), value.into()));
        self
    }

    pub fn section(mut self, section: ErrorSection) -> Self {
        self.content.push(ErrorContent::Section(section));
        self
    }

    pub fn table<const N: usize>(
        mut self,
        headers: [&str; N],
        rows: impl IntoIterator<Item = [String; N]>,
    ) -> Self {
        self.content.push(table_content(headers, rows));
        self
    }

    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(normalize_sentence_case(fix));
        self
    }

    pub fn suggest(
        self,
        original_name: &str,
        suggestion_subject: Option<&str>,
        suggested_name: &str,
    ) -> Self {
        let suggestion_text = match suggestion_subject {
            Some(suggestion_subject) => {
                format!("Did you mean {suggestion_subject} \"{suggested_name}\"?")
            }
            None => format!("Did you mean \"{suggested_name}\"?"),
        };

        self.field(
            format!("suggestion for \"{original_name}\""),
            suggestion_text,
        )
    }

    pub fn build(mut self) -> String {
        let title = self.title.unwrap_or_default();
        let mut message = String::new();
        write!(&mut message, "[{}] {title}", self.category.as_tag())
            .expect("writing to String cannot fail");

        if let Some(fix) = self.fix {
            self.content
                .push(ErrorContent::Field("Fix".to_string(), fix));
        }
        render_content(&mut message, &self.content, 2);
        message
    }
}

#[cfg(test)]
mod tests {
    use super::{ErrorBuilder, ErrorCategory};

    #[test]
    fn suggest_formats_message_with_subject() {
        let message = ErrorBuilder::new(ErrorCategory::Naming)
            .suggest("Srvce", Some("class"), "Service")
            .build();

        assert_eq!(
            message,
            "[Naming] \n  Suggestion for \"Srvce\" : Did you mean class \"Service\"?"
        );
    }

    #[test]
    fn suggest_formats_message_without_subject() {
        let message = ErrorBuilder::new(ErrorCategory::Naming)
            .suggest("Srvce", None, "Service")
            .build();

        assert_eq!(
            message,
            "[Naming] \n  Suggestion for \"Srvce\" : Did you mean \"Service\"?"
        );
    }
}
