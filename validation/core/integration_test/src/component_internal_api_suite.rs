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

const SUITE_DIR: &str = "component_internal_api";

fn run_case_from_cli(
    case_dir: &str,
    component_fbs_paths: &[String],
    internal_api_fbs_paths: &[String],
) -> CliRunResult {
    run_validation_profile(
        &format!("component_internal_api_{case_dir}"),
        "architectural-design",
        serde_json::json!({
            "component_diagrams": component_fbs_paths,
            "internal_api_diagrams": internal_api_fbs_paths,
        }),
    )
}

fn assert_case(case_dir: &str) {
    let expected = load_expected_fixture(SUITE_DIR, case_dir);
    let component_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "component");
    let internal_api_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "internal_api");

    let result = if !component_fbs_paths.is_empty() && !internal_api_fbs_paths.is_empty() {
        run_case_from_cli(case_dir, &component_fbs_paths, &internal_api_fbs_paths)
    } else {
        panic!(
            "missing generated FBS fixtures for {case_dir}: expected component/*.fbs.bin and internal_api/*.fbs.bin",
        );
    };

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn negative_interface_missing_from_internal_api_suite_case() {
    assert_case("negative_interface_missing_from_internal_api");
}

#[test]
fn positive_interface_match_suite_case() {
    assert_case("positive_interface_match");
}
