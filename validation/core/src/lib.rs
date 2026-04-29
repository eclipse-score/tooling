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

//! Unified validation library.
//!
//! This crate contains the shared models, readers, and validators used by the
//! CLI entrypoints for architecture and design verification.

pub mod models;
pub mod readers;
pub mod validators;

pub use models::{
    BazelArchitecture, BazelInput, BazelInputEntry, ClassDiagramEntityInput, ClassDiagramIndex,
    ClassDiagramInput, ClassDiagramInputs, ClassDiagramRelationshipInput,
    ComponentDiagramArchitecture, ComponentDiagramInput, ComponentDiagramInputs, EntityKey, Errors,
};

pub use readers::{BazelReader, ClassDiagramReader, ComponentDiagramReader, Reader};

pub use validators::{
    validate_bazel_component, validate_component_class, validate_component_sequence,
    BazelComponentValidator, ComponentClassValidator, RequiredInput, SelectedValidator,
    ValidatorSpec, ALL_VALIDATORS,
};
