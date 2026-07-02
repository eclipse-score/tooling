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

//! Shared validator analysis and helper utilities.

mod diagram_analysis;
mod helpers;

pub(in crate::validators) use diagram_analysis::{
    all_interfaces_for_alias, build_unit_bindings, UnitBindings, UnitInterfaces,
};
pub(in crate::validators) use helpers::{
    extract_method_name, format_name_list, format_sequence_call, intersect_interfaces,
};
