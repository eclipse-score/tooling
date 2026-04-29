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

pub mod bazel_component_validator;
pub mod component_class_validator;
pub mod component_sequence_validator;

/// Typed inputs that a validator may require to run.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RequiredInput {
    Bazel,
    Component,
    Class,
}

/// Validators supported by the current CLI.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SelectedValidator {
    BazelComponent,
    ComponentClass,
}

pub const ALL_VALIDATORS: [SelectedValidator; 2] = [
    SelectedValidator::BazelComponent,
    SelectedValidator::ComponentClass,
];

/// Validator metadata and execution contract used by orchestrators.
pub trait ValidatorSpec {
    fn required_inputs(self) -> &'static [RequiredInput];

    fn can_run(self, is_available: impl Fn(RequiredInput) -> bool) -> bool
    where
        Self: Sized,
    {
        self.required_inputs()
            .iter()
            .all(|input| is_available(*input))
    }
}

impl ValidatorSpec for SelectedValidator {
    fn required_inputs(self) -> &'static [RequiredInput] {
        match self {
            SelectedValidator::BazelComponent => &[RequiredInput::Bazel, RequiredInput::Component],
            SelectedValidator::ComponentClass => &[RequiredInput::Component, RequiredInput::Class],
        }
    }
}

pub use bazel_component_validator::{validate_bazel_component, BazelComponentValidator};
pub use component_class_validator::{validate_component_class, ComponentClassValidator};
pub use component_sequence_validator::validate_component_sequence;
