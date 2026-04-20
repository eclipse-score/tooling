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

use class_diagram::{ClassDiagram, ClassResolverError};
use class_parser::PumlClassParser;
use class_resolver::ClassResolver;

use parser_core::DiagramParser;
use puml_utils::LogLevel;
use resolver_traits::DiagramResolver;
use test_framework::{run_case, DefaultExpectationChecker, DiagramProcessor};

// ===== Class Resolver adapter DiagramProcessor =====
struct ClassResolverRunner;
impl DiagramProcessor for ClassResolverRunner {
    type Output = ClassDiagram;
    type Error = ClassResolverError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, ClassDiagram>, ClassResolverError> {
        let mut results = HashMap::new();
        let mut parser = PumlClassParser;
        let mut resolver = ClassResolver::new();

        for path in files {
            let puml_file =
                fs::read_to_string(&**path).expect("Class Resolver: Failed to read test file");
            let parsed_ast = parser
                .parse_file(path, &puml_file, LogLevel::Error)
                .expect("Class Resolver: Failed to parse test file");
            let logic_ast = resolver.visit_document(&parsed_ast)?;
            results.insert(Rc::clone(path), logic_ast);
        }

        Ok(results)
    }
}

// Test entry
fn run_class_resolver_case(case_name: &str) {
    run_case(
        "integration_test/class_diagram",
        case_name,
        ClassResolverRunner,
        DefaultExpectationChecker,
    );
}

#[test]
fn test_class_positive() {
    run_class_resolver_case("class_diagram_positive");
}

#[test]
fn test_class_negative() {
    run_class_resolver_case("class_diagram_negative");
}

#[test]
fn test_unsupported_syntax() {
    run_class_resolver_case("class_diagram_unsupported_syntax");
}
