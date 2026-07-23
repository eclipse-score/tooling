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

use puml_parser::{
    ActivityParserError, BaseParseError, ClassError, ComponentError, IncludeExpandError,
    IncludeParseError, PreprocessError, ProcedureExpandError, ProcedureParseError, SequenceError,
};
use puml_resolver::{
    ActivityResolverError, ClassPumlResolverError, ComponentResolverError, SequenceResolverError,
};

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

            ProcedureExpandError::UnknownVariable { name } => {
                ProjectedError::new("UnknownVariable").with_field("name", name.clone())
            }

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
            ClassError::UnexpectedUsingAttribute => {
                let _ = base_dir;
                ProjectedError::new("UnexpectedUsingAttribute")
            }
            ClassError::UnexpectedClassMember(rule) => {
                let _ = base_dir;
                ProjectedError::new("UnexpectedClassMember").with_field("rule", rule.clone())
            }
        }
    }
}

impl ErrorView for ActivityParserError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        match self {
            ActivityParserError::Base(e) => e.project(base_dir),
            ActivityParserError::InvalidStatement(message) => {
                let _ = base_dir;
                ProjectedError::new("InvalidStatement").with_field("message", message.to_string())
            }
        }
    }
}

impl ErrorView for ComponentError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        match self {
            ComponentError::Base(e) => e.project(base_dir),
            ComponentError::InvalidStatement(message) => {
                let _ = base_dir;
                ProjectedError::new("InvalidStatement").with_field("message", message.to_string())
            }
        }
    }
}

impl ErrorView for SequenceError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        match self {
            SequenceError::Base(e) => e.project(base_dir),
            SequenceError::InvalidStatement(message) => {
                let _ = base_dir;
                ProjectedError::new("InvalidStatement").with_field("message", message.to_string())
            }
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

            ComponentResolverError::DuplicateElement { element_id } => {
                ProjectedError::new("DuplicateComponent")
                    .with_field("component_id", element_id.clone())
            }

            ComponentResolverError::UnknownElementType { element_type } => {
                ProjectedError::new("UnknownComponentType")
                    .with_field("component_type", element_type.clone())
            }

            ComponentResolverError::InvalidRelationship { from, to, reason } => {
                ProjectedError::new("InvalidRelationship")
                    .with_field("from", from.clone())
                    .with_field("to", to.clone())
                    .with_field("reason", reason.clone())
            }

            ComponentResolverError::AmbiguousReference {
                reference,
                candidates,
            } => ProjectedError::new("AmbiguousReference")
                .with_field("reference", reference.clone())
                .with_field("candidates", candidates.join(", ")),
        }
    }
}

impl ErrorView for ActivityResolverError {
    fn project(&self, _base_dir: &Path) -> ProjectedError {
        match self {
            ActivityResolverError::UnexpectedEndOfInput {
                context,
                line,
                column,
            } => ProjectedError::new("UnexpectedEndOfInput")
                .with_field("context", context.to_string())
                .with_field("line", line.to_string())
                .with_field("column", column.to_string()),
            ActivityResolverError::UnexpectedStatement {
                context,
                statement,
                line,
                column,
            } => ProjectedError::new("UnexpectedStatement")
                .with_field("context", context.to_string())
                .with_field("statement", statement.to_string())
                .with_field("line", line.to_string())
                .with_field("column", column.to_string()),
            ActivityResolverError::UnsupportedStatement {
                statement,
                line,
                column,
            } => ProjectedError::new("UnsupportedStatement")
                .with_field("statement", statement.to_string())
                .with_field("line", line.to_string())
                .with_field("column", column.to_string()),
        }
    }
}

impl ErrorView for ClassPumlResolverError {
    fn project(&self, _base_dir: &Path) -> ProjectedError {
        match self {
            ClassPumlResolverError::UnresolvedReference { reference } => {
                ProjectedError::new("UnresolvedReference")
                    .with_field("reference", reference.clone())
            }

            ClassPumlResolverError::DuplicateEntity { entity_id } => {
                ProjectedError::new("DuplicateEntity").with_field("entity_id", entity_id.clone())
            }

            ClassPumlResolverError::UnknownEntityType { entity_type } => {
                ProjectedError::new("UnknownEntityType")
                    .with_field("entity_type", entity_type.clone())
            }

            ClassPumlResolverError::InvalidRelationship { from, to, reason } => {
                ProjectedError::new("InvalidRelationship")
                    .with_field("from", from.clone())
                    .with_field("to", to.clone())
                    .with_field("reason", reason.clone())
            }

            ClassPumlResolverError::CircularInheritance { cycle } => {
                ProjectedError::new("CircularInheritance").with_field("cycle", cycle.clone())
            }

            ClassPumlResolverError::InvalidVisibility { modifier } => {
                ProjectedError::new("InvalidVisibility").with_field("modifier", modifier.clone())
            }

            ClassPumlResolverError::ParseError { message } => {
                ProjectedError::new("ParseError").with_field("message", message.clone())
            }
        }
    }
}

impl ErrorView for SequenceResolverError {
    fn project(&self, base_dir: &Path) -> ProjectedError {
        let _ = base_dir;

        match self {
            SequenceResolverError::UndeclaredParticipant { name, role } => {
                ProjectedError::new("UndeclaredParticipant")
                    .with_field("name", name.clone())
                    .with_field("role", (*role).to_string())
            }
        }
    }
}
