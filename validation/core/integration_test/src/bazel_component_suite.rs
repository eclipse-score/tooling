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
    assert_cli_result, case_file_path, collect_case_fbs_files, load_expected_fixture,
    run_validation_cli, CliRunResult,
};

const SUITE_DIR: &str = "bazel_component";

fn run_case_from_cli(
    case_dir: &str,
    architecture_json_path: &str,
    component_fbs_paths: &[String],
) -> CliRunResult {
    let mut cli_args = vec![
        "--architecture-json".to_string(),
        case_file_path(architecture_json_path).display().to_string(),
        "--component-fbs".to_string(),
    ];
    cli_args.extend(component_fbs_paths.iter().cloned());

    run_validation_cli(&format!("bazel_component_{case_dir}"), &cli_args)
}

fn assert_case(case_dir: &str) {
    let expected = load_expected_fixture(SUITE_DIR, case_dir);
    let component_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "component");

    let result = if !component_fbs_paths.is_empty() {
        run_case_from_cli(
            case_dir,
            &format!("validation/core/integration_test/{SUITE_DIR}/{case_dir}/architecture.json"),
            &component_fbs_paths,
        )
    } else {
        panic!("missing generated FBS fixtures for {case_dir}: expected at least one component/*.fbs.bin");
    };

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn positive_component_suite_case() {
    assert_case("positive_component");
}

#[test]
fn positive_case_insensitive_suite_case() {
    assert_case("positive_case_insensitive");
}

#[test]
fn negative_missing_unit_suite_case() {
    assert_case("negative_missing_unit");
}

#[test]
fn negative_extra_unit_suite_case() {
    assert_case("negative_extra_unit");
}

#[test]
fn negative_missing_component_suite_case() {
    assert_case("negative_missing_component");
}

#[test]
fn negative_extra_component_suite_case() {
    assert_case("negative_extra_component");
}

#[test]
fn negative_wrong_stereotype_suite_case() {
    assert_case("negative_wrong_stereotype");
}
