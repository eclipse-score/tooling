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
    assert_cli_result, collect_case_fbs_files, load_expected_yaml_fixture, normalize_yaml_result,
    run_validation_profile, CliRunResult,
};

const SUITE_DIR: &str = "component_sequence";

fn run_case_from_cli(
    case_dir: &str,
    component_fbs_paths: &[String],
    sequence_fbs_paths: &[String],
) -> CliRunResult {
    run_validation_profile(
        &format!("component_sequence_{case_dir}"),
        "architectural-design",
        serde_json::json!({
            "component_diagrams": component_fbs_paths,
            "sequence_diagrams": sequence_fbs_paths,
        }),
    )
}

fn assert_case(case_dir: &str) {
    let expected = load_expected_yaml_fixture(SUITE_DIR, case_dir);
    let component_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "component");
    let sequence_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "sequence");

    let result = if !component_fbs_paths.is_empty() && !sequence_fbs_paths.is_empty() {
        run_case_from_cli(case_dir, &component_fbs_paths, &sequence_fbs_paths)
    } else {
        panic!(
            "missing generated FBS fixtures for {case_dir}: expected at least one component/*.fbs.bin and sequence/*.fbs.bin",
        );
    };

    let result = normalize_yaml_result(result);

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn positive_matching_component_and_sequence_suite_case() {
    assert_case("positive_matching_component_and_sequence");
}

#[test]
fn positive_overview_preserves_unit_bindings_suite_case() {
    assert_case("positive_overview_preserves_unit_bindings");
}

#[test]
fn negative_component_unit_missing_from_sequence_suite_case() {
    assert_case("negative_component_unit_missing_from_sequence");
}

#[test]
fn positive_external_caller_call_allowed_suite_case() {
    assert_case("positive_external_caller_call_allowed");
}

#[test]
fn positive_external_callee_return_allowed_suite_case() {
    assert_case("positive_external_callee_return_allowed");
}

#[test]
fn negative_sequence_call_without_shared_interface_suite_case() {
    assert_case("negative_sequence_call_without_shared_interface");
}

#[test]
fn negative_shared_interface_without_sequence_call_suite_case() {
    assert_case("negative_shared_interface_without_sequence_call");
}
