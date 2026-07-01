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

use crate::models::{ClassDiagramInputs, ClassEntityIndex};
use crate::readers::ClassDiagramReader;
use crate::validators::validate_class_design_implementation;
use crate::ValidationResult;
use serde::Deserialize;

use super::profile::{merge_results, read_and_convert, ProfileRun};

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UnitInputs {
    design_classes: Vec<String>,
    implementation_classes: Vec<String>,
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

    let mut ran_validator = false;
    if let (Some(design_classes), Some(implementation_classes)) =
        (design_classes.as_ref(), implementation_classes.as_ref())
    {
        merge_results(
            &mut result,
            validate_class_design_implementation(design_classes, implementation_classes),
        );
        ran_validator = true;
    }

    Ok(ProfileRun {
        ran_validator,
        result,
    })
}
