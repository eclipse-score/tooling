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

use clap::{Parser, ValueEnum};
use env_logger::Builder;
use std::process;

use validation::{read_profile_inputs, run_profile, Profile};

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
}

fn run(args: Args) -> Result<(), String> {
    let inputs = read_profile_inputs(args.profile, &args.inputs)?;
    let profile_run = run_profile(args.profile, &inputs)?;
    report::finish_profile_validation(
        args.output.as_deref(),
        args.warn_on_errors,
        args.profile,
        &profile_run,
    )
}

fn main() {
    let args = Args::parse();
    Builder::new()
        .filter_level(args.log_level.to_level_filter())
        .init();
    if let Err(msg) = run(args) {
        log::error!("{}", msg);
        process::exit(1);
    }
}
