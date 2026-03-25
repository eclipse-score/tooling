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

//! Logic parser test suite: Compare logic_parse output with expected JSON

use sequence_parser::syntax_ast::Statement;
use sequence_resolver::logic_parser::build_tree;
use std::fs;

#[test]
fn test_logic_parse_output() {
    // Read the syntax.json file
    let syntax_file = "plantuml/parser/puml_resolver/src/sequence_diagram/test/syntax.json";
    let expected_file = "plantuml/parser/puml_resolver/src/sequence_diagram/test/logic.json";

    let json_content = fs::read_to_string(syntax_file)
        .unwrap_or_else(|e| panic!("Error reading input file '{}': {}", syntax_file, e));

    // Deserialize the statements
    let statements: Vec<Statement> = serde_json::from_str(&json_content)
        .unwrap_or_else(|e| panic!("Error parsing JSON from '{}': {}", syntax_file, e));

    // Build the tree (same logic as logic_parse_main.rs)
    let tree = build_tree(&statements);

    // Serialize the tree to JSON
    let actual_json = serde_json::to_string_pretty(&tree).expect("Error serializing tree to JSON");

    // Read expected JSON
    let expected_json = fs::read_to_string(expected_file)
        .unwrap_or_else(|e| panic!("Error reading expected file '{}': {}", expected_file, e));

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
        panic!("Logic parse output does not match expected output");
    }
}
