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

//! Models for class-diagram and internal-API FlatBuffer inputs

use std::collections::{BTreeMap, BTreeSet};

use class_diagram::{ClassDiagram as ClassDiagramInput, EntityType, SimpleEntity};

use crate::ValidationResult;

/// Collection of class diagrams loaded from one or more FlatBuffer files.
pub type ClassDiagramInputs = Vec<ClassDiagramInput>;

/// Class-like entities from one or more class diagrams, keyed by lower-case id.
pub struct ClassEntityIndex {
    entities: BTreeMap<String, SimpleEntity>,
}

impl ClassEntityIndex {
    /// Build an index from class diagrams for class implementation validation.
    pub fn build_index(diagrams: &[ClassDiagramInput], result: &mut ValidationResult) -> Self {
        let mut entities: BTreeMap<String, SimpleEntity> = BTreeMap::new();

        for diagram in diagrams {
            for entity in &diagram.entities {
                let indexed_entity = entity.clone();

                let key = indexed_entity.id.to_lowercase();
                if let Some(prev) = entities.get(&key) {
                    result.add_failure(format!(
                        "Duplicate class entity in validation input:\n\
                           Key             : {key}\n\
                           First location  : {}\n\
                           Second location : {}",
                        prev.source_location, indexed_entity.source_location
                    ));
                } else {
                    entities.insert(key, indexed_entity);
                }
            }
        }

        Self { entities }
    }

    pub fn entities(&self) -> impl Iterator<Item = &SimpleEntity> + '_ {
        self.entities.values()
    }

    pub fn find_by_id(&self, id: &str) -> Option<&SimpleEntity> {
        self.entities.get(&id.to_lowercase())
    }
}

/// Indexed internal-API data prepared for interface and method validators.
pub struct InternalApiInterface {
    pub id: String,
    pub method_names: BTreeSet<String>,
}

/// Indexed internal-API data prepared for validators.
pub struct InternalApiIndex {
    interfaces: Vec<InternalApiInterface>,
}

impl InternalApiIndex {
    /// Build an [`InternalApiIndex`] from internal-API diagram inputs.
    pub fn build_index(diagrams: &[ClassDiagramInput], _result: &mut ValidationResult) -> Self {
        let mut interfaces = Vec::new();

        for diagram in diagrams {
            for entity in &diagram.entities {
                if entity.entity_type != EntityType::Interface {
                    continue;
                }

                let interface = InternalApiInterface {
                    id: entity.id.clone(),
                    method_names: entity
                        .methods
                        .iter()
                        .map(|method| method.name.clone())
                        .filter(|name| !name.is_empty())
                        .collect(),
                };

                interfaces.push(interface);
            }
        }

        Self { interfaces }
    }

    pub fn interfaces(&self) -> impl Iterator<Item = &InternalApiInterface> + '_ {
        self.interfaces.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use class_diagram::{ClassDiagram, Method, SimpleEntity, SourceLocation, Visibility};

    fn method(name: &str) -> Method {
        Method {
            name: name.to_string(),
            return_type: None,
            source_location: SourceLocation::new("test.puml", 1),
            visibility: Visibility::Public,
            parameters: Vec::new(),
            template_parameters: None,
            modifiers: Vec::new(),
        }
    }

    fn entity(id: &str, source_file: &str, source_line: u32) -> SimpleEntity {
        SimpleEntity {
            id: id.to_string(),
            name: id.rsplit('.').next().unwrap_or(id).to_string(),
            enclosing_namespace_id: None,
            entity_type: EntityType::Class,
            type_aliases: Vec::new(),
            variables: Vec::new(),
            methods: Vec::new(),
            template_parameters: None,
            enum_literals: Vec::new(),
            relationships: Vec::new(),
            source_location: SourceLocation::new(source_file, source_line),
        }
    }

    #[test]
    fn class_entity_index_reports_duplicate_source_locations() {
        let diagrams = vec![ClassDiagram {
            name: "classes".to_string(),
            entities: vec![
                entity("Unit.Sample", "design_a.puml", 12),
                entity("unit.sample", "design_b.puml", 34),
            ],
            relationships: Vec::new(),
            source_files: Vec::new(),
            version: None,
        }];

        let mut result = ValidationResult::default();
        let _index = ClassEntityIndex::build_index(&diagrams, &mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("Key             : unit.sample"));
        assert!(result.failures[0].contains("First location  : design_a.puml:12"));
        assert!(result.failures[0].contains("Second location : design_b.puml:34"));
    }

