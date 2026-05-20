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

const SUITE_DIR: &str = "component_sequence";

fn run_case_from_cli(
    case_dir: &str,
    component_fbs_paths: &[String],
    sequence_fbs_paths: &[String],
) -> CliRunResult {
    let mut cli_args = vec!["--component-fbs".to_string()];
    cli_args.extend(component_fbs_paths.iter().cloned());
    cli_args.push("--sequence-fbs".to_string());
    cli_args.extend(sequence_fbs_paths.iter().cloned());

    run_validation_cli(&format!("component_sequence_{case_dir}"), &cli_args)
}

fn assert_case(case_dir: &str) {
    let expected = load_expected_fixture(SUITE_DIR, case_dir);
    let component_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "component");
    let sequence_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "sequence");

    let result = if !component_fbs_paths.is_empty() && !sequence_fbs_paths.is_empty() {
        run_case_from_cli(case_dir, &component_fbs_paths, &sequence_fbs_paths)
    } else {
        panic!(
            "missing generated FBS fixtures for {case_dir}: expected at least one component/*.fbs.bin and sequence/*.fbs.bin",
        );
    };

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn positive_exact_match_suite_case() {
    assert_case("positive_exact_match");
}

#[test]
fn negative_missing_participant_suite_case() {
    assert_case("negative_missing_participant");
}

#[test]
fn negative_orphan_participant_suite_case() {
    assert_case("negative_orphan_participant");
}

#[test]
fn negative_mixed_mismatch_suite_case() {
    assert_case("negative_mixed_mismatch");
}
