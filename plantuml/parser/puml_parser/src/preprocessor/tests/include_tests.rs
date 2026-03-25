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

// Include Preprocessor Test Entry
fn run_include_preprocess_case(case_name: &str) {
    run_case(
        "puml_parser/tests/preprocessor/include",
        case_name,
        PreprocessRunner,
        DefaultExpectationChecker,
    );
}

// --------- test for include ---------
#[test]
fn test_simple_include() {
    run_include_preprocess_case("simple_include");
}

#[test]
fn test_include_repeat() {
    run_include_preprocess_case("repeat_include");
}

#[test]
fn test_invalid_cycle_include() {
    run_include_preprocess_case("invalid_cycle_include");
}

#[test]
fn test_invalid_include_path() {
    run_include_preprocess_case("invalid_include_path");
}

// --------- test for incluesub ---------
#[test]
fn test_simple_includesub() {
    run_include_preprocess_case("simple_includesub");
}

#[test]
fn test_include_with_invalid_suffix() {
    run_include_preprocess_case("invalid_suffix_for_include");
}

#[test]
fn test_includesub_with_serveral_subblock() {
    run_include_preprocess_case("several_subblock");
}

#[test]
fn test_includesub_with_invalid_suffix() {
    run_include_preprocess_case("invalid_suffix_for_includesub");
}

#[test]
fn test_invalid_nested_subblock() {
    run_include_preprocess_case("invalid_nested_subblock");
}

#[test]
fn test_invalid_include_unknow_sub() {
    run_include_preprocess_case("invalid_include_unknow_sub");
}

// --------- test for include_once ---------
#[test]
fn test_simple_include_once() {
    run_include_preprocess_case("simple_include_once");
}

#[test]
fn test_repeat_include_once_raise_error() {
    run_include_preprocess_case("invalid_repeat_include_once");
}

// --------- test for include_many ---------
#[test]
fn test_simple_include_many() {
    run_include_preprocess_case("simple_include_many");
}

#[test]
fn test_combine_include_and_include_many_with_diff_sort() {
    run_include_preprocess_case("combine_include_and_include_many");
}

// --------- test for complex include ---------
#[test]
fn test_complex_include() {
    run_include_preprocess_case("complex_include");
}
