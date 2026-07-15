///////////////////////////////////////////////////////////////////////////////////
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
////////////////////////////////////////////////////////////////////////////////////

use std::{path::PathBuf, str::FromStr};

fn remove_source_location_file(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Object(source_location)) = map.get_mut("source_location")
            {
                source_location.remove("file");
            }

            for value in map.values_mut() {
                remove_source_location_file(value);
            }
        }
        serde_json::Value::Array(array) => {
            for value in array {
                remove_source_location_file(value);
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

    remove_source_location_file(&mut expected_json);
    remove_source_location_file(&mut actual_json);
    assert_json_diff::assert_json_eq!(expected_json, actual_json);
}

pub fn run_parser_case() {
    let expected_path = std::env::var("EXPECTED_OUTPUT_PATH").unwrap();
    let output_path = std::env::var("DEBUG_JSON_OUTPUT_PATH").unwrap();
    compare(&expected_path, &output_path);
}
