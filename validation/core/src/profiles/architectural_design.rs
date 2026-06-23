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
    ClassDiagramInputs, ComponentDiagramArchitecture, ComponentDiagramInputs, Errors,
    InternalApiIndex, SequenceDiagramIndex, SequenceDiagramInputs,
};
use crate::readers::{ClassDiagramReader, ComponentDiagramReader, SequenceDiagramReader};
use crate::validators::validate_component_sequence;
use serde::Deserialize;

use super::profile::{merge_errors, read_and_convert, ProfileRun};

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArchitecturalDesignInputs {
    component_diagrams: Vec<String>,
    sequence_diagrams: Vec<String>,
    internal_api_diagrams: Vec<String>,
    public_api_diagrams: Vec<String>,
}

pub fn run(inputs: &ArchitecturalDesignInputs) -> Result<ProfileRun, String> {
    let mut errors = Errors::default();
    let component = read_and_convert::<ComponentDiagramReader, ComponentDiagramArchitecture>(
        inputs.component_diagrams.as_slice(),
        &mut errors,
        |raw: ComponentDiagramInputs, errs| raw.to_diagram_architecture(errs),
    )?;
    let sequence = read_and_convert::<SequenceDiagramReader, SequenceDiagramIndex>(
        inputs.sequence_diagrams.as_slice(),
        &mut errors,
        |raw: SequenceDiagramInputs, errs| raw.to_sequence_diagram_index(errs),
    )?;
    let internal_api = read_and_convert::<ClassDiagramReader, InternalApiIndex>(
        inputs.internal_api_diagrams.as_slice(),
        &mut errors,
        |raw: ClassDiagramInputs, errs| InternalApiIndex::build_index(&raw, errs),
    )?;

    let mut ran_validator = false;
    if let (Some(component), Some(sequence)) = (component.as_ref(), sequence.as_ref()) {
        merge_errors(
            &mut errors,
            validate_component_sequence(
                component,
                sequence,
                internal_api.as_ref(),
                Errors::default(),
            ),
        );
        ran_validator = true;
    }

    Ok(ProfileRun {
        ran_validator,
        errors,
    })
}
