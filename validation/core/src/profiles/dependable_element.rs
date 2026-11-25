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

use crate::links::{diagram_links, DiagramFacts, DiagramLinks};
use crate::models::{
    BazelArchitecture, BazelInput, ComponentDiagramArchitecture, ComponentDiagramInputs,
    SequenceDiagramInputs, SequenceRefinementIndex,
};
use crate::readers::{BazelReader, ComponentDiagramReader, SequenceDiagramReader};
use crate::validators::{validate_bazel_component, validate_sequence_refinement};
use crate::ValidationResult;
use serde::Deserialize;

use super::profile::{merge_results, read_and_convert, ProfileRun};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependableElementInputs {
    architecture: String,
    #[serde(default)]
    component_diagrams: Vec<String>,
    /// Sequence diagrams of all architectural designs.
    #[serde(default)]
    sequence_diagrams: Vec<String>,
    /// Further diagrams that only take part in the links.
    #[serde(default)]
    link_diagrams: Vec<String>,
}

/// Zoom index of the sequence diagrams in the component architecture.
fn refinement_index(
    inputs: &DependableElementInputs,
    component: Option<&ComponentDiagramArchitecture>,
) -> Result<Option<SequenceRefinementIndex>, String> {
    let Some(component) = component else {
        return Ok(None);
    };
    read_and_convert::<SequenceDiagramReader, SequenceRefinementIndex>(
        inputs.sequence_diagrams.as_slice(),
        &mut ValidationResult::default(),
        |raw: SequenceDiagramInputs, _| {
            // Sequence model findings belong to the architectural-design profile.
            let sequence = raw.to_sequence_diagram_index(&mut ValidationResult::default());
            SequenceRefinementIndex::build(&raw.diagrams, &sequence, component)
        },
    )
}

fn component_architecture(
    inputs: &DependableElementInputs,
    result: &mut ValidationResult,
) -> Result<Option<ComponentDiagramArchitecture>, String> {
    read_and_convert::<ComponentDiagramReader, ComponentDiagramArchitecture>(
        inputs.component_diagrams.as_slice(),
        result,
        |raw: ComponentDiagramInputs, errs| raw.to_diagram_architecture(errs),
    )
}

pub fn run(inputs: &DependableElementInputs) -> Result<ProfileRun, String> {
    let mut result = ValidationResult::default();
    let bazel = read_and_convert::<BazelReader, BazelArchitecture>(
        &inputs.architecture,
        &mut result,
        |raw: BazelInput, errs| raw.to_bazel_architecture(errs),
    )?;
    let bazel = bazel.filter(|architecture| !architecture.seooc_set.is_empty());
    let component = component_architecture(inputs, &mut result)?;
    let refinement = refinement_index(inputs, component.as_ref())?;

    let mut ran_validator = false;
    if let (Some(bazel), Some(component)) = (bazel.as_ref(), component.as_ref()) {
        merge_results(&mut result, validate_bazel_component(bazel, component));
        ran_validator = true;
    }
    if let Some(refinement) = &refinement {
        merge_results(&mut result, validate_sequence_refinement(refinement));
        ran_validator = true;
    }

    Ok(ProfileRun {
        ran_validator,
        result,
    })
}

/// Links of all diagrams of the dependable element. Findings of the inputs
/// do not stop the links.
pub fn links(inputs: &DependableElementInputs) -> Result<DiagramLinks, String> {
    let mut ignored = ValidationResult::default();
    let component = component_architecture(inputs, &mut ignored)?;
    let refinement = refinement_index(inputs, component.as_ref())?;

    let mut paths: Vec<String> = Vec::new();
    for path in inputs
        .component_diagrams
        .iter()
        .chain(&inputs.sequence_diagrams)
        .chain(&inputs.link_diagrams)
    {
        if !paths.contains(path) {
            paths.push(path.clone());
        }
    }
    let facts = DiagramFacts::read_files(&paths)?;
    Ok(diagram_links(&facts, refinement.as_ref()))
}
