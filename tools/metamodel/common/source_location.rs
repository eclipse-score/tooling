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

use std::path::Path;
use std::{fmt, rc::Rc};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct SourceLocation {
    pub file: Rc<str>,
    pub line: u32,
}

impl SourceLocation {
    pub fn new(file: impl Into<Rc<str>>, line: u32) -> Self {
        Self {
            file: file.into(),
            line,
        }
    }

    fn display_file(&self) -> String {
        let file_path = Path::new(self.file.as_ref());

        if file_path.is_absolute() {
            if let Ok(cwd) = std::env::current_dir() {
                if let Ok(relative) = file_path.strip_prefix(&cwd) {
                    return normalize_source_path(relative.to_string_lossy().as_ref());
                }
            }
        }

        normalize_source_path(self.file.as_ref())
    }

    pub fn display(&self) -> (String, u32) {
        (self.display_file(), self.line)
    }
}

fn normalize_source_path(path: &str) -> String {
    path.strip_prefix("./").unwrap_or(path).to_string()
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.display_file(), self.line)
    }
}

#[cfg(test)]
mod tests {
    use super::SourceLocation;

    #[test]
    fn display_uses_workspace_relative_path_for_absolute_file() {
        let cwd = std::env::current_dir().expect("cwd must exist");
        let abs = cwd.join("tools/metamodel/common/source_location.rs");
        let location = SourceLocation::new(abs.to_string_lossy().to_string(), 7);

        assert_eq!(
            location.to_string(),
            "tools/metamodel/common/source_location.rs:7"
        );
    }

    #[test]
    fn display_keeps_relative_path() {
        let location = SourceLocation::new("relative/path.puml", 3);
        assert_eq!(location.to_string(), "relative/path.puml:3");
    }

    #[test]
    fn display_strips_dot_slash_prefix() {
        let location = SourceLocation::new("./relative/path.puml", 9);
        assert_eq!(location.to_string(), "relative/path.puml:9");
    }
}
