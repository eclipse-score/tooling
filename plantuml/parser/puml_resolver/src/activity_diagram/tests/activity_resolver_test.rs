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

use activity_parser::PumlActivityParser;
use activity_resolver::{ActivityDiagram, ActivityResolver, ActivityResolverError};
use parser_core::DiagramParser;
use puml_utils::LogLevel;
use resolver_traits::DiagramResolver;
use test_framework::{run_case, DefaultExpectationChecker, DiagramProcessor};

struct ActivityResolverRunner;

impl DiagramProcessor for ActivityResolverRunner {
    type Output = ActivityDiagram;
    type Error = ActivityResolverError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, ActivityDiagram>, ActivityResolverError> {
        let mut results = HashMap::new();
        let mut parser = PumlActivityParser;
        let mut resolver = ActivityResolver::new();

        for path in files {
            let puml_file = fs::read_to_string(&**path).expect("Failed to read test file");
            let parsed_ast = parser
                .parse_file(path, &puml_file, LogLevel::Error)
                .expect("Failed to parse test file");
            let logic_ast = resolver.resolve(&parsed_ast)?;

            results.insert(Rc::clone(path), logic_ast);
        }

        Ok(results)
    }
}

fn run_activity_resolver_case(case_name: &str) {
    run_case(
        "integration_test/activity_diagram/resolver",
        case_name,
        ActivityResolverRunner,
        DefaultExpectationChecker,
    );
}

#[test]
fn test_activity_resolver_smoke() {
    run_activity_resolver_case("activity_it");
}
