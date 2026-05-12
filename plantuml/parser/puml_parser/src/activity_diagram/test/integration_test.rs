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

use activity_parser::{ActivityParserError, PumlActivityParser, RawActivityDiagram};
use parser_core::{BaseParseError, DiagramParser};
use puml_utils::LogLevel;
use test_framework::{run_case, DefaultExpectationChecker, DiagramProcessor};

struct ActivityRunner;

impl DiagramProcessor for ActivityRunner {
    type Output = RawActivityDiagram;
    type Error = ActivityParserError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, RawActivityDiagram>, ActivityParserError> {
        let mut results: HashMap<Rc<PathBuf>, RawActivityDiagram> = HashMap::new();
        let mut parser = PumlActivityParser;

        for puml_path in files {
            let uml_content = fs::read_to_string(&**puml_path).map_err(|error| {
                ActivityParserError::Base(BaseParseError::IoError {
                    path: puml_path.as_ref().to_path_buf(),
                    error: Box::new(error),
                })
            })?;

            let activity_ast = parser.parse_file(puml_path, &uml_content, LogLevel::Error)?;

            results.insert(Rc::clone(puml_path), activity_ast);
        }

        Ok(results)
    }
}

fn run_activity_diagram_parser_case(case_name: &str) {
    run_case(
        "integration_test/activity_diagram/parser",
        case_name,
        ActivityRunner,
        DefaultExpectationChecker,
    );
}

#[test]
fn test_action_simple() {
    run_activity_diagram_parser_case("action_simple");
}

#[test]
fn test_action_separated() {
    run_activity_diagram_parser_case("action_separated");
}

#[test]
fn test_start_stop() {
    run_activity_diagram_parser_case("start_stop");
}

#[test]
fn test_if() {
    run_activity_diagram_parser_case("if");
}

#[test]
fn test_color() {
    run_activity_diagram_parser_case("color");
}

#[test]
fn test_arrows() {
    run_activity_diagram_parser_case("arrows");
}

#[test]
fn test_repeat_while() {
    run_activity_diagram_parser_case("repeat_while");
}

#[test]
fn test_fork() {
    run_activity_diagram_parser_case("fork");
}

#[test]
fn test_swimlane() {
    run_activity_diagram_parser_case("swimlane");
}

#[test]
fn test_while() {
    run_activity_diagram_parser_case("while");
}
