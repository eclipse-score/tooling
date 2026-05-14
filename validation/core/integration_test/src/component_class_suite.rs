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

use test_framework::{
    assert_cli_result, collect_case_fbs_files, load_expected_fixture, run_validation_cli,
    CliRunResult,
};

const SUITE_DIR: &str = "component_class";

fn run_case_from_cli(
    case_dir: &str,
    component_fbs_paths: &[String],
    class_fbs_paths: &[String],
) -> CliRunResult {
    let mut cli_args = vec!["--component-fbs".to_string()];
    cli_args.extend(component_fbs_paths.iter().cloned());
    cli_args.push("--class-fbs".to_string());
    cli_args.extend(class_fbs_paths.iter().cloned());

    run_validation_cli(&format!("component_class_{case_dir}"), &cli_args)
}

fn assert_case(case_dir: &str) {
    let expected = load_expected_fixture(SUITE_DIR, case_dir);
    let component_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "component");
    let class_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "class");

    let result = if !component_fbs_paths.is_empty() && !class_fbs_paths.is_empty() {
        run_case_from_cli(case_dir, &component_fbs_paths, &class_fbs_paths)
    } else {
        panic!(
            "missing generated FBS fixtures for {case_dir}: expected at least one component_diagram*.fbs.bin and class_diagram*.fbs.bin",
        );
    };

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn positive_exact_match_suite_case() {
    assert_case("positive_exact_match");
}

#[test]
fn positive_suffix_suite_case() {
    assert_case("positive_suffix_match");
}

#[test]
fn positive_multi_class_files_suite_case() {
    assert_case("positive_multi_class_files");
}

#[test]
fn negative_missing_namespace_coverage_suite_case() {
    assert_case("negative_missing_namespace_coverage");
}

#[test]
fn negative_boundary_mismatch_suite_case() {
    assert_case("negative_boundary_mismatch");
}

#[test]
fn negative_case_sensitive_mismatch_suite_case() {
    assert_case("negative_case_sensitive_mismatch");
}

#[test]
fn negative_multi_class_files_with_mismatch_suite_case() {
    assert_case("negative_multi_class_files_with_mismatch");
}
