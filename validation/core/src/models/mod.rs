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

//! Shared data models for validation inputs, indexed architectures, and error
//! accumulation.

mod bazel_models;
mod class_diagram_models;
mod component_diagram_models;
mod error_models;
mod sequence_diagram_models;
mod shared;

use shared::EntityKey;

#[cfg(test)]
pub use bazel_models::BazelInputEntry;
pub use bazel_models::{BazelArchitecture, BazelInput};
pub use class_diagram_models::{ClassDiagramIndex, ClassDiagramInputs};
pub use component_diagram_models::{
    ComponentDiagramArchitecture, ComponentDiagramInput, ComponentDiagramInputs,
};
pub use error_models::Errors;
pub use sequence_diagram_models::{
    SequenceDiagramIndex, SequenceDiagramInput, SequenceDiagramInputs,
};
