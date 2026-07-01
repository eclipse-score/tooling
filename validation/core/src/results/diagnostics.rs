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

//! Ordered diagnostic output for validation reports.

#[derive(Debug, Default)]
pub struct Diagnostics {
    entries: Vec<DiagnosticEntry>,
}

impl Diagnostics {
    pub fn debug(&mut self, message: impl FnOnce() -> String) {
        if log::log_enabled!(log::Level::Debug) {
            self.entries
                .push(DiagnosticEntry::new(DiagnosticLevel::Debug, message()));
        }
    }

    pub fn trace(&mut self, message: impl FnOnce() -> String) {
        if log::log_enabled!(log::Level::Trace) {
            self.entries
                .push(DiagnosticEntry::new(DiagnosticLevel::Trace, message()));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn render(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let mut rendered = self
            .entries
            .iter()
            .map(DiagnosticEntry::render)
            .collect::<Vec<_>>()
            .join("\n");
        rendered.push('\n');
        rendered
    }

    pub(super) fn append(&mut self, incoming: Self) {
        self.entries.extend(incoming.entries);
    }
}

#[derive(Debug)]
struct DiagnosticEntry {
    level: DiagnosticLevel,
    message: String,
}

impl DiagnosticEntry {
    fn new(level: DiagnosticLevel, message: String) -> Self {
        Self { level, message }
    }

    fn render(&self) -> String {
        let mut lines = self.message.trim_end().lines();
        let Some(first_line) = lines.next() else {
            return format!("[{}]:", self.level.as_str());
        };

        let mut rendered = format!("[{}]: {}", self.level.as_str(), first_line);
        for line in lines {
            rendered.push_str("\n  ");
            rendered.push_str(line);
        }
        rendered
    }
}

#[derive(Debug)]
enum DiagnosticLevel {
    Debug,
    Trace,
}

impl DiagnosticLevel {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_entry_render_formats_level_and_indents_multiline_message() {
        let entry = DiagnosticEntry::new(
            DiagnosticLevel::Trace,
            "Design entity details:\nEntity: id=\"unit_1::Foo\"\nmethods=[]\n".to_string(),
        );

        assert_eq!(
            entry.render(),
            "[TRACE]: Design entity details:\n  Entity: id=\"unit_1::Foo\"\n  methods=[]"
        );
    }
}
