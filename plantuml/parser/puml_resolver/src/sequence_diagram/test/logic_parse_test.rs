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

//! Logic parser test suite: Compare logic_parse output with expected JSON

use sequence_parser::syntax_ast::Statement;
use sequence_resolver::logic_parser::build_tree;
use sequence_resolver::{ConditionType, Event};
use std::fs;

#[test]
fn test_logic_parse_output() {
    // Read the syntax.json file
    let syntax_file =
        "plantuml/parser/integration_test/sequence_diagram/simple_sequence.json";
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

#[test]
fn test_logic_parse_nested_loops_match_branch_nesting_rules() {
    let syntax_file =
        "plantuml/parser/integration_test/sequence_diagram/simple_sequence.json";

    let json_content = fs::read_to_string(syntax_file)
        .unwrap_or_else(|e| panic!("Error reading input file '{}': {}", syntax_file, e));
    let statements: Vec<Statement> = serde_json::from_str(&json_content)
        .unwrap_or_else(|e| panic!("Error parsing JSON from '{}': {}", syntax_file, e));

    let tree = build_tree(&statements);
    assert_eq!(tree.len(), 1, "expected a single root interaction");

    let root = &tree[0];
    assert_eq!(root.branches_node.len(), 2, "expected alt and else arms");

    let else_branch = &root.branches_node[1];
    match &else_branch.event {
        Event::Condition(condition) => assert_eq!(condition.condition_type, ConditionType::Else),
        other => panic!("expected else condition node, got {:?}", other),
    }

    assert!(
        else_branch.branches_node.len() >= 4,
        "expected direct call, two loops, and trailing branch inside else arm"
    );

    let for_loop = &else_branch.branches_node[1];
    match &for_loop.event {
        Event::Condition(condition) => {
            assert_eq!(condition.condition_type, ConditionType::Loop);
            assert_eq!(condition.condition_value, "for i = 0; i < 3; ++i");
        }
        other => panic!("expected for-loop condition node, got {:?}", other),
    }
    assert_eq!(
        for_loop.branches_node.len(),
        2,
        "expected call and nested branch in for-loop"
    );
    match &for_loop.branches_node[1].event {
        Event::Condition(condition) => {
            assert_eq!(condition.condition_type, ConditionType::Alt);
            assert_eq!(condition.condition_value, "innerConditionA");
        }
        other => panic!("expected nested alt inside for-loop, got {:?}", other),
    }

    let while_loop = &else_branch.branches_node[2];
    match &while_loop.event {
        Event::Condition(condition) => {
            assert_eq!(condition.condition_type, ConditionType::Loop);
            assert_eq!(condition.condition_value, "while count > 0");
        }
        other => panic!("expected while-loop condition node, got {:?}", other),
    }
    assert_eq!(
        while_loop.branches_node.len(),
        3,
        "expected call plus alt/else arms inside while-loop"
    );
    match &while_loop.branches_node[1].event {
        Event::Condition(condition) => assert_eq!(condition.condition_type, ConditionType::Alt),
        other => panic!("expected nested alt inside while-loop, got {:?}", other),
    }
    match &while_loop.branches_node[2].event {
        Event::Condition(condition) => assert_eq!(condition.condition_type, ConditionType::Else),
        other => panic!("expected nested else inside while-loop, got {:?}", other),
    }
}
