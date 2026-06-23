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

use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIntegrationCase {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct ExpectedFixture {
    pub should_pass: bool,
    pub error_contains: Vec<String>,
}

pub struct CliRunResult {
    pub success: bool,
    pub stderr: String,
    pub log_contents: String,
}

pub fn case_file_path(relative_path: &str) -> PathBuf {
    let test_srcdir = std::env::var("TEST_SRCDIR").expect("TEST_SRCDIR is not set");
    let workspace = std::env::var("TEST_WORKSPACE").expect("TEST_WORKSPACE is not set");

    PathBuf::from(test_srcdir)
        .join(workspace)
        .join(relative_path)
}

fn read_case_file(relative_path: &str) -> String {
    let path = case_file_path(relative_path);

    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", path.display()))
}

pub fn collect_case_fbs_files(suite_dir: &str, case_dir: &str, category: &str) -> Vec<String> {
    let dir_rel_path =
        format!("validation/core/integration_test/{suite_dir}/{case_dir}/{category}");
    let dir_path = case_file_path(&dir_rel_path);
    if !dir_path.exists() {
        return Vec::new();
    }

    let mut matches: Vec<String> = fs::read_dir(&dir_path)
        .unwrap_or_else(|error| {
            panic!(
                "failed to list generated fixture directory {}: {error}",
                dir_path.display()
            )
        })
        .filter_map(|entry| {
            let path = entry
                .unwrap_or_else(|error| {
                    panic!("failed to read entry in {}: {error}", dir_path.display())
                })
                .path();
            if !path.is_file() {
                return None;
            }

            let file_name = path.file_name()?.to_str()?;
            if !file_name.ends_with(".fbs.bin") {
                return None;
            }

            Some(path.display().to_string())
        })
        .collect();

    matches.sort();
    matches
}

pub fn load_expected_fixture(suite_dir: &str, case_dir: &str) -> ExpectedFixture {
    let expected_json = read_case_file(&format!(
        "validation/core/integration_test/{suite_dir}/{case_dir}/expected.json"
    ));

    serde_json::from_str(&expected_json).expect("failed to parse expected fixture")
}

pub fn assert_cli_result(case_dir: &str, expected: &ExpectedFixture, result: &CliRunResult) {
    assert_eq!(
        result.success, expected.should_pass,
        "unexpected validation result for {case_dir}: stderr={}, log={}",
        result.stderr, result.log_contents,
    );

    for fragment in &expected.error_contains {
        assert!(
            result.log_contents.contains(fragment),
            "expected an error containing {fragment:?} for {case_dir}, got log={} stderr={}",
            result.log_contents,
            result.stderr,
        );
    }
}

fn test_tmp_file_path(case_name: &str, suffix: &str) -> PathBuf {
    let sanitized_case_name = case_name.replace(['/', ' '], "_");
    let test_tmpdir = std::env::var("TEST_TMPDIR").expect("TEST_TMPDIR is not set");
    PathBuf::from(test_tmpdir).join(format!("{sanitized_case_name}{suffix}"))
}

fn run_validation_cli(case_name: &str, cli_args: &[String]) -> CliRunResult {
    let log_path = test_tmp_file_path(case_name, ".log");

    let validation_cli = case_file_path("validation/core/validation_cli");
    let output = Command::new(validation_cli)
        .args(cli_args)
        .arg("--output")
        .arg(&log_path)
        .output()
        .unwrap_or_else(|error| {
            panic!("failed to execute validation_cli for {case_name}: {error}")
        });

    let log_contents = fs::read_to_string(&log_path).unwrap_or_else(|error| {
        panic!(
            "failed to read validation log {}: {error}",
            log_path.display()
        )
    });

    CliRunResult {
        success: output.status.success(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        log_contents,
    }
}

pub fn run_validation_profile(case_name: &str, profile: &str, input_bundle: Value) -> CliRunResult {
    let inputs_path = test_tmp_file_path(case_name, "_inputs.json");
    fs::write(
        &inputs_path,
        serde_json::to_string_pretty(&input_bundle).expect("failed to serialize validation inputs"),
    )
    .unwrap_or_else(|error| {
        panic!(
            "failed to write validation input bundle {}: {error}",
            inputs_path.display()
        )
    });

    run_validation_cli(
        case_name,
        &[
            "--profile".to_string(),
            profile.to_string(),
            "--inputs".to_string(),
            inputs_path.display().to_string(),
        ],
    )
}
