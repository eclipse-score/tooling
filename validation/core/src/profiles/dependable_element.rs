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
    BazelArchitecture, BazelInput, ClassDiagramInputs, ClassEntityIndex,
    ComponentDiagramArchitecture, ComponentDiagramInputs,
};
use crate::readers::{BazelReader, ClassDiagramReader, ComponentDiagramReader};
use crate::validators::{
    validate_architectural_unit_class_design, validate_bazel_component, UnitDesignEntity,
};
use crate::ValidationResult;
use serde::Deserialize;
use std::collections::BTreeMap;

use super::profile::{merge_results, read_and_convert, ProfileRun};

type ProfileValidator<'a> = Box<dyn Fn() -> Option<ValidationResult> + 'a>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependableElementInputs {
    architecture: String,
    #[serde(default)]
    component_diagrams: Vec<String>,
    #[serde(default)]
    unit_design_class_diagrams: Option<BTreeMap<String, Vec<String>>>,
}

fn registered_validators<'a>(
    bazel: &'a Option<BazelArchitecture>,
    component: &'a Option<ComponentDiagramArchitecture>,
    unit_design_classes: &'a Option<BTreeMap<String, Vec<UnitDesignEntity>>>,
) -> Vec<ProfileValidator<'a>> {
    vec![
        Box::new(move || {
            let (bazel, component) = (bazel.as_ref()?, component.as_ref()?);
            Some(validate_bazel_component(bazel, component))
        }),
        Box::new(move || {
            let (bazel, component, unit_design_classes) = (
                bazel.as_ref()?,
                component.as_ref()?,
                unit_design_classes.as_ref()?,
            );
            Some(validate_architectural_unit_class_design(
                bazel,
                component,
                unit_design_classes,
            ))
        }),
    ]
}

pub fn run(inputs: &DependableElementInputs) -> Result<ProfileRun, String> {
    let mut result = ValidationResult::default();
    let bazel = read_and_convert::<BazelReader, BazelArchitecture>(
        &inputs.architecture,
        &mut result,
        |raw: BazelInput, errs| raw.to_bazel_architecture(errs),
    )?;
    let component = read_and_convert::<ComponentDiagramReader, ComponentDiagramArchitecture>(
        inputs.component_diagrams.as_slice(),
        &mut result,
        |raw: ComponentDiagramInputs, errs| raw.to_diagram_architecture(errs),
    )?;
    let unit_design_classes = match &inputs.unit_design_class_diagrams {
        Some(diagrams_by_label) => {
            let mut entity_ids_by_label = BTreeMap::new();
            for (unit_label, paths) in diagrams_by_label {
                let diagrams = read_and_convert::<ClassDiagramReader, ClassDiagramInputs>(
                    paths.as_slice(),
                    &mut result,
                    |raw: ClassDiagramInputs, _errs| raw,
                )?
                .unwrap_or_default();
                let index = ClassEntityIndex::build_index(&diagrams, &mut result);
                let entities = index
                    .entities()
                    .filter(|entity| is_unit_refinement_entity(entity.entity_type))
                    .map(|entity| UnitDesignEntity {
                        id: entity.id.clone(),
                        source_location: entity.source_location.clone(),
                    })
                    .collect();
                entity_ids_by_label.insert(unit_label.clone(), entities);
            }
            Some(entity_ids_by_label)
        }
        None => None,
    };

    let validators = registered_validators(&bazel, &component, &unit_design_classes);

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

fn is_unit_refinement_entity(entity_type: class_diagram::EntityType) -> bool {
    matches!(
        entity_type,
        class_diagram::EntityType::Class
            | class_diagram::EntityType::Struct
            | class_diagram::EntityType::Interface
            | class_diagram::EntityType::AbstractClass
    )
}

#[cfg(test)]
mod tests {
    use super::is_unit_refinement_entity;
    use class_diagram::EntityType;

    #[test]
    fn unit_refinement_accepts_class_like_entities_but_not_enums() {
        for entity_type in [
            EntityType::Class,
            EntityType::Struct,
            EntityType::Interface,
            EntityType::AbstractClass,
        ] {
            assert!(is_unit_refinement_entity(entity_type));
        }

        assert!(!is_unit_refinement_entity(EntityType::Enum));
    }
}
