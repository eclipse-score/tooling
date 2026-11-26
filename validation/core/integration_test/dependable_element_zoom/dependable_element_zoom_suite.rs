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

//! Suite covering zoom validation and diagram links of the
//! `dependable-element` profile across architectural designs.

use serde_json::{json, Value};
use test_framework::{
    assert_cli_result, collect_case_fbs_files, load_expected_yaml_fixture, normalize_yaml_result,
    run_dependable_element_links, run_dependable_element_links_only, run_validation_profile,
};

const SUITE_DIR: &str = "dependable_element_zoom";

fn case_source(case_dir: &str, file: &str) -> String {
    format!("validation/core/integration_test/{SUITE_DIR}/{case_dir}/{file}")
}

fn inputs(case_dir: &str) -> Value {
    json!({
        "architecture": "",
        "component_diagrams": collect_case_fbs_files(SUITE_DIR, case_dir, "component"),
        "sequence_diagrams": collect_case_fbs_files(SUITE_DIR, case_dir, "sequence"),
    })
}

fn assert_case(case_dir: &str) -> Value {
    let expected = load_expected_yaml_fixture(SUITE_DIR, case_dir);
    let (result, links) =
        run_dependable_element_links(&format!("{SUITE_DIR}_{case_dir}"), inputs(case_dir));
    assert_cli_result(case_dir, &expected, &normalize_yaml_result(result));
    links
}

fn call_links(links: &Value) -> &Vec<Value> {
    links["call_links"].as_array().expect("call_links array")
}

fn same_level_inputs(included: &[&str]) -> Value {
    let mut result = inputs("same_level");
    let diagrams = result["sequence_diagrams"]
        .as_array_mut()
        .expect("sequence diagrams array");
    diagrams.retain(|diagram| {
        included
            .iter()
            .any(|name| diagram.as_str().unwrap().rsplit('/').next() == Some(*name))
    });
    assert_eq!(diagrams.len(), included.len(), "included {included:?}");
    result
}

#[test]
fn positive_cross_design_suite_case() {
    let case_dir = "positive_cross_design";
    let links = assert_case(case_dir);

    assert_eq!(
        call_links(&links),
        &vec![json!({
            "source": case_source(case_dir, "overview.puml"),
            "line": 20,
            "method": "Get",
            "targets": [case_source(case_dir, "detail.puml")],
        })],
    );
}

#[test]
fn negative_cross_design_exit_missing_suite_case() {
    let case_dir = "negative_cross_design_exit_missing";
    let links = assert_case(case_dir);

    // The failing refinement gets no link; element links are unaffected.
    assert!(call_links(&links).is_empty(), "links={links}");
    assert!(!links["element_links"].as_array().unwrap().is_empty());
}

#[test]
fn links_only_writes_links_of_a_failing_design_without_validating() {
    let case_dir = "negative_cross_design_exit_missing";

    let (success, links) = run_dependable_element_links_only(
        &format!("{SUITE_DIR}_{case_dir}_links_only"),
        inputs(case_dir),
    );

    assert!(success);
    assert!(call_links(&links).is_empty(), "links={links}");
    assert!(!links["element_links"].as_array().unwrap().is_empty());
}

#[test]
fn links_only_gives_the_links_of_a_valid_design() {
    let case_dir = "positive_cross_design";

    let (success, links) = run_dependable_element_links_only(
        &format!("{SUITE_DIR}_{case_dir}_links_only"),
        inputs(case_dir),
    );

    assert!(success);
    assert_eq!(call_links(&links).len(), 1, "links={links}");
}

#[test]
fn same_level_refinement_links_across_designs() {
    let case_dir = "same_level";
    let expected = load_expected_yaml_fixture(SUITE_DIR, case_dir);
    let input = same_level_inputs(&["overview.fbs.bin", "detail.fbs.bin"]);
    let (result, links) = run_dependable_element_links("same_level_valid", input.clone());
    assert_cli_result(case_dir, &expected, &normalize_yaml_result(result));

    assert_eq!(
        call_links(&links),
        &vec![json!({
            "source": case_source(case_dir, "overview.puml"),
            "line": 19,
            "method": "Get",
            "targets": [case_source(case_dir, "detail.puml")],
        })],
    );

    let (success, links_only) = run_dependable_element_links_only("same_level_valid", input);
    assert!(success);
    assert_eq!(call_links(&links_only), call_links(&links));
}

#[test]
fn unclosed_same_level_detail_is_not_linked_or_reported() {
    let input = same_level_inputs(&["overview.fbs.bin", "unclosed.fbs.bin"]);
    let (result, links) = run_dependable_element_links("same_level_unclosed", input.clone());

    assert!(result.success, "log={}", result.log_contents);
    assert!(call_links(&links).is_empty(), "links={links}");

    let (success, links_only) = run_dependable_element_links_only("same_level_unclosed", input);
    assert!(success);
    assert!(call_links(&links_only).is_empty(), "links={links_only}");
}

#[test]
fn async_boundary_survives_parsing_and_links_across_designs() {
    let input = same_level_inputs(&["async_overview.fbs.bin", "async_detail.fbs.bin"]);
    let (result, links) = run_dependable_element_links("same_level_async", input);
    assert!(result.success, "log={}", result.log_contents);
    assert_eq!(
        call_links(&links),
        &vec![json!({
            "source": case_source("same_level", "async_overview.puml"),
            "line": 18,
            "method": "Get",
            "targets": [case_source("same_level", "async_detail.puml")],
        })],
    );
}

#[test]
fn multi_participant_window_end_is_reported_through_the_parser() {
    let input = same_level_inputs(&["invalid_end.fbs.bin"]);
    let (result, _) = run_dependable_element_links("same_level_invalid_end", input);

    assert!(!result.success, "log={}", result.log_contents);
    assert!(
        result
            .log_contents
            .contains("SameLevelAsyncWindowEndInvalid"),
        "log={}",
        result.log_contents
    );
}

#[test]
fn architectural_design_profile_has_no_zoom_findings() {
    let case_dir = "negative_cross_design_exit_missing";
    let result = run_validation_profile(
        &format!("{SUITE_DIR}_{case_dir}_architectural_design"),
        "architectural-design",
        json!({
            "component_diagrams": collect_case_fbs_files(SUITE_DIR, case_dir, "component"),
            "sequence_diagrams": collect_case_fbs_files(SUITE_DIR, case_dir, "sequence"),
        }),
    );
    assert!(
        !result.log_contents.contains("Zoom"),
        "log={}",
        result.log_contents
    );
}
