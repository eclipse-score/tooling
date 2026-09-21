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

//! Suite covering `ComponentDiagramArchitecture`'s entity model: duplicate/
//! casefold-shadow detection and the multi-file `static` merge.

use test_framework::{
    assert_cli_result, collect_case_fbs_files, load_expected_yaml_fixture, run_validation_profile,
    CliRunResult,
};

const SUITE_DIR: &str = "component_model";

fn run_case_from_cli(case_dir: &str, component_fbs_paths: &[String]) -> CliRunResult {
    run_validation_profile(
        &format!("component_model_{case_dir}"),
        "architectural-design",
        serde_json::json!({
            "component_diagrams": component_fbs_paths,
        }),
    )
}

fn assert_case(case_dir: &str) {
    let expected = load_expected_yaml_fixture(SUITE_DIR, case_dir);
    let component_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "component");

    let result = if !component_fbs_paths.is_empty() {
        run_case_from_cli(case_dir, &component_fbs_paths)
    } else {
        panic!("missing generated FBS fixtures for {case_dir}: expected component/*.fbs.bin");
    };

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn negative_duplicate_unit_alias_casefolded_suite_case() {
    assert_case("negative_duplicate_unit_alias_casefolded");
}

#[test]
fn negative_duplicate_component_alias_casefolded_suite_case() {
    assert_case("negative_duplicate_component_alias_casefolded");
}

#[test]
fn negative_duplicate_interface_alias_casefolded_suite_case() {
    assert_case("negative_duplicate_interface_alias_casefolded");
}

#[test]
fn negative_duplicate_dependable_element_alias_casefolded_suite_case() {
    assert_case("negative_duplicate_dependable_element_alias_casefolded");
}

#[test]
fn positive_overview_subset_suite_case() {
    assert_case("positive_overview_subset");
}

#[test]
fn negative_conflicting_stereotype_suite_case() {
    assert_case("negative_conflicting_stereotype");
}

#[test]
fn negative_children_split_across_files_suite_case() {
    assert_case("negative_children_split_across_files");
}

#[test]
fn negative_conflicting_element_type_suite_case() {
    assert_case("negative_conflicting_element_type");
}

#[test]
fn positive_overview_partial_subset_suite_case() {
    assert_case("positive_overview_partial_subset");
}

#[test]
fn positive_three_file_merge_suite_case() {
    assert_case("positive_three_file_merge");
}

#[test]
fn positive_overview_nested_public_interface_suite_case() {
    assert_case("positive_overview_nested_public_interface");
}
