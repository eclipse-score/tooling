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

//! Profile-owned input schemas and validation orchestration.

mod architectural_design;
mod dependable_element;
mod profile;

pub use architectural_design::ArchitecturalDesignInputs;
pub use dependable_element::DependableElementInputs;
pub use profile::{Profile, ProfileRun};

pub enum ProfileInputs {
    ArchitecturalDesign(ArchitecturalDesignInputs),
    DependableElement(DependableElementInputs),
}

pub fn read_profile_inputs(profile: Profile, path: &str) -> Result<ProfileInputs, String> {
    match profile {
        Profile::ArchitecturalDesign => {
            profile::read_input_bundle(path).map(ProfileInputs::ArchitecturalDesign)
        }
        Profile::DependableElement => {
            profile::read_input_bundle(path).map(ProfileInputs::DependableElement)
        }
    }
}

pub fn run_profile(profile: Profile, inputs: &ProfileInputs) -> Result<ProfileRun, String> {
    match (profile, inputs) {
        (Profile::ArchitecturalDesign, ProfileInputs::ArchitecturalDesign(inputs)) => {
            architectural_design::run(inputs)
        }
        (Profile::DependableElement, ProfileInputs::DependableElement(inputs)) => {
            dependable_element::run(inputs)
        }
        _ => Err(format!(
            "Input bundle does not match validation profile {}",
            profile.as_str()
        )),
    }
}
