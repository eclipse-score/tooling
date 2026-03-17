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
use std::collections::HashMap;
use std::path::Path;

use class_diagram::ClassResolverError;
use puml_parser::{
    BaseParseError, ClassError, IncludeExpandError, IncludeParseError, PreprocessError,
    ProcedureExpandError, ProcedureParseError,
};
use puml_resolver::ComponentResolverError;

#[derive(Debug)]
pub struct ProjectedError {
    pub kind: String,
    pub fields: HashMap<String, String>,
    pub source: Option<Box<ProjectedError>>,
}

impl ProjectedError {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            fields: HashMap::new(),
            source: None,
        }
    }

    pub fn with_field(mut self, k: &str, v: impl Into<String>) -> Self {
        self.fields.insert(k.to_string(), v.into());
        self
    }

    pub fn with_source(mut self, src: ProjectedError) -> Self {
        self.source = Some(Box::new(src));
        self
    }
}

pub trait ErrorView {
    fn project(&self, base_dir: &Path) -> ProjectedError;
}

fn relative_path(path: &Path, dir: &Path) -> String {
    path.strip_prefix(dir)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

impl<Rule> ErrorView for BaseParseError<Rule> {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        match self {
            BaseParseError::IoError { path, .. } => {
                ProjectedError::new("IoError").with_field("path", relative_path(path, base_dir))
            }

            BaseParseError::SyntaxError {
                file,
                line,
                column,
                message,
                source_line,
                cause: _,
            } => ProjectedError::new("SyntaxError")
                .with_field("file", relative_path(file, base_dir))
                .with_field("line", line.to_string())
                .with_field("column", column.to_string())
                .with_field("message", message.clone())
                .with_field("source_line", source_line.clone()),
        }
    }
}

impl ErrorView for IncludeParseError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        match self {
            IncludeParseError::Base(e) => e.project(base_dir),

            IncludeParseError::InvalidTextLine { line, file } => {
                ProjectedError::new("InvalidTextLine")
                    .with_field("line", line.clone())
                    .with_field("file", relative_path(file, base_dir))
            }
        }
    }
}

impl ErrorView for IncludeExpandError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        match self {
            IncludeExpandError::FileNotFound { file } => ProjectedError::new("FileNotFound")
                .with_field("file", relative_path(file, base_dir)),

            IncludeExpandError::ParseFailed { file, error } => ProjectedError::new("ParseFailed")
                .with_field("file", relative_path(file, base_dir))
                .with_source(error.project(base_dir)),

            IncludeExpandError::CycleInclude { chain } => {
                let chain_str = chain
                    .iter()
                    .map(|p| relative_path(p, base_dir))
                    .collect::<Vec<_>>()
                    .join(" -> ");

                ProjectedError::new("CycleInclude").with_field("chain", chain_str)
            }

            IncludeExpandError::IncludeOnceViolated { file, conflict } => {
                ProjectedError::new("IncludeOnceViolated")
                    .with_field("file", relative_path(file, base_dir))
                    .with_field("conflict", relative_path(conflict, base_dir))
            }

            IncludeExpandError::UnknownSub { suffix, file } => ProjectedError::new("UnknownSub")
                .with_field("file", relative_path(file, base_dir))
                .with_field("suffix", suffix.clone()),
        }
    }
}

impl ErrorView for ProcedureParseError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        match self {
            ProcedureParseError::Base(e) => e.project(base_dir),
        }
    }
}

impl ErrorView for ProcedureExpandError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        match self {
            ProcedureExpandError::ParseFailed { file, error } => ProjectedError::new("ParseFailed")
                .with_field("file", relative_path(file, base_dir))
                .with_source(error.project(base_dir)),

            ProcedureExpandError::MacroNotDefined(name) => {
                ProjectedError::new("MacroNotDefined").with_field("name", name.clone())
            }

            ProcedureExpandError::ArgumentMismatch {
                name,
                expected,
                actual,
            } => ProjectedError::new("ArgumentMismatch")
                .with_field("name", name.clone())
                .with_field("expected", expected.to_string())
                .with_field("actual", actual.to_string()),

            ProcedureExpandError::RecursiveMacro { chain, name } => {
                let chain_str = chain.join(" -> ");
                ProjectedError::new("RecursiveMacro")
                    .with_field("chain", chain_str)
                    .with_field("name", name.clone())
            }

            ProcedureExpandError::MaxDepthExceeded => ProjectedError::new("MaxDepthExceeded"),
        }
    }
}

impl ErrorView for PreprocessError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        match self {
            PreprocessError::IncludeFailed(e) => e.project(base_dir),
            PreprocessError::ProcedureFailed(e) => e.project(base_dir),
        }
    }
}

impl ErrorView for ClassError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        match self {
            ClassError::Base(e) => e.project(base_dir),
        }
    }
}

impl ErrorView for ComponentResolverError {
    fn project(&self, _base_dir: &Path) -> ProjectedError {
        match self {
            ComponentResolverError::UnresolvedReference { reference } => {
                ProjectedError::new("UnresolvedReference")
                    .with_field("reference", reference.clone())
            }

            ComponentResolverError::DuplicateComponent { component_id } => {
                ProjectedError::new("DuplicateComponent")
                    .with_field("component_id", component_id.clone())
            }

            ComponentResolverError::UnknownComponentType { component_type } => {
                ProjectedError::new("UnknownComponentType")
                    .with_field("component_type", component_type.clone())
            }
        }
    }
}

impl ErrorView for ClassResolverError {
    fn project(&self, _base_dir: &Path) -> ProjectedError {
        match self {
            ClassResolverError::UnresolvedReference { reference } => {
                ProjectedError::new("UnresolvedReference")
                    .with_field("reference", reference.clone())
            }

            ClassResolverError::DuplicateEntity { entity_id } => {
                ProjectedError::new("DuplicateEntity").with_field("entity_id", entity_id.clone())
            }

            ClassResolverError::UnknownEntityType { entity_type } => {
                ProjectedError::new("UnknownEntityType")
                    .with_field("entity_type", entity_type.clone())
            }

            ClassResolverError::InvalidRelationship { from, to, reason } => {
                ProjectedError::new("InvalidRelationship")
                    .with_field("from", from.clone())
                    .with_field("to", to.clone())
                    .with_field("reason", reason.clone())
            }

            ClassResolverError::CircularInheritance { cycle } => {
                ProjectedError::new("CircularInheritance").with_field("cycle", cycle.clone())
            }

            ClassResolverError::InvalidVisibility { modifier } => {
                ProjectedError::new("InvalidVisibility").with_field("modifier", modifier.clone())
            }

            ClassResolverError::ParseError { message } => {
                ProjectedError::new("ParseError").with_field("message", message.clone())
            }
        }
    }
}
