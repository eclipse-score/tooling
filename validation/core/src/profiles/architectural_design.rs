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
    ClassDiagramInputs, ComponentDiagramArchitecture, ComponentDiagramInputs, InternalApiIndex,
    SequenceDiagramIndex, SequenceDiagramInputs,
};
use crate::readers::{ClassDiagramReader, ComponentDiagramReader, SequenceDiagramReader};
use crate::validators::{
    validate_component_internal_api, validate_component_sequence, validate_sequence_internal_api,
};
use crate::ValidationResult;
use serde::Deserialize;

use super::profile::{merge_results, read_and_convert, ProfileRun};

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArchitecturalDesignInputs {
    component_diagrams: Vec<String>,
    sequence_diagrams: Vec<String>,
    internal_api_diagrams: Vec<String>,
    public_api_diagrams: Vec<String>,
}

pub fn run(inputs: &ArchitecturalDesignInputs) -> Result<ProfileRun, String> {
    let mut result = ValidationResult::default();
    let component = read_and_convert::<ComponentDiagramReader, ComponentDiagramArchitecture>(
        inputs.component_diagrams.as_slice(),
        &mut result,
        |raw: ComponentDiagramInputs, errs| raw.to_diagram_architecture(errs),
    )?;
    let sequence = read_and_convert::<SequenceDiagramReader, SequenceDiagramIndex>(
        inputs.sequence_diagrams.as_slice(),
        &mut result,
        |raw: SequenceDiagramInputs, errs| raw.to_sequence_diagram_index(errs),
    )?;
    let internal_api = read_and_convert::<ClassDiagramReader, InternalApiIndex>(
        inputs.internal_api_diagrams.as_slice(),
        &mut result,
        |raw: ClassDiagramInputs, errs| InternalApiIndex::build_index(&raw, errs),
    )?;

    let mut ran_validator = false;
    if let (Some(component), Some(sequence)) = (component.as_ref(), sequence.as_ref()) {
        merge_results(
            &mut results,
            validate_component_sequence(component, sequence, Errors::default()),
        );
        ran_validator = true;
    }
    if let (Some(component), Some(internal_api)) = (component.as_ref(), internal_api.as_ref()) {
        merge_results(
            &mut results,
            validate_component_internal_api(component, internal_api, Errors::default()),
        );
        ran_validator = true;
    }
    if let (Some(sequence), Some(internal_api)) = (sequence.as_ref(), internal_api.as_ref()) {
        merge_results(
            &mut results,
            validate_sequence_internal_api(
                sequence,
                internal_api,
                component.as_ref(),
                Errors::default(),
            ),
        );
        ran_validator = true;
    }

    Ok(ProfileRun {
        ran_validator,
        result,
    })
}
