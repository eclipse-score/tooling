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

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use test_framework::{
    assert_cli_result, case_file_path, collect_case_fbs_files, load_expected_yaml_fixture,
    normalize_yaml_result, run_validation_profile, CliRunResult,
};

const SUITE_DIR: &str = "architectural_unit_class_design";

fn collect_unit_design_class_fbs(
    architecture_path: &Path,
    class_fbs_paths: &[String],
) -> BTreeMap<String, Vec<String>> {
    let architecture: Value = serde_json::from_slice(
        &std::fs::read(architecture_path).expect("failed to read architecture fixture"),
    )
    .expect("failed to parse architecture fixture");
    let components = architecture["components"]
        .as_object()
        .expect("architecture fixture must contain a components object");

    let mut class_fbs_by_unit_label = BTreeMap::new();
    let mut assigned_paths = BTreeSet::new();
    for component in components.values() {
        let Some(units) = component.get("units").and_then(Value::as_array) else {
            continue;
        };

        for unit in units {
            let label = unit["label"]
                .as_str()
                .expect("architecture unit must have a label");
            let design_name = unit["design_name"]
                .as_str()
                .expect("architecture unit must have a design_name");
            let file_prefix = format!("{design_name}_");
            let unit_class_fbs = class_fbs_paths
                .iter()
                .filter(|path| {
                    Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(&file_prefix))
                })
                .cloned()
                .collect::<Vec<_>>();

            for path in &unit_class_fbs {
                assert!(
                    assigned_paths.insert(path.clone()),
                    "class-diagram FlatBuffer {path} matches multiple unit design names"
                );
            }
            class_fbs_by_unit_label.insert(label.to_string(), unit_class_fbs);
        }
    }

    assert_eq!(
        assigned_paths.len(),
        class_fbs_paths.len(),
        "every unit-design class diagram must match a unit design_name"
    );
    class_fbs_by_unit_label
}

fn assert_case(case_dir: &str) {
    let expected = load_expected_yaml_fixture(SUITE_DIR, case_dir);
    let component_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "component");
    let class_fbs_paths = collect_case_fbs_files(SUITE_DIR, case_dir, "unit_design_class");
    let architecture_path = case_file_path(&format!(
        "validation/core/integration_test/{SUITE_DIR}/{case_dir}/architecture.json"
    ));
    let unit_design_class_diagrams =
        collect_unit_design_class_fbs(&architecture_path, &class_fbs_paths);

    let result: CliRunResult = run_validation_profile(
        &format!("architectural_unit_class_design_{case_dir}"),
        "dependable-element",
        serde_json::json!({
            "architecture": architecture_path.display().to_string(),
            "component_diagrams": component_fbs_paths,
            "unit_design_class_diagrams": unit_design_class_diagrams,
        }),
    );

    let result = normalize_yaml_result(result);

    assert_cli_result(case_dir, &expected, &result);
}

#[test]
fn positive_refining_class() {
    assert_case("positive_refining_class");
}

#[test]
fn negative_path_prefix() {
    assert_case("negative_path_prefix");
}

#[test]
fn negative_missing_class() {
    assert_case("negative_missing_class");
}
