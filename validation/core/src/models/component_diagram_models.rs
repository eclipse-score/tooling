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

use std::collections::BTreeMap;

use super::{EntityKey, Errors};

/// Supported component-diagram entity kinds needed for validation.
#[derive(Clone, PartialEq)]
pub enum ComponentDiagramElementType {
    Component,
    Package,
    Interface,
}

/// One relation attached to a component-diagram entity.
#[derive(Clone)]
pub struct ComponentDiagramRelation {
    pub target: String,
    #[allow(dead_code)]
    pub annotation: Option<String>,
    #[allow(dead_code)]
    pub relation_type: Option<String>,
    pub source_role: Option<String>,
}

/// A single component-level entity parsed from a PlantUML `.fbs.bin` file.
#[derive(Clone)]
pub struct ComponentDiagramInput {
    pub id: String,
    pub alias: Option<String>,
    pub parent_id: Option<String>,
    pub element_type: ComponentDiagramElementType,
    pub stereotype: Option<String>,
    pub relations: Vec<ComponentDiagramRelation>,
}

impl ComponentDiagramInput {
    /// Canonical match key: alias (lowercased) when present, otherwise raw id.
    pub fn match_key(&self) -> String {
        self.alias.as_deref().unwrap_or(&self.id).to_lowercase()
    }

    pub fn is_component(&self) -> bool {
        self.stereotype.as_deref() == Some("component")
    }

    pub fn is_unit(&self) -> bool {
        self.stereotype.as_deref() == Some("unit")
    }

    pub fn is_interface(&self) -> bool {
        self.element_type == ComponentDiagramElementType::Interface
    }

    /// Returns `true` for `<<SEooC>>` package entities (dependable elements).
    pub fn is_seooc_package(&self) -> bool {
        self.stereotype.as_deref() == Some("SEooC")
    }
}

/// Collection of raw PlantUML entities read from FlatBuffers files.
///
/// Symmetric peer of [`BazelInput`]: produced by [`ComponentDiagramReader`] and
/// consumed by [`to_diagram_architecture`](ComponentDiagramInputs::to_diagram_architecture).
pub struct ComponentDiagramInputs {
    pub entities: Vec<ComponentDiagramInput>,
}

impl ComponentDiagramInputs {
    /// Build a [`ComponentDiagramArchitecture`] index from these diagram inputs.
    pub fn to_diagram_architecture(&self, errors: &mut Errors) -> ComponentDiagramArchitecture {
        ComponentDiagramArchitecture::from_entities(&self.entities, errors)
    }
}

/// Indexed entity key-maps derived from the parsed PlantUML diagram entities.
///
/// Built via [`ComponentDiagramInputs::to_diagram_architecture`].
pub struct ComponentDiagramArchitecture {
    /// `<<SEooC>>` package entities, keyed with `parent = None`.
    pub seooc_set: BTreeMap<EntityKey, ComponentDiagramInput>,
    /// `<<component>>` entities, keyed with `parent = Some(..)`.
    pub comp_set: BTreeMap<EntityKey, ComponentDiagramInput>,
    pub unit_set: BTreeMap<EntityKey, ComponentDiagramInput>,
    /// Full raw entity list, kept for debug output.
    pub entities: Vec<ComponentDiagramInput>,
    pub filtered_seooc_count: usize,
    pub filtered_component_count: usize,
    pub filtered_unit_count: usize,
}

impl ComponentDiagramArchitecture {
    /// Index `entities` by stereotype and parent alias.
    ///
    /// `<<SEooC>>` go into `seooc_set`;
    /// `<<component>>` go into `comp_set`;
    /// `<<unit>>` go into `unit_set`.
    /// Duplicates (same [`EntityKey`]) are reported via `errors`.
    fn from_entities(entities: &[ComponentDiagramInput], errors: &mut Errors) -> Self {
        // Index by raw id for parent resolution; PlantUML nesting uses id,
        // not alias.
        let mut id_index: BTreeMap<String, &ComponentDiagramInput> = BTreeMap::new();
        for entity in entities {
            let key = entity.id.to_lowercase();
            if let Some(prev) = id_index.insert(key.clone(), entity) {
                errors.push(format!(
                    "Duplicate entity ID in PlantUML diagram (case-insensitive):\n\
                       ID : {key:?}\n\
                       IDs: {} and {}",
                    prev.id, entity.id
                ));
            }
        }

        let seoocs: Vec<&ComponentDiagramInput> = entities
            .iter()
            .filter(|entity| entity.is_seooc_package())
            .collect();
        let components: Vec<&ComponentDiagramInput> = entities
            .iter()
            .filter(|entity| entity.is_component())
            .collect();
        let units: Vec<&ComponentDiagramInput> =
            entities.iter().filter(|entity| entity.is_unit()).collect();

        let filtered_seooc_count = seoocs.len();
        let filtered_component_count = components.len();
        let filtered_unit_count = units.len();

        let seooc_set = Self::build_set(&seoocs, &id_index, errors);
        let comp_set = Self::build_set(&components, &id_index, errors);
        let unit_set = Self::build_set(&units, &id_index, errors);

        Self {
            seooc_set,
            comp_set,
            unit_set,
            entities: entities.to_vec(),
            filtered_seooc_count,
            filtered_component_count,
            filtered_unit_count,
        }
    }

