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

use class_parser::{ClassError, ClassUmlFile, PumlClassParser};
use parser_core::{BaseParseError, DiagramParser};
use puml_utils::LogLevel;
use test_framework::{run_case, DefaultExpectationChecker, DiagramProcessor};

struct ClassRunner;
impl DiagramProcessor for ClassRunner {
    type Output = ClassUmlFile;
    type Error = ClassError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, ClassUmlFile>, ClassError> {
        let mut results: HashMap<Rc<PathBuf>, ClassUmlFile> = HashMap::new();
        let mut parser = PumlClassParser;

        for puml_path in files {
            let uml_content = fs::read_to_string(&**puml_path).map_err(|e| {
                ClassError::Base(BaseParseError::IoError {
                    path: puml_path.as_ref().to_path_buf(),
                    error: Box::new(e),
                })
            })?;

            let class_ast = parser.parse_file(puml_path, &uml_content, LogLevel::Error)?;

            results.insert(Rc::clone(puml_path), class_ast);
        }

        Ok(results)
    }
}

// Test entry
fn run_class_diagram_parser_case(case_name: &str) {
    run_case(
        "puml_parser/tests/class_diagram",
        case_name,
        ClassRunner,
        DefaultExpectationChecker,
    );
}

// --------- test for include ---------
#[test]
fn test_alias_class() {
    run_class_diagram_parser_case("alias_class");
}

#[test]
fn test_alias_package() {
    run_class_diagram_parser_case("alias_package");
}

#[test]
fn test_attr_method() {
    run_class_diagram_parser_case("attr_method");
}

#[test]
fn test_class_merge() {
    run_class_diagram_parser_case("class_merge");
}

#[test]
fn test_color() {
    run_class_diagram_parser_case("color");
}

#[test]
fn test_cpp_style() {
    run_class_diagram_parser_case("cpp_style");
}

#[test]
fn test_ctrl_instruct() {
    run_class_diagram_parser_case("ctrl_instruct");
}

#[test]
fn test_empty() {
    run_class_diagram_parser_case("empty");
}

#[test]
fn test_enum() {
    run_class_diagram_parser_case("enum");
}

#[test]
fn test_interface() {
    run_class_diagram_parser_case("interface");
}

#[test]
fn test_namespace_1() {
    run_class_diagram_parser_case("namespace_1");
}

#[test]
fn test_namespace_2() {
    run_class_diagram_parser_case("namespace_2");
}

#[test]
fn test_namespace_3() {
    run_class_diagram_parser_case("namespace_3");
}

#[test]
fn test_negative_pkg_comp() {
    run_class_diagram_parser_case("negative_pkg_comp");
}

#[test]
fn test_one_class() {
    run_class_diagram_parser_case("one_class");
}

#[test]
fn test_only_attribute() {
    run_class_diagram_parser_case("only_attribute");
}

#[test]
fn test_only_method() {
    run_class_diagram_parser_case("only_method");
}

#[test]
fn test_package() {
    run_class_diagram_parser_case("package");
}

#[test]
fn test_param() {
    run_class_diagram_parser_case("param");
}

#[test]
fn test_param_templete() {
    run_class_diagram_parser_case("param_templete");
}

#[test]
fn test_relationship_arrows() {
    run_class_diagram_parser_case("relationship_arrows");
}

#[test]
fn test_relationship_inheritance() {
    run_class_diagram_parser_case("relationship_inheritance");
}

#[test]
fn test_relationship_mix_inher() {
    run_class_diagram_parser_case("relationship_mix_inher");
}

#[test]
fn test_relationship_normal() {
    run_class_diagram_parser_case("relationship_normal");
}

#[test]
fn test_relationship_qualified_id() {
    run_class_diagram_parser_case("relationship_qualified_id");
}

#[test]
fn test_stereotype_definition() {
    run_class_diagram_parser_case("stereotype_definition");
}

#[test]
fn test_stereotype_relationship() {
    run_class_diagram_parser_case("stereotype_relationship");
}

#[test]
fn test_struct() {
    run_class_diagram_parser_case("struct");
}

#[test]
fn test_full_features() {
    run_class_diagram_parser_case("full_features");
}
