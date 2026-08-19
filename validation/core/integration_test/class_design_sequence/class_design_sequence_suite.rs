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

const SUITE_DIR: &str = "class_design_sequence";

fn run_case_from_cli(
    case_dir: &str,
    design_class_fbs_paths: &[String],
    sequence_fbs_paths: &[String],
) -> CliRunResult {
    run_validation_profile(
        &format!("class_design_sequence_{case_dir}"),
        "unit",
        serde_json::json!({
            "design_classes": design_class_fbs_paths,
            "sequence_diagrams": sequence_fbs_paths,
        }),
    )
}

fn assert_case(case_dir: &str) {
    let expected = load_expected_yaml_fixture(SUITE_DIR, case_dir);
    let design_class_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "unit_design_class");
    let sequence_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "unit_design_sequence");

    let result = if !design_class_fbs_paths.is_empty() && !sequence_fbs_paths.is_empty() {
        run_case_from_cli(case_dir, &design_class_fbs_paths, &sequence_fbs_paths)
    } else {
        panic!(
            "missing generated FBS fixtures for {case_dir}: expected at least one unit_design_class/*.fbs.bin and unit_design_sequence/*.fbs.bin",
        );
    };

    let result = normalize_yaml_result(result);

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn positive_participant_abstract_base_method_match_suite_case() {
    assert_case("positive_participant_abstract_base_method_match");
}

#[test]
fn positive_participant_multilevel_inherited_method_match_suite_case() {
    assert_case("positive_participant_multilevel_inherited_method_match");
}

#[test]
fn positive_participant_alias_display_name_class_name_match_suite_case() {
    assert_case("positive_participant_alias_display_name_class_name_match");
}

#[test]
fn positive_participant_alias_display_name_namespace_match_suite_case() {
    assert_case("positive_participant_alias_display_name_namespace_match");
}

#[test]
fn positive_participant_namespace_callee_method_match_suite_case() {
    assert_case("positive_participant_namespace_callee_method_match");
}

#[test]
fn positive_participant_short_name_namespace_match_suite_case() {
    assert_case("positive_participant_short_name_namespace_match");
}

#[test]
fn positive_participant_special_display_leading_colon_short_name_match_suite_case() {
    assert_case("positive_participant_special_display_leading_colon_short_name_match");
}

#[test]
fn positive_participant_special_display_qualified_type_match_suite_case() {
    assert_case("positive_participant_special_display_qualified_type_match");
}

#[test]
fn positive_participant_special_display_short_type_match_suite_case() {
    assert_case("positive_participant_special_display_short_type_match");
}

#[test]
fn positive_participant_special_display_encoded_newline_match_suite_case() {
    assert_case("positive_participant_special_display_encoded_newline_match");
}

#[test]
fn negative_participant_missing_suite_case() {
    assert_case("negative_participant_missing");
}

#[test]
fn negative_participant_ambiguous_short_name_suite_case() {
    assert_case("negative_participant_ambiguous_short_name");
}

#[test]
fn negative_participant_method_missing_suite_case() {
    assert_case("negative_participant_method_missing");
}

#[test]
fn negative_participant_method_missing_with_suggestion_suite_case() {
    assert_case("negative_participant_method_missing_with_suggestion");
}

#[test]
fn negative_participant_private_inherited_method_suite_case() {
    assert_case("negative_participant_private_inherited_method");
}

#[test]
fn negative_participant_missing_with_suggestion_suite_case() {
    assert_case("negative_participant_missing_with_suggestion");
}

#[test]
fn negative_participant_special_display_multiple_colons_suite_case() {
    assert_case("negative_participant_special_display_multiple_colons");
}

#[test]
fn negative_participant_special_display_empty_suffix_suite_case() {
    assert_case("negative_participant_special_display_empty_suffix");
}
