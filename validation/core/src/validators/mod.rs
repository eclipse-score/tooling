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

//! Validator entrypoints for architecture checks.

mod bazel_component_validator;
mod class_design_implementation_validator;
mod component_internal_api_validator;
mod component_sequence_validator;
mod sequence_internal_api_validator;
mod shared;

#[cfg(test)]
#[path = "test/fixtures.rs"]
pub(crate) mod fixtures;

pub use bazel_component_validator::validate_bazel_component;
pub use class_design_implementation_validator::validate_class_design_implementation;
pub use component_internal_api_validator::validate_component_internal_api;
pub use component_sequence_validator::validate_component_sequence;
pub use sequence_internal_api_validator::validate_sequence_internal_api;
