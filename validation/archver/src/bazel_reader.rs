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

use crate::models::BazelInput;

/// Reads the `architecture.json` file produced by the `dependable_element`
/// Bazel rule and deserializes it into a [`BazelInput`] model.
pub struct BazelReader;

impl BazelReader {
    /// Read and parse the architecture JSON at `path`.
    pub fn read(path: &str) -> Result<BazelInput, String> {
        let json_content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
        serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse architecture JSON: {e}"))
    }
}
