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

//! Validation CLI entrypoint.
//!
//! Supports architecture and design validations selected by validation profile.
mod report;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, ValueEnum};
use env_logger::Builder;
use std::fs;
use std::process;

use validation::{diagram_links, read_profile_inputs, run_profile, Profile, ProfileInputs};

/// CLI-visible log level (mirrors the parser/linker convention).
#[derive(Copy, Clone, ValueEnum, Debug)]
enum CliLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl CliLogLevel {
    fn to_level_filter(self) -> log::LevelFilter {
        match self {
            CliLogLevel::Error => log::LevelFilter::Error,
            CliLogLevel::Warn => log::LevelFilter::Warn,
            CliLogLevel::Info => log::LevelFilter::Info,
            CliLogLevel::Debug => log::LevelFilter::Debug,
            CliLogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "validation")]
#[command(version = "1.0")]
#[command(about = "Validate architecture and design consistency from PlantUML exports")]
struct Args {
    /// Validation profile. Profiles select the validators to run.
    #[arg(long, value_enum)]
    profile: Profile,

    /// JSON file containing input paths for the selected validation profile.
    #[arg(long)]
    inputs: String,

    #[arg(long)]
    output: Option<String>,

    /// When set, validation errors are printed as warnings and the tool exits
    /// with code 0. Intended for use during development (maturity=development).
    #[arg(long, default_value_t = false)]
    warn_on_errors: bool,

    /// Log level: error, warn, info, debug, trace
    #[arg(long, value_enum, default_value = "warn")]
    log_level: CliLogLevel,

    /// Write the diagram links (`diagram_links.json`) to this file. Only with
    /// `--profile dependable-element`. Call links lead to refinements that pass
    /// the zoom check; written even when validation fails.
    #[arg(long)]
    links_output: Option<String>,

    /// Write only the diagram links: no validation, no report. Needs
    /// `--links-output`.
    #[arg(
        long,
        default_value_t = false,
        requires = "links_output",
        conflicts_with_all = ["output", "warn_on_errors"]
    )]
    links_only: bool,
}

fn write_links(path: &str, inputs: &ProfileInputs) -> Result<(), String> {
    let links = diagram_links(inputs)?;
    fs::write(path, links.to_json() + "\n")
        .map_err(|e| format!("Failed to write diagram links {path}: {e}"))
}

fn run(args: Args) -> Result<(), String> {
    let inputs = read_profile_inputs(args.profile, &args.inputs)?;
    if let (true, Some(path)) = (args.links_only, &args.links_output) {
        return write_links(path, &inputs);
    }
    let profile_run = run_profile(args.profile, &inputs)?;
    if let Some(path) = &args.links_output {
        write_links(path, &inputs)?;
    }
    report::finish_profile_validation(
        args.output.as_deref(),
        args.warn_on_errors,
        args.profile,
        &profile_run,
    )
}

fn main() {
    let args = Args::parse();
    if args.links_output.is_some() && args.profile != Profile::DependableElement {
        Args::command()
            .error(
                ErrorKind::ArgumentConflict,
                "--links-output needs --profile dependable-element",
            )
            .exit();
    }
    Builder::new()
        .filter_level(args.log_level.to_level_filter())
        .init();
    if let Err(msg) = run(args) {
        log::error!("{}", msg);
        process::exit(1);
    }
}
