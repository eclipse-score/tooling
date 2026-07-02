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

use super::EntityKey;
use crate::ValidationResult;
pub use component_diagram::{
    ComponentRelationType, ComponentType, EndpointRole, LogicComponent, LogicRelation,
};

/// Validation-specific helpers for component metamodel entities.
pub trait LogicComponentExt {
    /// Canonical match key: alias (lowercased) when present, otherwise raw id.
    fn match_key(&self) -> String;

    fn is_component(&self) -> bool;

    fn is_unit(&self) -> bool;

    fn is_interface(&self) -> bool;

    /// Returns `true` for `<<SEooC>>` package entities (dependable elements).
    fn is_seooc_package(&self) -> bool;
}

impl LogicComponentExt for LogicComponent {
    fn match_key(&self) -> String {
        self.alias.as_deref().unwrap_or(&self.id).to_lowercase()
    }

    fn is_component(&self) -> bool {
        self.stereotype.as_deref() == Some("component")
    }

    fn is_unit(&self) -> bool {
        self.stereotype.as_deref() == Some("unit")
    }

    fn is_interface(&self) -> bool {
        self.element_type == ComponentType::Interface
    }

    fn is_seooc_package(&self) -> bool {
        self.stereotype.as_deref() == Some("SEooC")
    }
}

/// Collection of raw PlantUML entities read from FlatBuffers files.
///
/// Symmetric peer of [`BazelInput`]: produced by [`ComponentDiagramReader`] and
/// consumed by [`to_diagram_architecture`](ComponentDiagramInputs::to_diagram_architecture).
pub struct ComponentDiagramInputs {
    pub entities: Vec<LogicComponent>,
}

impl ComponentDiagramInputs {
    /// Build a [`ComponentDiagramArchitecture`] index from these diagram inputs.
    pub fn to_diagram_architecture(
        &self,
        result: &mut ValidationResult,
    ) -> ComponentDiagramArchitecture {
        ComponentDiagramArchitecture::from_entities(&self.entities, result)
    }
}

