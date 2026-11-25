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

//! Method name of a sequence message label.

const CALL_OPERATOR: &str = "operator()";

/// Method name of a sequence message label, without the argument list.
/// The call operator keeps its parentheses, as in the class diagrams.
pub fn extract_method_name(method: &str) -> &str {
    let method = method.trim();
    if method.starts_with(CALL_OPERATOR) {
        return CALL_OPERATOR;
    }

    method.split('(').next().unwrap_or(method).trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_method_name_strips_the_argument_list() {
        assert_eq!(extract_method_name("Run(int x)"), "Run");
        assert_eq!(extract_method_name(" Run "), "Run");
        assert_eq!(extract_method_name("Run"), "Run");
    }

    #[test]
    fn extract_method_name_keeps_the_call_operator() {
        assert_eq!(extract_method_name("operator()"), "operator()");
        assert_eq!(extract_method_name("operator()(int x)"), "operator()");
        assert_eq!(extract_method_name("operator<(int x)"), "operator<");
    }
}