    fn build_set(
        items: &[&ComponentDiagramInput],
        id_index: &BTreeMap<String, &ComponentDiagramInput>,
        errors: &mut Errors,
    ) -> BTreeMap<EntityKey, ComponentDiagramInput> {
        let mut set = BTreeMap::new();
        for entity in items {
            let alias = entity.match_key();
            let parent_alias = match &entity.parent_id {
                Some(parent_id) => match id_index.get(&parent_id.to_lowercase()) {
                    Some(parent) => Some(parent.match_key()),
                    None => {
                        errors.push(format!(
                            "Unresolved parent_id in PlantUML diagram:\n\
                               Entity ID : {}\n\
                               Parent ID : {}\n\
                               Action    : Fix the parent reference or add the missing parent entity",
                            entity.id, parent_id
                        ));
                        None
                    }
                },
                None => None,
            };
            let key = (alias, parent_alias);
            if let Some(prev) = set.insert(key.clone(), (*entity).clone()) {
                errors.push(format!(
                    "Duplicate entity in PlantUML diagram:\n\
                       Key: {:?}\n\
                       IDs: {} and {}",
                    key, prev.id, entity.id
                ));
            }
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relation(target: &str) -> ComponentDiagramRelation {
        ComponentDiagramRelation {
            target: target.to_string(),
            annotation: None,
            relation_type: Some("None".to_string()),
            source_role: Some("None".to_string()),
        }
    }

    fn entity(
        id: &str,
        alias: Option<&str>,
        parent_id: Option<&str>,
        element_type: ComponentDiagramElementType,
        stereotype: Option<&str>,
        relations: Vec<ComponentDiagramRelation>,
    ) -> ComponentDiagramInput {
        ComponentDiagramInput {
            id: id.to_string(),
            alias: alias.map(str::to_string),
            parent_id: parent_id.map(str::to_string),
            element_type,
            stereotype: stereotype.map(str::to_string),
            relations,
        }
    }

    #[test]
    fn interfaces_and_relations_are_indexed_for_future_sequence_validation() {
        let inputs = ComponentDiagramInputs {
            entities: vec![
                entity(
                    "safety_software_seooc_example",
                    Some("safety_software_seooc_example"),
                    None,
                    ComponentDiagramElementType::Package,
                    Some("SEooC"),
                    Vec::new(),
                ),
                entity(
                    "safety_software_seooc_example.component_example",
                    Some("component_example"),
                    Some("safety_software_seooc_example"),
                    ComponentDiagramElementType::Component,
                    Some("component"),
                    Vec::new(),
                ),
                entity(
                    "safety_software_seooc_example.InternalInterface",
                    Some("InternalInterface"),
                    Some("safety_software_seooc_example"),
                    ComponentDiagramElementType::Interface,
                    None,
                    Vec::new(),
                ),
                entity(
                    "safety_software_seooc_example.component_example.unit_1",
                    Some("unit_1"),
                    Some("safety_software_seooc_example.component_example"),
                    ComponentDiagramElementType::Component,
                    Some("unit"),
                    vec![relation("safety_software_seooc_example.InternalInterface")],
                ),
            ],
        };

        let mut errors = Errors::default();
        let architecture = inputs.to_diagram_architecture(&mut errors);

        assert!(errors.is_empty());
        assert!(architecture
            .entities
            .iter()
            .find(|entity| entity.id == "safety_software_seooc_example.InternalInterface")
            .expect("expected interface entity")
            .is_interface());
        assert_eq!(
            architecture
                .entities
                .iter()
                .find(|entity| {
                    entity.id == "safety_software_seooc_example.component_example.unit_1"
                })
                .expect("expected unit entity")
                .relations[0]
                .target,
            "safety_software_seooc_example.InternalInterface"
        );
    }
}
