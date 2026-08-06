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

/// Classifies an integer. Three branches; the unit test exercises only two so
/// the report is guaranteed to contain covered and uncovered Rust branches.
pub fn classify(value: i32) -> &'static str {
    if value < 0 {
        "negative"
    } else if value == 0 {
        "zero"
    } else {
        "positive"
    }
}

/// Never called by any test: must appear as uncovered (0 hits) in the report
/// thanks to -Clink-dead-code keeping it in the coverage map.
pub fn never_called(value: i32) -> i32 {
    value * 2
}

#[cfg(test)]
mod tests {
    use super::classify;

    #[test]
    fn classifies_negative() {
        assert_eq!(classify(-5), "negative");
    }

    #[test]
    fn classifies_zero() {
        assert_eq!(classify(0), "zero");
    }
}
