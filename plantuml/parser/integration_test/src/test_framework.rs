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
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::test_error_view::{ErrorView, ProjectedError};

// =================== Common Functions ===================
fn get_case_dir(test_module: &str, case_name: &str) -> PathBuf {
    let runfiles_dir = std::env::var("TEST_SRCDIR").unwrap();
    let workspace = std::env::var("TEST_WORKSPACE").unwrap();

    PathBuf::from(format!(
        "{}/{}/plantuml/parser/{}/{}",
        runfiles_dir, workspace, test_module, case_name
    ))
}

fn get_puml_files(dir: &Path) -> HashSet<Rc<PathBuf>> {
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| {
            let path = e.unwrap().path();
            if path.extension().map(|s| s == "puml").unwrap_or(false) {
                Some(Rc::new(path))
            } else {
                None
            }
        })
        .collect()
}

// =================== Helper: normalize YAML text ===================
fn normalize_yaml_text(s: &str) -> String {
    s.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

// =================== Golden File ===================
enum GoldenValue {
    Yaml(YamlValue),
    Json(JsonValue),
}

fn load_golden_file(test_module: &str, case_name: &str) -> GoldenValue {
    let dir = get_case_dir(test_module, case_name);

    let yaml_path = dir.join("output.yaml");
    let yaml_exists = yaml_path.exists();

    let json_path = dir.join("output.json");
    let json_exists = json_path.exists();

    if yaml_exists && json_exists {
        panic!(
            "Both output.yaml and output.json exist in {:?}. Please keep only one.",
            dir
        );
    }

    if yaml_exists {
        let content = fs::read_to_string(&yaml_path).unwrap();
        return GoldenValue::Yaml(serde_yaml::from_str(&content).unwrap());
    }

    if json_exists {
        let content = fs::read_to_string(&json_path).unwrap();
        return GoldenValue::Json(serde_json::from_str(&content).unwrap());
    }

    panic!(
        "No golden file found in {:?}. Expected output.yaml or output.json",
        dir
    );
}

pub enum Expected<Output> {
    Text(String),
    Ast(Output),
}

fn materialize_expected<Output>(value: &GoldenValue) -> Expected<Output>
where
    Output: DeserializeOwned,
{
    match value {
        GoldenValue::Yaml(v) => match v {
            YamlValue::String(s) => Expected::Text(normalize_yaml_text(s)),
            _ => {
                panic!(
                    "YAML golden only supports string output, \
                         use JSON if you want to compare AST"
                );
            }
        },
        GoldenValue::Json(v) => {
            let ast: Output = serde_json::from_value(v.clone())
                .expect("Failed to deserialize expected AST from JSON");
            Expected::Ast(ast)
        }
    }
}

// =================== DiagramProcessor ===================
pub trait DiagramProcessor {
    type Output;
    type Error;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, Self::Output>, Self::Error>;
}

// =================== ExpectationChecker ===================
pub trait ExpectationChecker<Error: ErrorView, Output> {
    fn check_ok(&self, actual: &Output, expected: &Expected<Output>);
    fn check_err(&self, err: &Error, expected: &YamlValue, base_dir: &Path);
}

// =================== Default Checker ===================
pub struct DefaultExpectationChecker;

impl<Error, Output> ExpectationChecker<Error, Output> for DefaultExpectationChecker
where
    Error: ErrorView,
    Output: Serialize + PartialEq + std::fmt::Debug,
{
    fn check_ok(&self, actual: &Output, expected: &Expected<Output>) {
        match expected {
            Expected::Text(expected_text) => {
                let actual_str = match serde_yaml::to_value(actual) {
                    Ok(YamlValue::String(s)) => s,
                    _ => {
                        format!("{:?}", actual)
                    }
                };

                assert_eq!(
                    normalize_yaml_text(&actual_str),
                    normalize_yaml_text(expected_text),
                    "String output mismatch"
                );
            }
            Expected::Ast(expected_ast) => {
                assert_eq!(actual, expected_ast, "AST output mismatch");
            }
        }
    }

    fn check_err(&self, err: &Error, expected: &YamlValue, base_dir: &Path) {
        let projected = err.project(base_dir);
        assert_projected_error_matches_yaml(&projected, expected);
    }
}

// =================== Error Matcher ===================
fn assert_projected_error_matches_yaml(err: &ProjectedError, expected: &YamlValue) {
    let expected_type = expected
        .get("type")
        .and_then(|v| v.as_str())
        .expect("Missing error.type");

    assert_eq!(err.kind, expected_type, "Error kind mismatch");

    if let Some(fields) = expected.get("fields") {
        let fields = fields.as_mapping().expect("fields must be a map");

        for (k, v) in fields {
            let key = k.as_str().unwrap();
            let expected_val = v.as_str().unwrap();

            let actual_val = err
                .fields
                .get(key)
                .unwrap_or_else(|| panic!("Missing field: {}", key));

            assert_eq!(actual_val, expected_val, "Field '{}' mismatch", key);
        }
    }

    match (expected.get("source"), &err.source) {
        (Some(expected_src), Some(actual_src)) => {
            assert_projected_error_matches_yaml(actual_src, expected_src);
        }
        (None, None) => {}
        (Some(_), None) => panic!("Expected source error, but none found"),
        (None, Some(_)) => panic!("Unexpected source error"),
    }
}

// =================== Test Driver ===================
pub fn run_case<P, C>(test_module: &str, case_name: &str, processor: P, checker: C)
where
    P: DiagramProcessor,
    P::Error: std::fmt::Debug + ErrorView,
    P::Output: std::fmt::Debug + Serialize + DeserializeOwned + PartialEq,
    C: ExpectationChecker<P::Error, P::Output>,
{
    let dir = get_case_dir(test_module, case_name);
    let file_list = get_puml_files(&dir);

    let result = processor.run(&file_list);
    let expected = load_golden_file(test_module, case_name);

    match expected {
        GoldenValue::Yaml(ref yaml) => {
            for (key_value, expected_value) in yaml.as_mapping().unwrap() {
                let key = key_value.as_str().unwrap();
                let path = dir.join(key);

                if let Some(error_obj) = expected_value.get("error") {
                    let err = result
                        .as_ref()
                        .err()
                        .unwrap_or_else(|| panic!("Expected error for {}", key));
                    checker.check_err(err, error_obj, &dir);
                } else {
                    let actual = result
                        .as_ref()
                        .unwrap()
                        .get(&path)
                        .unwrap_or_else(|| panic!("Missing output for {}", key));
                    let expected = materialize_expected::<P::Output>(&GoldenValue::Yaml(
                        expected_value.clone(),
                    ));
                    checker.check_ok(actual, &expected);
                }
            }
        }
        GoldenValue::Json(ref json) => {
            for (key, expected_value) in json.as_object().unwrap() {
                let path = dir.join(key);
                let actual = result
                    .as_ref()
                    .unwrap()
                    .get(&path)
                    .unwrap_or_else(|| panic!("Missing output for {}", key));
                let expected =
                    materialize_expected::<P::Output>(&GoldenValue::Json(expected_value.clone()));
                checker.check_ok(actual, &expected);
            }
        }
    }
}
