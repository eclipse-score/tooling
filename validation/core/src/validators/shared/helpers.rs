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

//! Helper functions shared by validators.

use std::collections::BTreeSet;

pub(in crate::validators) fn format_name_list(names: &BTreeSet<String>) -> String {
    if names.is_empty() {
        return "<none>".to_string();
    }

    names
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in crate::validators) fn format_sequence_call(
    caller_unit: &str,
    callee_unit: &str,
    method_name: &str,
) -> String {
    format!("\"{caller_unit}\" -> \"{callee_unit}\" : \"{method_name}\"")
}

pub(in crate::validators) fn extract_method_name(method: &str) -> &str {
    method.split('(').next().unwrap_or(method).trim()
}

pub(in crate::validators) fn intersect_interfaces(
    left_interfaces: &BTreeSet<String>,
    right_interfaces: &BTreeSet<String>,
) -> BTreeSet<String> {
    left_interfaces
        .intersection(right_interfaces)
        .cloned()
        .collect()
}
