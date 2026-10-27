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
};

const SUITE_DIR: &str = "sequence_design";

fn assert_case(case_dir: &str) {
    let expected = load_expected_yaml_fixture(SUITE_DIR, case_dir);
    let sequence_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "sequence");
    assert!(
        !sequence_fbs_paths.is_empty(),
        "missing generated FBS fixtures for {case_dir}: expected sequence/*.fbs.bin",
    );

    let result = run_validation_profile(
        &format!("sequence_design_{case_dir}"),
        "architectural-design",
        serde_json::json!({
            "component_diagrams": collect_case_fbs_files(SUITE_DIR, case_dir, "component"),
            "sequence_diagrams": sequence_fbs_paths,
            "internal_api_diagrams": collect_case_fbs_files(SUITE_DIR, case_dir, "internal_api"),
            "public_api_diagrams": collect_case_fbs_files(SUITE_DIR, case_dir, "public_api"),
        }),
    );

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn negative_actor_call_other_units_interface_suite_case() {
    assert_case("negative_actor_call_other_units_interface");
}

#[test]
fn negative_actor_call_undeclared_method_suite_case() {
    assert_case("negative_actor_call_undeclared_method");
}

#[test]
fn negative_actor_names_unit_suite_case() {
    assert_case("negative_actor_names_unit");
}

#[test]
fn negative_callback_not_in_public_api_suite_case() {
    assert_case("negative_callback_not_in_public_api");
}

#[test]
fn negative_conflicting_environment_declarations_suite_case() {
    assert_case("negative_conflicting_environment_declarations");
}

#[test]
fn negative_container_call_does_not_cover_unit_pair_suite_case() {
    assert_case("negative_container_call_does_not_cover_unit_pair");
}

#[test]
fn negative_gate_message_undeclared_method_suite_case() {
    assert_case("negative_gate_message_undeclared_method");
}

#[test]
fn negative_legacy_external_endpoint_suite_case() {
    assert_case("negative_legacy_external_endpoint");
}

#[test]
fn positive_actor_boundary_suite_case() {
    assert_case("positive_actor_boundary");
}

#[test]
fn positive_container_lifeline_suite_case() {
    assert_case("positive_container_lifeline");
}

#[test]
fn positive_create_message_suite_case() {
    assert_case("positive_create_message");
}

#[test]
fn positive_create_statement_suite_case() {
    assert_case("positive_create_statement");
}

#[test]
fn positive_external_participant_suite_case() {
    assert_case("positive_external_participant");
}

#[test]
fn positive_gate_message_declared_method_suite_case() {
    assert_case("positive_gate_message_declared_method");
}
