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
mod component_class_validator;
mod component_sequence_validator;

/// Typed inputs that a validator may require to run.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RequiredInput {
    Bazel,
    Component,
    Sequence,
    Class,
}

/// Validators supported by the current CLI.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SelectedValidator {
    BazelComponent,
    ComponentClass,
    ComponentSequence,
}

pub const ALL_VALIDATORS: [SelectedValidator; 3] = [
    SelectedValidator::BazelComponent,
    SelectedValidator::ComponentClass,
    SelectedValidator::ComponentSequence,
];

impl SelectedValidator {
    pub fn required_inputs(self) -> &'static [RequiredInput] {
        match self {
            Self::BazelComponent => &[RequiredInput::Bazel, RequiredInput::Component],
            Self::ComponentClass => &[RequiredInput::Component, RequiredInput::Class],
            Self::ComponentSequence => &[RequiredInput::Component, RequiredInput::Sequence],
        }
    }

    pub fn can_run(self, is_available: impl Fn(RequiredInput) -> bool) -> bool {
        self.required_inputs()
            .iter()
            .all(|input| is_available(*input))
    }
}

pub use bazel_component_validator::validate_bazel_component;
pub use component_class_validator::validate_component_class;
pub use component_sequence_validator::validate_component_sequence;
