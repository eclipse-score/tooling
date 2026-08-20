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

// This binary has no test at all: its source must appear at exactly 0% in the
// coverage report via the --empty-profile baseline over the coverage-built
// executable (rust_binary targets provide no CcInfo archive).
fn main() {
    println!("{}", integration_lib::classify(1));
}
