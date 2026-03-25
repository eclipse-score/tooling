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
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;

use preprocessor::{PreprocessError, Preprocessor};
use puml_utils::LogLevel;
use test_framework::DiagramProcessor;

// ===== Preprocess adapter DiagramProcessor =====
pub struct PreprocessRunner;
impl DiagramProcessor for PreprocessRunner {
    type Output = String;
    type Error = PreprocessError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, std::string::String>, PreprocessError> {
        Preprocessor::new().preprocess(files, LogLevel::Error)
    }
}
