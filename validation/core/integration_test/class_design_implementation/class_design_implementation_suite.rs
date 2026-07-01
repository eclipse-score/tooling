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

const SUITE_DIR: &str = "class_design_implementation";

fn run_case_from_cli(
    case_dir: &str,
    design_class_fbs_paths: &[String],
    implementation_class_fbs_paths: &[String],
) -> CliRunResult {
    run_validation_profile(
        &format!("class_design_implementation_{case_dir}"),
        "unit",
        serde_json::json!({
            "design_classes": design_class_fbs_paths,
            "implementation_classes": implementation_class_fbs_paths,
        }),
    )
}

fn assert_case(case_dir: &str) {
    let expected = load_expected_yaml_fixture(SUITE_DIR, case_dir);
    let design_class_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "unit_design_class");
    let implementation_class_fbs_paths =
        collect_case_fbs_files(SUITE_DIR, case_dir, "unit_implementation_class");

    let result = if !design_class_fbs_paths.is_empty() && !implementation_class_fbs_paths.is_empty()
    {
        run_case_from_cli(
            case_dir,
            &design_class_fbs_paths,
            &implementation_class_fbs_paths,
        )
    } else {
        panic!(
            "missing generated FBS fixtures for {case_dir}: expected at least one unit_design_class/*.fbs.bin and unit_implementation_class/*.fbs.bin",
        );
    };

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn positive_class_features() {
    assert_case("positive_class_features");
}

#[test]
fn positive_method_features() {
    assert_case("positive_method_features");
}

// #[test]
// fn positive_method_template_pack_features() {
//     assert_case("positive_method_template_pack_features");
// }

#[test]
fn positive_relationship_features() {
    assert_case("positive_relationship_features");
}

#[test]
fn positive_variable_features() {
    assert_case("positive_variable_features");
}

#[test]
fn negative_class_missing() {
    assert_case("negative_class_missing");
}

#[test]
fn negative_class_member_missing() {
    assert_case("negative_class_member_missing");
}

#[test]
fn negative_entity_type_mismatch() {
    assert_case("negative_entity_type_mismatch");
}

#[test]
fn negative_variable_mismatch() {
    assert_case("negative_variable_mismatch");
}

#[test]
fn negative_variable_static_mismatch() {
    assert_case("negative_variable_static_mismatch");
}

#[test]
fn negative_variable_visibility_mismatch() {
    assert_case("negative_variable_visibility_mismatch");
}

#[test]
fn negative_method_missing() {
    assert_case("negative_method_missing");
}

#[test]
fn negative_method_mismatch() {
    assert_case("negative_method_mismatch");
}

#[test]
fn negative_method_modifier_mismatch() {
    assert_case("negative_method_modifier_mismatch");
}

#[test]
fn negative_method_parameter_count_mismatch() {
    assert_case("negative_method_parameter_count_mismatch");
}

#[test]
fn negative_method_parameter_name_mismatch() {
    assert_case("negative_method_parameter_name_mismatch");
}

#[test]
fn negative_method_parameter_type_mismatch() {
    assert_case("negative_method_parameter_type_mismatch");
}

#[test]
fn negative_method_parameter_variadic_mismatch() {
    assert_case("negative_method_parameter_variadic_mismatch");
}

#[test]
fn negative_method_visibility_mismatch() {
    assert_case("negative_method_visibility_mismatch");
}

#[test]
fn negative_relationship_missing() {
    assert_case("negative_relationship_missing");
}

#[test]
fn negative_relationship_type_mismatch() {
    assert_case("negative_relationship_type_mismatch");
}

#[test]
fn negative_type_alias_missing() {
    assert_case("negative_type_alias_missing");
}

#[test]
fn negative_type_alias_mismatch() {
    assert_case("negative_type_alias_mismatch");
}

#[test]
fn negative_enum_literal_missing() {
    assert_case("negative_enum_literal_missing");
}

#[test]
fn negative_enum_literal_mismatch() {
    assert_case("negative_enum_literal_mismatch");
}

#[test]
fn negative_template_parameter_mismatch() {
    assert_case("negative_template_parameter_mismatch");
}
