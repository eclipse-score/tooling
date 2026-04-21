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

use component_parser::PumlComponentParser;
use component_resolver::{ComponentResolver, ComponentResolverError, LogicComponent};
use parser_core::DiagramParser;
use puml_utils::LogLevel;
use resolver_traits::DiagramResolver;
use test_framework::{run_case, DefaultExpectationChecker, DiagramProcessor};

// ===== Component Resolver adapter DiagramProcessor =====
struct ComponentResolverRunner;
impl DiagramProcessor for ComponentResolverRunner {
    type Output = HashMap<String, LogicComponent>;
    type Error = ComponentResolverError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, HashMap<String, LogicComponent>>, ComponentResolverError> {
        let mut results = HashMap::new();
        let mut parser = PumlComponentParser;
        let mut resolver = ComponentResolver::new();

        for path in files {
            let puml_file = fs::read_to_string(&**path).expect("Failed to read test file");
            let parsed_ast = parser
                .parse_file(path, &puml_file, LogLevel::Error)
                .expect("Failed to parse test file");
            let logic_ast = resolver.visit_document(&parsed_ast)?;

            results.insert(Rc::clone(path), logic_ast);
        }
        Ok(results)
    }
}

// Test entry
fn run_component_resolver_case(case_name: &str) {
    run_case(
        "integration_test/component_diagram",
        case_name,
        ComponentResolverRunner,
        DefaultExpectationChecker,
    );
}

#[test]
fn test_relation_simple_name() {
    run_component_resolver_case("relation_simple_name");
}

#[test]
fn test_relation_fqn() {
    run_component_resolver_case("relation_fqn");
}

#[test]
fn test_relation_relative_name() {
    run_component_resolver_case("relation_relative_name");
}

#[test]
fn test_relation_simple_name_alias() {
    run_component_resolver_case("relation_simple_name_alias");
}

#[test]
fn test_relation_absolute_fqn() {
    run_component_resolver_case("relation_absolute_fqn");
}

#[test]
fn test_invalid_unresolved_reference() {
    run_component_resolver_case("invalid_unresolved_reference");
}

#[test]
fn test_invalid_duplicate_component() {
    run_component_resolver_case("invalid_duplicate_component");
}
