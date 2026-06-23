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
    assert_cli_result, collect_case_fbs_files, load_expected_fixture, run_validation_profile,
    CliRunResult,
};

const SUITE_DIR: &str = "component_class";

fn run_case_from_cli(
    case_dir: &str,
    component_fbs_paths: &[String],
    class_fbs_paths: &[String],
) -> CliRunResult {
    run_validation_profile(
        &format!("component_class_{case_dir}"),
        "architectural-design",
        serde_json::json!({
            "component_diagrams": component_fbs_paths,
            "class_diagrams": class_fbs_paths,
        }),
    )
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
#[ignore = "component-class validation profile is pending"]
fn positive_exact_match_suite_case() {
    assert_case("positive_exact_match");
}

#[test]
#[ignore = "component-class validation profile is pending"]
fn positive_suffix_suite_case() {
    assert_case("positive_suffix_match");
}

#[test]
#[ignore = "component-class validation profile is pending"]
fn positive_multi_class_files_suite_case() {
    assert_case("positive_multi_class_files");
}

#[test]
#[ignore = "component-class validation profile is pending"]
fn negative_missing_namespace_coverage_suite_case() {
    assert_case("negative_missing_namespace_coverage");
}

#[test]
#[ignore = "component-class validation profile is pending"]
fn negative_boundary_mismatch_suite_case() {
    assert_case("negative_boundary_mismatch");
}

#[test]
#[ignore = "component-class validation profile is pending"]
fn negative_case_sensitive_mismatch_suite_case() {
    assert_case("negative_case_sensitive_mismatch");
}

#[test]
#[ignore = "component-class validation profile is pending"]
fn negative_multi_class_files_with_mismatch_suite_case() {
    assert_case("negative_multi_class_files_with_mismatch");
}
