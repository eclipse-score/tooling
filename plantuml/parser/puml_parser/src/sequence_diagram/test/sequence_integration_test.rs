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
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use parser_core::{BaseParseError, DiagramParser};
use puml_utils::LogLevel;
use sequence_parser::{PumlSequenceParser, SeqPumlDocument, SequenceError};
use test_framework::{run_case, DefaultExpectationChecker, DiagramProcessor};

const TEST_MODULE: &str = "puml_parser/tests/sequence_diagram";

struct SequenceRunner;

impl DiagramProcessor for SequenceRunner {
    type Output = SeqPumlDocument;
    type Error = SequenceError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, SeqPumlDocument>, SequenceError> {
        let mut results = HashMap::new();
        let mut parser = PumlSequenceParser;

        for puml_path in files {
            let content = fs::read_to_string(&**puml_path).map_err(|e| {
                SequenceError::Base(BaseParseError::IoError {
                    path: puml_path.as_ref().to_path_buf(),
                    error: Box::new(e),
                })
            })?;

            let sequence_ast = parser.parse_file(puml_path, &content, LogLevel::Error)?;

            results.insert(Rc::clone(puml_path), sequence_ast);
        }

        Ok(results)
    }
}

fn run_sequence_diagram_parser_case(case_name: &str) {
    run_case(
        TEST_MODULE,
        case_name,
        SequenceRunner,
        DefaultExpectationChecker,
    );
}

#[test]
fn test_participant_identifiers() {
    run_sequence_diagram_parser_case("participant_identifiers");
}
