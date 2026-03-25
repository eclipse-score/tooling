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

//! Syntax parser job: Parse PUML file and output JSON

use sequence_parser::parse_sequence_diagram;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <input.puml> <output.json>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];

    // Read the PUML file
    let puml_content = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", input_file, e);
            std::process::exit(1);
        }
    };

    // Parse the sequence diagram
    let (_doc_name, statements) = match parse_sequence_diagram(&puml_content) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error parsing sequence diagram: {}", e);
            std::process::exit(1);
        }
    };

    // Serialize to JSON (use IDs as-is, don't translate to quoted names)
    let json = match serde_json::to_string_pretty(&statements) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Error serializing to JSON: {}", e);
            std::process::exit(1);
        }
    };

    // Write to output file
    if let Err(e) = fs::write(output_file, json) {
        eprintln!("Error writing to '{}': {}", output_file, e);
        std::process::exit(1);
    }

    println!(
        "✓ Syntax parse complete: {} statements → {}",
        statements.len(),
        output_file
    );
}
