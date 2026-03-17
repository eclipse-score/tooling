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
use std::fmt::Write;
use std::path::{Component, Path, PathBuf};

/// Normalizes paths resolving `.` and `..`.
pub fn normalize_path(path: &Path) -> PathBuf {
    path.components().fold(PathBuf::new(), |mut acc, comp| {
        match comp {
            Component::ParentDir => {
                acc.pop();
            }
            Component::CurDir => {}
            _ => acc.push(comp.as_os_str()),
        }
        acc
    })
}

/// Strips `@startuml` and `@enduml` from text.
pub fn strip_start_end(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let t = line.trim_start();
        if !t.starts_with("@startuml") && !t.starts_with("@enduml") {
            writeln!(out, "{}", line).unwrap();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn normalize_path_test() {
        let path = Path::new("/a/b/../c/./d");
        let normalized = normalize_path(path);
        assert_eq!(normalized, Path::new("/a/c/d"));
    }

    #[test]
    fn normalize_path_root_and_parent() {
        // /../ should stay at root
        let path = Path::new("/../a");
        let normalized = normalize_path(path);
        assert_eq!(normalized, Path::new("/a"));
    }

    #[test]
    fn normalize_path_empty_and_cur() {
        // empty path -> empty
        let path = Path::new("");
        let normalized = normalize_path(path);
        assert_eq!(normalized, Path::new(""));

        // ./././ -> empty
        let path = Path::new("./././");
        let normalized = normalize_path(path);
        assert_eq!(normalized, Path::new(""));
    }

    #[test]
    fn strip_start_end_test() {
        let text = "@startuml\nline1\nline2\n@enduml\n";
        let stripped = strip_start_end(text);
        assert_eq!(stripped, "line1\nline2\n");
    }

    #[test]
    fn strip_start_end_no_tags() {
        let text = "line1\nline2\n";
        let stripped = strip_start_end(text);
        assert_eq!(stripped, "line1\nline2\n");
    }

    #[test]
    fn strip_start_end_similar_tags() {
        let text = "!@startuml\nline1\nline2\n!@enduml\n";
        let stripped = strip_start_end(text);
        assert_eq!(stripped, "!@startuml\nline1\nline2\n!@enduml\n");
    }

    #[test]
    fn strip_start_end_empty_string() {
        let text = "";
        let stripped = strip_start_end(text);
        assert_eq!(stripped, "");
    }
}
