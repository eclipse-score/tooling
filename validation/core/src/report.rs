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

use std::fs;

use validation::{Profile, ProfileRun};

pub fn finish_profile_validation(
    output_path: Option<&str>,
    warn_on_errors: bool,
    profile: Profile,
    profile_run: &ProfileRun,
) -> Result<(), String> {
    if !profile_run.ran_validator {
        log::info!(
            "Skipping validation profile {}: no selected validators have their required inputs.",
            profile.as_str()
        );
        write_skipped_log(output_path, profile)?;
        return Ok(());
    }

    finish_validation(output_path, warn_on_errors, profile_run)
}

fn finish_validation(
    output_path: Option<&str>,
    warn_on_errors: bool,
    profile_run: &ProfileRun,
) -> Result<(), String> {
    let result = &profile_run.result;
    if let Some(path) = output_path {
        write_log(path, profile_run)?;
    }

    if result.is_empty() {
        Ok(())
    } else {
        let output = format!(
            "Verification FAILED ({} error(s)):\n\n{}",
            result.failures.len(),
            format_error_details(profile_run, "  ")
        );
        if warn_on_errors {
            log::warn!("{}", output);
            Ok(())
        } else {
            Err(output)
        }
    }
}

fn write_skipped_log(path: Option<&str>, profile: Profile) -> Result<(), String> {
    if let Some(path) = path {
        let content = format!(
            "SKIPPED\n\nNo validators ran for profile {}: required inputs were not present.\n",
            profile.as_str()
        );
        fs::write(path, content).map_err(|e| format!("Failed to write output file {path}: {e}"))?;
    }
    Ok(())
}

fn write_log(path: &str, profile_run: &ProfileRun) -> Result<(), String> {
    let result = &profile_run.result;
    let content = if result.is_empty() {
        if result.diagnostics.is_empty() {
            "PASS\n".to_string()
        } else {
            let mut output = "PASS\n\n".to_string();
            output.push_str("\n--- Diagnostic Information ---\n\n");
            output.push_str(&result.diagnostics.render());
            output
        }
    } else {
        let mut output = format!(
            "FAILED ({} error(s)):\n\n{}\n\n",
            result.failures.len(),
            format_error_details(profile_run, "")
        );
        if !result.diagnostics.is_empty() {
            output.push_str("\n--- Diagnostic Information ---\n\n");
            output.push_str(&result.diagnostics.render());
        }
        output
    };
    fs::write(path, content).map_err(|e| format!("Failed to write output file {path}: {e}"))
}

fn format_error_details(profile_run: &ProfileRun, prefix: &str) -> String {
    profile_run
        .result
        .failures
        .iter()
        .enumerate()
        .map(|(i, msg)| format!("{}[{}] {}", prefix, i + 1, msg))
        .collect::<Vec<_>>()
        .join("\n\n")
}
