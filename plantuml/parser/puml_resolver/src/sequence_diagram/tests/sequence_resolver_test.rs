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

use parser_core::DiagramParser;
use puml_utils::LogLevel;
use resolver_traits::DiagramResolver;
use sequence_logic::SequenceTree;
use sequence_parser::PumlSequenceParser;
use sequence_resolver::{SequenceResolver, SequenceResolverError};
use test_framework::{run_case, DefaultExpectationChecker, DiagramProcessor};

struct SequenceResolverRunner;

impl DiagramProcessor for SequenceResolverRunner {
    type Output = SequenceTree;
    type Error = SequenceResolverError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, SequenceTree>, SequenceResolverError> {
        let mut results = HashMap::new();
        let mut parser = PumlSequenceParser;
        let mut resolver = SequenceResolver;

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

fn run_sequence_resolver_case(case_name: &str) {
    run_case(
        "integration_test/sequence_diagram",
        case_name,
        SequenceResolverRunner,
        DefaultExpectationChecker,
    );
}

#[test]
fn test_simple_sequence() {
    run_sequence_resolver_case("simple_sequence");
}

#[test]
fn test_participant_identifier_examples() {
    run_sequence_resolver_case("participant_identifier_examples");
}

#[test]
fn test_official_lost_found_endpoint_variants() {
    run_sequence_resolver_case("official_lost_found_endpoint_variants");
}

#[test]
fn test_invalid_undeclared_participant() {
    run_sequence_resolver_case("invalid_undeclared_participant");
}
