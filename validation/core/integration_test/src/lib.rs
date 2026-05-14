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

mod test_framework;

pub use test_framework::{
    assert_cli_result, case_file_path, collect_case_fbs_files, load_expected_fixture,
    read_case_file, run_validation_cli, CliRunResult, ExpectedFixture, ValidationIntegrationCase,
};
