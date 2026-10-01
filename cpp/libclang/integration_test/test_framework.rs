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

use std::{path::PathBuf, str::FromStr};

/// Expected `source_location.file == ""` ignores the actual file; otherwise both
/// must match after dropping a leading `./`.
fn normalize_source_files(expected: &mut serde_json::Value, actual: &mut serde_json::Value) {
    use serde_json::Value;

    match (expected, actual) {
        (Value::Object(expected_map), Value::Object(actual_map)) => {
            if let (Some(Value::Object(expected_location)), Some(Value::Object(actual_location))) = (
                expected_map.get_mut("source_location"),
                actual_map.get_mut("source_location"),
            ) {
                if expected_location.get("file").and_then(Value::as_str) == Some("") {
                    expected_location.remove("file");
                    actual_location.remove("file");
                } else {
                    for location in [expected_location, actual_location] {
                        if let Some(Value::String(file)) = location.get_mut("file") {
                            if let Some(stripped) = file.strip_prefix("./") {
                                *file = stripped.to_string();
                            }
                        }
                    }
                }
            }
            for (key, expected_value) in expected_map.iter_mut() {
                if let Some(actual_value) = actual_map.get_mut(key) {
                    normalize_source_files(expected_value, actual_value);
                }
            }
        }
        (Value::Array(expected_array), Value::Array(actual_array)) => {
            for (expected_value, actual_value) in expected_array.iter_mut().zip(actual_array) {
                normalize_source_files(expected_value, actual_value);
            }
        }
        _ => {}
    }
}

fn compare(expected_path: &str, output_path: &str) {
    let expected = PathBuf::from_str(expected_path).unwrap();
    let output = PathBuf::from_str(output_path).unwrap();

    let mut expected_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(expected).unwrap()).unwrap();
    println!("Expected JSON: {}", expected_json);

    let mut actual_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(output).unwrap()).unwrap();

    normalize_source_files(&mut expected_json, &mut actual_json);
    assert_json_diff::assert_json_eq!(expected_json, actual_json);
}

pub fn run_parser_case() {
    let expected_path = std::env::var("EXPECTED_OUTPUT_PATH").unwrap();
    let output_path = std::env::var("DEBUG_JSON_OUTPUT_PATH").unwrap();
    compare(&expected_path, &output_path);
}
