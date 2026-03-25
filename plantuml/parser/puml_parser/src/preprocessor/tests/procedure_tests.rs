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
mod preprocess_runner;

use preprocess_runner::PreprocessRunner;
use test_framework::{run_case, DefaultExpectationChecker};

// Procedure Preprocess Test Entry
fn run_procedure_preprocess_case(case_name: &str) {
    run_case(
        "puml_parser/tests/preprocessor/procedure",
        case_name,
        PreprocessRunner,
        DefaultExpectationChecker,
    );
}

// --------- test for procedure ---------
#[test]
fn test_simple_macro() {
    run_procedure_preprocess_case("simple_macro");
}

#[test]
fn test_simple_template() {
    run_procedure_preprocess_case("simple_template");
}

#[test]
fn test_use_include() {
    run_procedure_preprocess_case("use_include");
}

#[test]
fn test_mix_call() {
    run_procedure_preprocess_case("mix_call");
}

#[test]
fn test_fta_metamodel() {
    run_procedure_preprocess_case("fta_metamodel");
}

#[test]
fn test_macro_not_define() {
    run_procedure_preprocess_case("macro_not_define");
}

#[test]
fn test_args_not_match() {
    run_procedure_preprocess_case("args_not_match");
}

#[test]
fn test_recursive_macro() {
    run_procedure_preprocess_case("recursive_macro");
}

#[test]
fn test_noise_item() {
    run_procedure_preprocess_case("noise_item");
}

#[test]
fn test_empty() {
    run_procedure_preprocess_case("empty");
}
