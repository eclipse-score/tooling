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
//! Include-expanded text that remembers where each line was written.

use std::path::PathBuf;
use std::rc::Rc;

use crate::utils::is_start_end_line;

/// A line of a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLine {
    pub file: Rc<PathBuf>,
    /// 1-based.
    pub line: u32,
}

/// Include-expanded text and the source line of every line of it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Expanded {
    pub text: String,
    /// One entry per line of `text`.
    pub origins: Vec<SourceLine>,
}

impl Expanded {
    /// `text` unchanged, every line attributed to its own line of `file`.
    pub fn verbatim(file: &Rc<PathBuf>, text: &str) -> Self {
        Self {
            text: text.to_string(),
            origins: (1..=text.split_inclusive('\n').count() as u32)
                .map(|line| SourceLine {
                    file: Rc::clone(file),
                    line,
                })
                .collect(),
        }
    }

    /// Appends one line written at `line` of `file`.
    pub fn push_line(&mut self, text: &str, file: &Rc<PathBuf>, line: u32) {
        self.text.push_str(text);
        if !text.ends_with('\n') {
            self.text.push('\n');
        }
        self.origins.push(SourceLine {
            file: Rc::clone(file),
            line,
        });
    }

    pub fn append(&mut self, other: Expanded) {
        self.text.push_str(&other.text);
        self.origins.extend(other.origins);
    }

    /// Drops the `@startuml` / `@enduml` lines and normalizes line endings.
    pub fn without_start_end(self) -> Self {
        let mut kept = Self::default();
        for (line, origin) in self.text.split_inclusive('\n').zip(self.origins) {
            if is_start_end_line(line) {
                continue;
            }
            kept.text.push_str(line.trim_end_matches(['\r', '\n']));
            kept.text.push('\n');
            kept.origins.push(origin);
        }
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> Rc<PathBuf> {
        Rc::new(PathBuf::from(name))
    }

    #[test]
    fn verbatim_numbers_every_line_of_the_file() {
        let a = file("a.puml");
        let expanded = Expanded::verbatim(&a, "x\n\ny");

        assert_eq!(expanded.text, "x\n\ny");
        let lines: Vec<u32> = expanded.origins.iter().map(|o| o.line).collect();
        assert_eq!(lines, [1, 2, 3]);
    }

    #[test]
    fn push_line_terminates_the_line() {
        let a = file("a.puml");
        let mut expanded = Expanded::default();
        expanded.push_line("x", &a, 4);
        expanded.push_line("y\n", &a, 9);

        assert_eq!(expanded.text, "x\ny\n");
        assert_eq!(expanded.origins.len(), 2);
    }

    #[test]
    fn without_start_end_keeps_the_origin_of_the_remaining_lines() {
        let a = file("a.puml");
        let expanded = Expanded::verbatim(&a, "@startuml\nx\r\n  @enduml\ny\n").without_start_end();

        assert_eq!(expanded.text, "x\ny\n");
        let lines: Vec<u32> = expanded.origins.iter().map(|o| o.line).collect();
        assert_eq!(lines, [2, 4]);
    }

    #[test]
    fn append_keeps_each_origin() {
        let (a, b) = (file("a.puml"), file("b.puml"));
        let mut expanded = Expanded::verbatim(&a, "x\n");
        expanded.append(Expanded::verbatim(&b, "y\n"));

        assert_eq!(expanded.text, "x\ny\n");
        assert_eq!(expanded.origins[1].file, b);
    }
}
