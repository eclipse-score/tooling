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
    assert_cli_result, collect_case_fbs_files, load_expected_yaml_fixture, run_validation_profile,
    CliRunResult,
};

const SUITE_DIR: &str = "component_public_api";

fn run_case_from_cli(
    case_dir: &str,
    component_fbs_paths: &[String],
    public_api_fbs_paths: &[String],
) -> CliRunResult {
    run_validation_profile(
        &format!("component_public_api_{case_dir}"),
        "architectural-design",
        serde_json::json!({
            "component_diagrams": component_fbs_paths,
            "public_api_diagrams": public_api_fbs_paths,
        }),
    )
}

fn assert_case(case_dir: &str) {
    let expected = load_expected_yaml_fixture(SUITE_DIR, case_dir);
    let component_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "component");
    let public_api_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "public_api");

    let result = if !component_fbs_paths.is_empty() && !public_api_fbs_paths.is_empty() {
        run_case_from_cli(case_dir, &component_fbs_paths, &public_api_fbs_paths)
    } else {
        panic!(
            "missing generated FBS fixtures for {case_dir}: expected component/*.fbs.bin and public_api/*.fbs.bin",
        );
    };

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn positive_public_api_declared_and_related_to_seooc_suite_case() {
    assert_case("positive_public_api_declared_and_related_to_seooc");
}

#[test]
fn positive_overview_seooc_relation_merge_suite_case() {
    assert_case("positive_overview_seooc_relation_merge");
}

#[test]
fn negative_public_apis_missing_from_public_api_diagram_suite_case() {
    assert_case("negative_public_apis_missing_from_public_api_diagram");
}

#[test]
fn negative_public_apis_missing_from_public_api_diagram_with_suggestions_suite_case() {
    assert_case("negative_public_apis_missing_from_public_api_diagram_with_suggestions");
}

#[test]
fn negative_public_api_declared_as_class_is_treated_as_missing_suite_case() {
    assert_case("negative_public_api_declared_as_class_is_treated_as_missing");
}

#[test]
fn negative_public_api_case_mismatch_suite_case() {
    assert_case("negative_public_api_case_mismatch");
}

#[test]
fn negative_public_api_without_seooc_relationship_suite_case() {
    assert_case("negative_public_api_without_seooc_relationship");
}
