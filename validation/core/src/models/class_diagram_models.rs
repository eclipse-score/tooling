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
use source_location::SourceLocation;

use crate::validators::shared::display_entity_name;
use crate::{ErrorBuilder, ErrorCategory, ValidationResult};

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
                let current_entity = entity.clone();

                let normalized_entity_id = current_entity.id.to_lowercase();
                if let Some(existing_entity) = entities.get(&normalized_entity_id) {
                    let (first_source_file, _) = existing_entity.source_location.display();
                    let (second_source_file, _) = current_entity.source_location.display();

                    result.add_failure(
                        ErrorBuilder::new(ErrorCategory::Class)
                            .title(format!(
                                "class \"{}\" is defined more than once in the class diagram.",
                                display_entity_name(existing_entity),
                            ))
                            .field(
                                "class",
                                format!("\"{}\"", display_entity_name(existing_entity)),
                            )
                            .field(
                                "design source file",
                                format!("\"{}\"", first_source_file),
                            )
                            .field(
                                "design source line",
                                existing_entity.source_location.line.to_string(),
                            )
                            .field(
                                "duplicate source file",
                                format!("\"{}\"", second_source_file),
                            )
                            .field(
                                "duplicate source line",
                                current_entity.source_location.line.to_string(),
                            )
                            .fix(format!(
                                "remove or rename one of the duplicate class \"{}\" declarations in the class diagram",
                                display_entity_name(existing_entity),
                            ))
                            .build(),
                    );
                } else {
                    entities.insert(normalized_entity_id, current_entity);
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
    pub source_location: SourceLocation,
}

/// Indexed internal-API data prepared for validators.
pub struct InternalApiIndex {
    interfaces: Vec<InternalApiInterface>,
}

impl InternalApiIndex {
    /// Build an [`InternalApiIndex`] from internal-API diagram inputs.
    pub fn build_index(diagrams: &[ClassDiagramInput]) -> Self {
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
                    source_location: entity.source_location.clone(),
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

/// Indexed public-API data prepared for component/public-API validators.
pub struct PublicApiIndex {
    api_ids: BTreeSet<String>,
}

impl PublicApiIndex {
    /// Build a [`PublicApiIndex`] from public-API class diagram inputs.
    pub fn build_index(diagrams: &[ClassDiagramInput]) -> Self {
        let mut api_ids = BTreeSet::new();

        for diagram in diagrams {
            for entity in &diagram.entities {
                if entity.entity_type != EntityType::Interface {
                    continue;
                }

                api_ids.insert(entity.id.clone());
            }
        }

        Self { api_ids }
    }

    pub fn api_ids(&self) -> impl Iterator<Item = &String> + '_ {
        self.api_ids.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use class_diagram::{ClassDiagram, Method, SimpleEntity, Visibility};
    use source_location::SourceLocation;

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
            stereotypes: Vec::new(),
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
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let _index = ClassEntityIndex::build_index(&diagrams, &mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0]
            .contains("[Class] Class \"Sample\" is defined more than once in the class diagram."));
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
                    stereotypes: Vec::new(),
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
                    stereotypes: Vec::new(),
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
            free_functions: Vec::new(),
        }];

        let index = InternalApiIndex::build_index(&diagrams);

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
                    stereotypes: Vec::new(),
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
                    stereotypes: Vec::new(),
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
            free_functions: Vec::new(),
        }];

        let index = InternalApiIndex::build_index(&diagrams);

        let interface_ids: BTreeSet<&str> = index
            .interfaces()
            .map(|interface| interface.id.as_str())
            .collect();

        assert_eq!(interface_ids.len(), 2);
        assert!(interface_ids.contains("InternalAPI.InternalInterfaceA"));
        assert!(interface_ids.contains("InternalAPI.InternalInterfaceB"));
    }

    #[test]
    fn public_api_index_keeps_distinct_interface_ids_even_when_names_match() {
        let diagrams = vec![ClassDiagram {
            name: "public_api".to_string(),
            entities: vec![
                SimpleEntity {
                    id: "ServiceA.SampleLibraryAPI".to_string(),
                    name: "SampleLibraryAPI".to_string(),
                    enclosing_namespace_id: Some("ServiceA".to_string()),
                    stereotypes: Vec::new(),
                    entity_type: EntityType::Interface,
                    type_aliases: Vec::new(),
                    variables: Vec::new(),
                    methods: Vec::new(),
                    template_parameters: None,
                    enum_literals: Vec::new(),
                    relationships: Vec::new(),
                    source_location: SourceLocation::new("test_a.puml", 1),
                },
                SimpleEntity {
                    id: "ServiceB.SampleLibraryAPI".to_string(),
                    name: "SampleLibraryAPI".to_string(),
                    enclosing_namespace_id: Some("ServiceB".to_string()),
                    stereotypes: Vec::new(),
                    entity_type: EntityType::Interface,
                    type_aliases: Vec::new(),
                    variables: Vec::new(),
                    methods: Vec::new(),
                    template_parameters: None,
                    enum_literals: Vec::new(),
                    relationships: Vec::new(),
                    source_location: SourceLocation::new("test_b.puml", 1),
                },
            ],
            free_functions: Vec::new(),
        }];

        let index = PublicApiIndex::build_index(&diagrams);

        let public_api_ids: BTreeSet<&str> = index.api_ids().map(String::as_str).collect();

        assert_eq!(public_api_ids.len(), 2);
        assert!(public_api_ids.contains("ServiceA.SampleLibraryAPI"));
        assert!(public_api_ids.contains("ServiceB.SampleLibraryAPI"));
    }
}
