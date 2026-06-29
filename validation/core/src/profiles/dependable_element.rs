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

use crate::models::{
    BazelArchitecture, BazelInput, ComponentDiagramArchitecture, ComponentDiagramInputs, Errors,
};
use crate::readers::{BazelReader, ComponentDiagramReader};
use crate::validators::validate_bazel_component;
use serde::Deserialize;

use super::profile::{merge_errors, read_and_convert, ProfileRun};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependableElementInputs {
    architecture: String,
    #[serde(default)]
    component_diagrams: Vec<String>,
}

pub fn run(inputs: &DependableElementInputs) -> Result<ProfileRun, String> {
    let mut errors = Errors::default();
    let bazel = read_and_convert::<BazelReader, BazelArchitecture>(
        &inputs.architecture,
        &mut errors,
        |raw: BazelInput, errs| raw.to_bazel_architecture(errs),
    )?;
    let component = read_and_convert::<ComponentDiagramReader, ComponentDiagramArchitecture>(
        inputs.component_diagrams.as_slice(),
        &mut errors,
        |raw: ComponentDiagramInputs, errs| raw.to_diagram_architecture(errs),
    )?;

    let mut ran_validator = false;
    if let (Some(bazel), Some(component)) = (bazel.as_ref(), component.as_ref()) {
        merge_errors(
            &mut errors,
            validate_bazel_component(bazel, component, Errors::default()),
        );
        ran_validator = true;
    }

    Ok(ProfileRun {
        ran_validator,
        errors,
    })
}
