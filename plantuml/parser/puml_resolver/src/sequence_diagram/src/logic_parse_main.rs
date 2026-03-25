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

//! Logic parser job: Build hierarchical tree from syntax JSON

use sequence_parser::syntax_ast::Statement;
use sequence_resolver::logic_parser::build_tree;

use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <syntax.json> <output.json>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];

    // Read the syntax JSON file
    let json_content = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", input_file, e);
            std::process::exit(1);
        }
    };

    // Deserialize the statements
    let statements: Vec<Statement> = match serde_json::from_str(&json_content) {
        Ok(stmts) => stmts,
        Err(e) => {
            eprintln!("Error parsing JSON: {}", e);
            std::process::exit(1);
        }
    };

    // Build the logic tree
    let tree = build_tree(&statements);

    // Serialize to JSON
    let json_output = match serde_json::to_string_pretty(&tree) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Error serializing to JSON: {}", e);
            std::process::exit(1);
        }
    };

    // Write to output file
    if let Err(e) = fs::write(output_file, json_output) {
        eprintln!("Error writing to file '{}': {}", output_file, e);
        std::process::exit(1);
    }

    println!("✓ Logic tree built: {} nodes → {}", tree.len(), output_file);
}