/// Indexed entity key-maps derived from the parsed PlantUML diagram entities.
///
/// Built via [`ComponentDiagramInputs::to_diagram_architecture`].
pub struct ComponentDiagramArchitecture {
    /// `<<SEooC>>` package entities, keyed with `parent = None`.
    pub seooc_set: BTreeMap<EntityKey, LogicComponent>,
    /// `<<component>>` entities, keyed with `parent = Some(..)`.
    pub comp_set: BTreeMap<EntityKey, LogicComponent>,
    pub unit_set: BTreeMap<EntityKey, LogicComponent>,
    /// Full raw entity list, kept for debug output.
    pub entities: Vec<LogicComponent>,
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
    /// Duplicates (same [`EntityKey`]) are reported via `result`.
    fn from_entities(entities: &[LogicComponent], result: &mut ValidationResult) -> Self {
        // Index by raw id for parent resolution; PlantUML nesting uses id,
        // not alias.
        let mut id_index: BTreeMap<String, &LogicComponent> = BTreeMap::new();
        for entity in entities {
            let key = entity.id.to_lowercase();
            if let Some(prev) = id_index.insert(key.clone(), entity) {
                result.add_failure(format!(
                    "Duplicate entity ID in PlantUML diagram (case-insensitive):\n\
                       ID : {key:?}\n\
                       IDs: {} and {}",
                    prev.id, entity.id
                ));
            }
        }

        let seoocs: Vec<&LogicComponent> = entities
            .iter()
            .filter(|entity| entity.is_seooc_package())
            .collect();
        let components: Vec<&LogicComponent> = entities
            .iter()
            .filter(|entity| entity.is_component())
            .collect();
        let units: Vec<&LogicComponent> =
            entities.iter().filter(|entity| entity.is_unit()).collect();

        let filtered_seooc_count = seoocs.len();
        let filtered_component_count = components.len();
        let filtered_unit_count = units.len();

        let seooc_set = Self::build_set(&seoocs, &id_index, result);
        let comp_set = Self::build_set(&components, &id_index, result);
        let unit_set = Self::build_set(&units, &id_index, result);

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
        items: &[&LogicComponent],
        id_index: &BTreeMap<String, &LogicComponent>,
        result: &mut ValidationResult,
    ) -> BTreeMap<EntityKey, LogicComponent> {
        let mut set = BTreeMap::new();
        for entity in items {
            let alias = entity.match_key();
            let parent_alias = match &entity.parent_id {
                Some(parent_id) => match id_index.get(&parent_id.to_lowercase()) {
                    Some(parent) => Some(parent.match_key()),
                    None => {
                        result.add_failure(format!(
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
                result.add_failure(format!(
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

    fn relation(target: &str) -> LogicRelation {
        LogicRelation {
            target: target.to_string(),
            annotation: None,
            relation_type: ComponentRelationType::Association,
            source_role: EndpointRole::None,
        }
    }

    fn entity(
        id: &str,
        alias: Option<&str>,
        parent_id: Option<&str>,
        element_type: ComponentType,
        stereotype: Option<&str>,
        relations: Vec<LogicRelation>,
    ) -> LogicComponent {
        LogicComponent {
            id: id.to_string(),
            name: alias.map(str::to_string),
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
                    ComponentType::Package,
                    Some("SEooC"),
                    Vec::new(),
                ),
                entity(
                    "safety_software_seooc_example.component_example",
                    Some("component_example"),
                    Some("safety_software_seooc_example"),
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                ),
                entity(
                    "safety_software_seooc_example.InternalInterface",
                    Some("InternalInterface"),
                    Some("safety_software_seooc_example"),
                    ComponentType::Interface,
                    None,
                    Vec::new(),
                ),
                entity(
                    "safety_software_seooc_example.component_example.unit_1",
                    Some("unit_1"),
                    Some("safety_software_seooc_example.component_example"),
                    ComponentType::Component,
                    Some("unit"),
                    vec![relation("safety_software_seooc_example.InternalInterface")],
                ),
            ],
        };

        let mut result = ValidationResult::default();
        let architecture = inputs.to_diagram_architecture(&mut result);

        assert!(result.is_empty());
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

    #[test]
    fn reports_duplicate_entity_id() {
        let inputs = ComponentDiagramInputs {
            entities: vec![
                entity(
                    "MyDE",
                    Some("my_de"),
                    None,
                    ComponentType::Package,
                    Some("SEooC"),
                    Vec::new(),
                ),
                entity(
                    "myDE",
                    Some("other_alias"),
                    None,
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                ),
            ],
        };

        let mut set_result = ValidationResult::default();
        let _architecture = inputs.to_diagram_architecture(&mut set_result);

        assert!(
            set_result
                .failures
                .iter()
                .any(|message| message.contains("Duplicate entity ID")),
            "Expected duplicate ID error, got: {:?}",
            set_result.failures
        );
    }

    #[test]
    fn reports_unresolved_parent_id() {
        let inputs = ComponentDiagramInputs {
            entities: vec![
                entity(
                    "MyDE",
                    Some("my_de"),
                    None,
                    ComponentType::Package,
                    Some("SEooC"),
                    Vec::new(),
                ),
                entity(
                    "CompA",
                    Some("comp_a"),
                    Some("NonExistent"),
                    ComponentType::Component,
                    Some("component"),
                    Vec::new(),
                ),
            ],
        };

        let mut set_result = ValidationResult::default();
        let _architecture = inputs.to_diagram_architecture(&mut set_result);

        assert!(
            set_result
                .failures
                .iter()
                .any(|message| message.contains("Unresolved parent_id")),
            "Expected unresolved parent error, got: {:?}",
            set_result.failures
        );
    }
}
