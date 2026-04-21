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

//! Architecture Verifier (archver)
//!
//! Validates that the PlantUML component diagram matches the Bazel build graph.
//! Uses stereotypes (`<<component>>`, `<<unit>>`, `<<SEooC>>`) to determine entity type.
//! Package entities (`<<SEooC>>`) are matched against the dependable element.
//! Comparison uses target names only (package paths are stripped).

mod bazel_reader;
mod diagram_reader;
mod models;
mod validation;

use std::fs;
use std::process;

use clap::Parser;

use bazel_reader::BazelReader;
use diagram_reader::DiagramReader;
use models::Errors;
use validation::validate;

#[derive(Parser, Debug)]
#[command(name = "archver")]
#[command(version = "1.0")]
#[command(about = "Validate architecture: compare Bazel build graph against PlantUML diagrams")]
struct Args {
    #[arg(long)]
    architecture_json: String,

    #[arg(long, num_args = 1..)]
    static_fbs: Vec<String>,

    #[arg(long)]
    output: Option<String>,
}

fn run(args: Args) -> Result<(), String> {
    let architecture = BazelReader::read(&args.architecture_json)?;
    let diagram = DiagramReader::read(&args.static_fbs)?;

    // Debug output is always produced; it is written to the log file which
    // is exposed via --output_groups=debug.
    let errors = validate(&architecture, &diagram);

    if let Some(ref path) = args.output {
        write_log(path, &errors)?;
    }

    if errors.is_empty() {
        Ok(())
    } else {
        let mut output = format!(
            "Architecture verification FAILED ({} error(s)):\n\n",
            errors.messages.len()
        );
        for (i, msg) in errors.messages.iter().enumerate() {
            output.push_str(&format!("  [{}] {}\n\n", i + 1, msg));
        }
        Err(output)
    }
}

fn write_log(path: &str, errors: &Errors) -> Result<(), String> {
    let content = if errors.is_empty() {
        format!("PASS\n\n{}", errors.debug_output)
    } else {
        let mut s = format!("FAILED ({} error(s)):\n\n", errors.messages.len());
        for (i, msg) in errors.messages.iter().enumerate() {
            s.push_str(&format!("[{}] {}\n\n", i + 1, msg));
        }
        s.push_str("\n--- Debug Information ---\n\n");
        s.push_str(&errors.debug_output);
        s
    };
    fs::write(path, content).map_err(|e| format!("Failed to write output file {path}: {e}"))
}

fn main() {
    if let Err(msg) = run(Args::parse()) {
        eprint!("{msg}");
        process::exit(1);
    }
}
