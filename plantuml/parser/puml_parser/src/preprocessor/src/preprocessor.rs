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
//! PlantUML Preprocessing Module
//! ==============================
//! This module implements the preprocessing logic for PlantUML files, including handling of `!include` directives and procedure/macro expansions. It serves as a crucial step before parsing, ensuring that all files are fully expanded and ready for syntax analysis.
//! The main components include:
//! - `Preprocessor`: The top-level coordinator that manages the entire preprocessing workflow.
//! - `IncludeExpander`: Responsible for resolving and expanding `!include` directives, ensuring that all included content is inlined correctly.
//! - `ProcedureExpander`: Handles the expansion of procedures and macros defined within the PlantUML files, replacing calls with their corresponding definitions.

use log::{debug, warn};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use thiserror::Error;

use crate::{
    IncludeExpandError, IncludeExpander, ProcedureExpandError, ProcedureExpander, SourceLine,
};
use puml_utils::LogLevel;

// ----------------------
// Type Aliases
// ----------------------
type PreprocessedFiles = HashMap<Rc<PathBuf>, String>;
type FileList = HashSet<Rc<PathBuf>>;

/// A preprocessed diagram.
#[derive(Debug)]
pub struct Preprocessed {
    pub text: String,
    /// Source line of each line of `text`; `None` when it is not known.
    pub origins: Option<Vec<SourceLine>>,
}

/// Top-level preprocessing errors. `Display` already holds the cause, so it is
/// not also the `source()`.
#[derive(Debug, Error)]
pub enum PreprocessError {
    #[error("include preprocess failed: {0}")]
    IncludeFailed(IncludeExpandError),

    #[error("procedure preprocess failed: {0}")]
    ProcedureFailed(ProcedureExpandError),
}

impl PreprocessError {
    /// The message, with `name_of` naming the files of a missing include.
    pub fn describe(&self, name_of: &dyn Fn(&Path) -> String) -> String {
        match self {
            Self::IncludeFailed(error) => {
                format!("include preprocess failed: {}", error.describe(name_of))
            }
            other => other.to_string(),
        }
    }
}

impl From<IncludeExpandError> for PreprocessError {
    fn from(error: IncludeExpandError) -> Self {
        Self::IncludeFailed(error)
    }
}

impl From<ProcedureExpandError> for PreprocessError {
    fn from(error: ProcedureExpandError) -> Self {
        Self::ProcedureFailed(error)
    }
}

#[derive(Default)]
pub struct Preprocessor {
    include_expander: IncludeExpander,
    procedure_expander: ProcedureExpander,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            include_expander: IncludeExpander::new(),
            procedure_expander: ProcedureExpander::new(),
        }
    }

    /// Top-level coordinator: preprocess all given PlantUML files.
    ///
    /// # Arguments
    /// - `file_list`: set of all PlantUML files to preprocess.
    ///
    /// # Returns
    /// - A map of each file to its fully expanded PlantUML text.
    ///
    /// # Errors
    /// Returns `PreprocessError`
    pub fn preprocess(
        &mut self,
        file_list: &FileList,
        log_level: LogLevel,
    ) -> Result<PreprocessedFiles, PreprocessError> {
        let mut preprocessed_files = HashMap::new();

        for file in file_list {
            debug!("Preprocess file: {}", file.display());

            let include_expanded = self.include_expander.expand(file, file_list)?;
            let procedure_expanded =
                self.procedure_expander
                    .expand(file, &include_expanded, log_level)?;

            preprocessed_files.insert(Rc::clone(file), procedure_expanded);
        }

        Ok(preprocessed_files)
    }

    /// Preprocesses one diagram. Its `!include` directives may name any file
    /// of `include_files` besides diagrams.
    ///
    /// # Errors
    /// Returns `PreprocessError`
    pub fn preprocess_with_includes(
        &mut self,
        file: &Rc<PathBuf>,
        include_files: &FileList,
        log_level: LogLevel,
    ) -> Result<Preprocessed, PreprocessError> {
        debug!("Preprocess file: {}", file.display());

        let mut available = include_files.clone();
        available.insert(Rc::clone(file));

        let expanded = self.include_expander.expand_mapped(file, &available)?;
        let text = self
            .procedure_expander
            .expand(file, &expanded.text, log_level)?;

        // Expanding procedures rewrites the lines.
        if text == expanded.text {
            return Ok(Preprocessed {
                text,
                origins: Some(expanded.origins),
            });
        }
        let includes_used = expanded
            .origins
            .iter()
            .zip(1u32..)
            .any(|(origin, line)| origin.file != *file || origin.line != line);
        if includes_used {
            warn!(
                "{} defines `!procedure`s and includes other files; its source locations name lines of the expanded text",
                file.display()
            );
        }
        let origins = None;
        Ok(Preprocessed { text, origins })
    }
}
