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

use log::debug;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use thiserror::Error;

use crate::{IncludeExpandError, IncludeExpander, ProcedureExpandError, ProcedureExpander};
use puml_utils::LogLevel;

// ----------------------
// Type Aliases
// ----------------------
type PreprocessedFiles = HashMap<Rc<PathBuf>, String>;
type FileList = HashSet<Rc<PathBuf>>;

/// Top-level preprocessing errors.
#[derive(Debug, Error)]
pub enum PreprocessError {
    #[error("include preprocess failed")]
    IncludeFailed(#[from] IncludeExpandError),

    #[error("procedure preprocess failed")]
    ProcedureFailed(#[from] ProcedureExpandError),
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
}
