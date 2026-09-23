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
mod display;
mod helpers;

pub(in crate::validators) use diagram_analysis::{
    build_observed_call_contexts, build_unit_bindings, SequenceCallContext, UnitBindings,
    UnitInterfaces,
};
pub(crate) use display::display_name_from_source_path;
pub(crate) use display::{display_entity_name, display_reference_name, display_relationship_name};
pub(in crate::validators) use display::{
    display_name_from_source_path_in_context, display_name_from_sources, display_names,
    display_names_from_sources, display_names_without_common_prefix, display_reference_name_set,
    display_unit_pair_from_optional_source_paths, format_display_names, format_name_list,
    format_sequence_call,
};
pub(in crate::validators) use helpers::{
    best_string_suggestion, earliest_source_by_id, extract_method_name, intersect_interfaces,
};
pub(crate) use uid_utils::normalize;
