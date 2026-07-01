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

//! Validation execution results.

mod diagnostics;

pub use diagnostics::Diagnostics;

#[derive(Debug, Default)]
pub struct ValidationResult {
    pub failures: Vec<String>,
    pub diagnostics: Diagnostics,
}

impl ValidationResult {
    pub fn add_failure(&mut self, failure: String) {
        self.failures.push(failure);
    }

    pub fn is_empty(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn merge(&mut self, incoming: Self) {
        self.failures.extend(incoming.failures);
        self.diagnostics.append(incoming.diagnostics);
    }
}