    #[test]
    fn class_entity_index_reports_duplicate_source_locations_with_distinct_files() {
        let diagrams = vec![ClassDiagram {
            name: "classes".to_string(),
            entities: vec![
                entity("Unit.Sample", "design_left.puml", 1),
                entity("unit.sample", "design_right.puml", 2),
            ],
            relationships: Vec::new(),
            source_files: Vec::new(),
            version: None,
        }];

        let mut result = ValidationResult::default();
        let _index = ClassEntityIndex::build_index(&diagrams, &mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("First location  : design_left.puml:1"));
        assert!(result.failures[0].contains("Second location : design_right.puml:2"));
    }

    #[test]
    fn internal_api_index_collects_interfaces_and_methods() {
        let diagrams = vec![ClassDiagram {
            name: "internal_api".to_string(),
            entities: vec![
                SimpleEntity {
                    id: "InternalAPI.InternalInterface".to_string(),
                    name: "InternalInterface".to_string(),
                    enclosing_namespace_id: Some("InternalAPI".to_string()),
                    entity_type: EntityType::Interface,
                    type_aliases: Vec::new(),
                    variables: Vec::new(),
                    methods: vec![method("GetData")],
                    template_parameters: None,
                    enum_literals: Vec::new(),
                    relationships: Vec::new(),
                    source_location: SourceLocation::new("test.puml", 1),
                },
                SimpleEntity {
                    id: "InternalAPI.Helper".to_string(),
                    name: "Helper".to_string(),
                    enclosing_namespace_id: Some("InternalAPI".to_string()),
                    entity_type: EntityType::Class,
                    type_aliases: Vec::new(),
                    variables: Vec::new(),
                    methods: vec![method("IgnoreMe")],
                    template_parameters: None,
                    enum_literals: Vec::new(),
                    relationships: Vec::new(),
                    source_location: SourceLocation::new("test.puml", 1),
                },
            ],
            relationships: Vec::new(),
            source_files: Vec::new(),
            version: None,
        }];

        let mut result = ValidationResult::default();
        let index = InternalApiIndex::build_index(&diagrams, &mut result);

        assert!(result.is_empty());
        assert!(index
            .interfaces()
            .find(|interface| interface.id == "InternalAPI.InternalInterface")
            .expect("expected interface entry")
            .method_names
            .contains("GetData"));
        assert!(index
            .interfaces()
            .all(|interface| interface.id != "InternalAPI.Helper"));
    }

    #[test]
    fn internal_api_index_keeps_distinct_interface_ids() {
        let diagrams = vec![ClassDiagram {
            name: "internal_api".to_string(),
            entities: vec![
                SimpleEntity {
                    id: "InternalAPI.InternalInterfaceA".to_string(),
                    name: "InternalInterface".to_string(),
                    enclosing_namespace_id: Some("InternalAPI".to_string()),
                    entity_type: EntityType::Interface,
                    type_aliases: Vec::new(),
                    variables: Vec::new(),
                    methods: vec![method("GetData")],
                    template_parameters: None,
                    enum_literals: Vec::new(),
                    relationships: Vec::new(),
                    source_location: SourceLocation::new("test.puml", 1),
                },
                SimpleEntity {
                    id: "InternalAPI.InternalInterfaceB".to_string(),
                    name: "InternalInterface".to_string(),
                    enclosing_namespace_id: Some("InternalAPI".to_string()),
                    entity_type: EntityType::Interface,
                    type_aliases: Vec::new(),
                    variables: Vec::new(),
                    methods: vec![method("GetData1")],
                    template_parameters: None,
                    enum_literals: Vec::new(),
                    relationships: Vec::new(),
                    source_location: SourceLocation::new("test.puml", 1),
                },
            ],
            relationships: Vec::new(),
            source_files: Vec::new(),
            version: None,
        }];

        let mut result = ValidationResult::default();
        let index = InternalApiIndex::build_index(&diagrams, &mut result);

        assert!(result.is_empty());
        let interface_ids: BTreeSet<&str> = index
            .interfaces()
            .map(|interface| interface.id.as_str())
            .collect();

        assert_eq!(interface_ids.len(), 2);
        assert!(interface_ids.contains("InternalAPI.InternalInterfaceA"));
        assert!(interface_ids.contains("InternalAPI.InternalInterfaceB"));
    }
}
