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

use component_parser::{CompPumlDocument, ComponentError, PumlComponentParser};
use parser_core::{BaseParseError, DiagramParser};
use puml_utils::LogLevel;
use test_framework::{run_case, DefaultExpectationChecker, DiagramProcessor};

const TEST_MODULE: &str = "integration_test/component_diagram/plantuml";

struct ComponentRunner;

impl DiagramProcessor for ComponentRunner {
    type Output = CompPumlDocument;
    type Error = ComponentError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, CompPumlDocument>, ComponentError> {
        let mut results: HashMap<Rc<PathBuf>, CompPumlDocument> = HashMap::new();
        let mut parser = PumlComponentParser;

        for puml_path in files {
            let content = fs::read_to_string(&**puml_path).map_err(|e| {
                ComponentError::Base(BaseParseError::IoError {
                    path: puml_path.as_ref().to_path_buf(),
                    error: Box::new(e),
                })
            })?;

            let component_ast = parser.parse_file(puml_path, &content, LogLevel::Error)?;

            results.insert(Rc::clone(puml_path), component_ast);
        }

        Ok(results)
    }
}

fn run_component_diagram_parser_case(case_name: &str) {
    run_case(
        TEST_MODULE,
        case_name,
        ComponentRunner,
        DefaultExpectationChecker,
    );
}

#[test]
fn test_basic_example() {
    run_component_diagram_parser_case("basic_example");
}

#[test]
fn test_changing_arrows_direction() {
    run_component_diagram_parser_case("changing_arrows_direction");
}

#[test]
fn test_changing_arrows_direction2() {
    run_component_diagram_parser_case("changing_arrows_direction2");
}

#[test]
fn test_changing_arrows_direction3() {
    run_component_diagram_parser_case("changing_arrows_direction3");
}

#[test]
fn test_component() {
    run_component_diagram_parser_case("component");
}

#[test]
fn test_grouping_components() {
    run_component_diagram_parser_case("grouping_components");
}

#[test]
fn test_hide_or_remove_unlinked_component() {
    run_component_diagram_parser_case("hide_or_remove_unlinked_component");
}

#[test]
fn test_individual_colors() {
    run_component_diagram_parser_case("individual_colors");
}

#[test]
fn test_interfaces() {
    run_component_diagram_parser_case("interfaces");
}

#[test]
fn test_long_description() {
    run_component_diagram_parser_case("long_description");
}

#[test]
fn test_use_uml2_notation() {
    run_component_diagram_parser_case("use_uml2_notation");
}
