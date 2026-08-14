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
    ClassDiagramInputs, ClassEntityIndex, SequenceDiagramIndex, SequenceDiagramInputs,
};
use crate::readers::{ClassDiagramReader, SequenceDiagramReader};
use crate::validators::{validate_class_design_implementation, validate_class_design_sequence};
use crate::ValidationResult;
use serde::Deserialize;

use super::profile::{merge_results, read_and_convert, ProfileRun};

type ProfileValidator<'a> = Box<dyn Fn() -> Option<ValidationResult> + 'a>;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UnitInputs {
    design_classes: Vec<String>,
    sequence_diagrams: Vec<String>,
    implementation_classes: Vec<String>,
}

fn registered_validators<'a>(
    design_classes: &'a Option<ClassEntityIndex>,
    sequence_diagrams: &'a Option<SequenceDiagramIndex>,
    implementation_classes: &'a Option<ClassEntityIndex>,
) -> Vec<ProfileValidator<'a>> {
    vec![
        Box::new(move || {
            let (design_classes, implementation_classes) =
                (design_classes.as_ref()?, implementation_classes.as_ref()?);
            Some(validate_class_design_implementation(
                design_classes,
                implementation_classes,
            ))
        }),
        Box::new(move || {
            let (design_classes, sequence_diagrams) =
                (design_classes.as_ref()?, sequence_diagrams.as_ref()?);
            Some(validate_class_design_sequence(
                design_classes,
                sequence_diagrams,
            ))
        }),
    ]
}

pub fn run(inputs: &UnitInputs) -> Result<ProfileRun, String> {
    let mut result = ValidationResult::default();
    let design_classes = read_and_convert::<ClassDiagramReader, ClassEntityIndex>(
        inputs.design_classes.as_slice(),
        &mut result,
        |raw: ClassDiagramInputs, errs| ClassEntityIndex::build_index(&raw, errs),
    )?;
    let implementation_classes = read_and_convert::<ClassDiagramReader, ClassEntityIndex>(
        inputs.implementation_classes.as_slice(),
        &mut result,
        |raw: ClassDiagramInputs, errs| ClassEntityIndex::build_index(&raw, errs),
    )?;
    let sequence_diagrams = read_and_convert::<SequenceDiagramReader, SequenceDiagramIndex>(
        inputs.sequence_diagrams.as_slice(),
        &mut result,
        |raw: SequenceDiagramInputs, errs| raw.to_sequence_diagram_index(errs),
    )?;

    let validators =
        registered_validators(&design_classes, &sequence_diagrams, &implementation_classes);

    let mut ran_validator = false;
    for validator in validators {
        if let Some(validator_result) = validator() {
            merge_results(&mut result, validator_result);
            ran_validator = true;
        }
    }

    Ok(ProfileRun {
        ran_validator,
        result,
    })
}
