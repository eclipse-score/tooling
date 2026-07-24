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
pub struct ErrorBuilder {
    category: ErrorCategory,
    title: Option<String>,
    fields: Vec<(String, String)>,
    fix: Option<String>,
}

impl ErrorBuilder {
    pub fn new(category: ErrorCategory) -> Self {
        Self {
            category,
            title: None,
            fields: Vec::new(),
            fix: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(normalize_sentence_case(title));
        self
    }

    pub fn field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push((uppercase_first_char(key), value.into()));
        self
    }

    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(normalize_sentence_case(fix));
        self
    }

    pub fn build(self) -> String {
        let label_width = self
            .fields
            .iter()
            .map(|(key, _)| key.len())
            .chain(self.fix.iter().map(|_| "Fix".len()))
            .max()
            .unwrap_or(0);

        let title = self.title.unwrap_or_default();
        let mut message = String::new();
        write!(&mut message, "[{}] {title}", self.category.as_tag())
            .expect("writing to String cannot fail");

        for (key, value) in self.fields {
            write!(&mut message, "\n  {key:label_width$} : {value}")
                .expect("writing to String cannot fail");
        }

        if let Some(fix) = self.fix {
            write!(&mut message, "\n  {:label_width$} : {fix}", "Fix")
                .expect("writing to String cannot fail");
        }

        message
    }
}
