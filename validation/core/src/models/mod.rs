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

//! Shared data models for validation inputs and indexed architectures.

mod architecture_tree;
mod bazel_models;
mod class_diagram_models;
mod component_diagram_models;
mod method_name;
mod sequence_diagram_models;
mod sequence_refinement_models;
mod shared;
pub(crate) mod type_names;

use shared::EntityKey;

pub use bazel_models::{BazelArchitecture, BazelInput};
#[cfg(test)]
pub use bazel_models::{BazelEntityRef, BazelInputEntry};
pub use class_diagram_models::{
    ClassDiagramInputs, ClassEntityIndex, InternalApiIndex, InternalApiInterface, PublicApiIndex,
};
pub use component_diagram_models::{
    ComponentDiagramArchitecture, ComponentDiagramInputs, ComponentRelationType, ComponentType,
    EndpointRole, LogicComponent, LogicComponentExt, LogicRelation,
};
pub use method_name::extract_method_name;
pub use sequence_diagram_models::{
    Environment, ObservedSequenceCall, SequenceDiagramIndex, SequenceDiagramInputs,
    SequenceParticipantInfo, GATE_LABEL,
};
pub use sequence_refinement_models::{Lifeline, SequenceRefinementIndex, WindowCall, ZoomEntry};
