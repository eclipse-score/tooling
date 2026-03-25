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

//! Syntax parser test suite: Compare parsed output with expected JSON for each test pair

use parser_core::DiagramParser;
use puml_utils::LogLevel;
use sequence_parser::PumlSequenceParser;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

fn test_file_pair(puml_file: &str, json_file: &str) {
    // Read and parse the PUML file
    let puml_content = fs::read_to_string(puml_file)
        .unwrap_or_else(|e| panic!("Error reading input file '{}': {}", puml_file, e));

    // Use DiagramParser trait directly
    let mut parser = PumlSequenceParser;
    let path = Rc::new(PathBuf::from(puml_file));
    let document = parser
        .parse_file(&path, &puml_content, LogLevel::Error)
        .unwrap_or_else(|e| panic!("Error parsing sequence diagram '{}': {}", puml_file, e));

    // Serialize parsed statements to JSON (not the full document with name)
    let actual_json = serde_json::to_string_pretty(&document.statements)
        .expect("Error serializing parsed result to JSON");

    // Read expected JSON
    let expected_json = fs::read_to_string(json_file)
        .unwrap_or_else(|e| panic!("Error reading expected file '{}': {}", json_file, e));

    // Parse both JSONs to normalize formatting
    let actual_value: serde_json::Value =
        serde_json::from_str(&actual_json).expect("Error parsing actual JSON");

    let expected_value: serde_json::Value =
        serde_json::from_str(&expected_json).expect("Error parsing expected JSON");

    // Compare the values
    if actual_value != expected_value {
        eprintln!(
            "\nExpected JSON:\n{}",
            serde_json::to_string_pretty(&expected_value).unwrap()
        );
        eprintln!(
            "\nActual JSON:\n{}",
            serde_json::to_string_pretty(&actual_value).unwrap()
        );

        panic!("Parsed output does not match expected JSON");
    }
}

#[test]
fn test_comprehensive_sequence() {
    test_file_pair(
        "plantuml/parser/integration_test/sequence_diagram/comprehensive_sequence_test.puml",
        "plantuml/parser/integration_test/sequence_diagram/comprehensive_sequence_test.json",
    );
}
