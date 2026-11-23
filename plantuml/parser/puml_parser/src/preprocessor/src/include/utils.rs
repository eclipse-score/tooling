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

/// Whether `line` opens (`@startuml`) or closes (`@enduml`) a diagram.
pub fn is_start_end_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("@startuml") || t.starts_with("@enduml")
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
    fn start_end_lines_are_recognized() {
        assert!(is_start_end_line("@startuml name\n"));
        assert!(is_start_end_line("  @enduml"));
        assert!(!is_start_end_line("!@startuml"));
        assert!(!is_start_end_line("a -> b"));
        assert!(!is_start_end_line(""));
    }
}
