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

/// Accumulates validation messages together with optional debug output.
#[derive(Debug, Default)]
pub struct Errors {
    pub messages: Vec<String>,
    pub debug_output: String,
}

impl Errors {
    pub fn push(&mut self, message: String) {
        self.messages.push(message);
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
