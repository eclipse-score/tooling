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
fn test_lost_found_endpoint_resolution() {
    run_sequence_resolver_case("lost_found_endpoint_resolution");
}

#[test]
fn test_implicit_participant_from_message_endpoint() {
    run_sequence_resolver_case("implicit_participant_from_message_endpoint");
}

#[test]
fn test_sequence_interaction_node() {
    run_sequence_resolver_case("sequence_interaction_node");
}

#[test]
fn test_sequence_arrow_direction() {
    run_sequence_resolver_case("sequence_arrow_direction");
}

#[test]
fn test_sequence_reference_node() {
    run_sequence_resolver_case("sequence_reference_node");
}

#[test]
fn test_sequence_lifecycle_nodes() {
    run_sequence_resolver_case("sequence_lifecycle_nodes");
}

#[test]
fn test_sequence_group_container() {
    run_sequence_resolver_case("sequence_group_container");
}

#[test]
fn test_combined_lifecycle_suffix() {
    run_sequence_resolver_case("combined_lifecycle_suffix");
}

#[test]
fn test_invalid_message_direction() {
    run_sequence_resolver_case("invalid_message_direction");
}

#[test]
fn test_invalid_unterminated_group() {
    run_sequence_resolver_case("invalid_unterminated_group");
}

#[test]
fn test_invalid_else_in_opt() {
    run_sequence_resolver_case("invalid_else_in_opt");
}

#[test]
fn test_invalid_mismatched_group_end() {
    run_sequence_resolver_case("invalid_mismatched_group_end");
}

#[test]
fn test_sequence_branch_node() {
    run_sequence_resolver_case("sequence_branch_node");
}

#[test]
fn test_sequence_loop_node() {
    run_sequence_resolver_case("sequence_loop_node");
}

#[test]
fn test_sequence_parallel_node() {
    run_sequence_resolver_case("sequence_parallel_node");
}

#[test]
fn test_sequence_early_exit_node() {
    run_sequence_resolver_case("sequence_early_exit_node");
}

#[test]
fn test_invalid_destroyed_participant_use_is_rejected() {
    run_sequence_resolver_case("invalid_destroyed_participant_use");
}

#[test]
fn test_recreate_destroyed_participant_is_allowed() {
    run_sequence_resolver_case("recreate_destroyed_participant");
}
