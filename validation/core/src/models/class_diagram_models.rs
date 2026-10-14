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

use class_diagram::{
    ClassDiagram as ClassDiagramInput, EntityType, FreeFunctionDecl, SimpleEntity,
};
use source_location::SourceLocation;

use crate::{ErrorBuilder, ErrorCategory, ValidationResult};

/// Collection of class diagrams loaded from one or more FlatBuffer files.
pub type ClassDiagramInputs = Vec<ClassDiagramInput>;

#[derive(Clone, Copy)]
enum ClassInputKind {
    Design,
    Implementation,
}

/// Class-like entities from one or more class diagrams, keyed by lower-case id.
pub struct ClassEntityIndex {
    entities: BTreeMap<String, SimpleEntity>,
    free_functions: Vec<FreeFunctionDecl>,
}

impl ClassEntityIndex {
    /// Builds an index from class design diagrams.
    pub fn build_diagram_index(
        diagrams: &[ClassDiagramInput],
        result: &mut ValidationResult,
    ) -> Self {
        Self::build_index(diagrams, result, ClassInputKind::Design)
    }

    /// Builds an index from C++ implementation diagrams.
    pub fn build_implementation_index(
        diagrams: &[ClassDiagramInput],
        result: &mut ValidationResult,
    ) -> Self {
        Self::build_index(diagrams, result, ClassInputKind::Implementation)
    }

    fn build_index(
        diagrams: &[ClassDiagramInput],
        result: &mut ValidationResult,
        input_kind: ClassInputKind,
    ) -> Self {
        let mut entities: BTreeMap<String, SimpleEntity> = BTreeMap::new();
        let mut free_functions = Vec::new();
        let (source_kind, ignore_same_source_location) = match input_kind {
            ClassInputKind::Design => ("design", false),
            ClassInputKind::Implementation => ("implementation", true),
        };

        for diagram in diagrams {
            for entity in &diagram.entities {
                let indexed_entity = entity.clone();

                let key = indexed_entity.id.to_lowercase();
                if let Some(prev) = entities.get(&key) {
                    if ignore_same_source_location
                        && prev.source_location == indexed_entity.source_location
                    {
                        continue;
                    }

                    let (first_source_file, _) = prev.source_location.display();
                    let (second_source_file, _) = indexed_entity.source_location.display();

                    result.add_failure(
                        ErrorBuilder::new(ErrorCategory::Class)
                            .title(format!(
                                "class \"{}\" is defined more than once in the class {}.",
                                prev.id,
                                source_kind,
                            ))
                            .field("class", format!("\"{}\"", prev.id))
                            .field(
                                format!("{source_kind} source file"),
                                format!("\"{}\"", first_source_file),
                            )
                            .field(
                                format!("{source_kind} source line"),
                                prev.source_location.line.to_string(),
                            )
                            .field(
                                "duplicate source file",
                                format!("\"{}\"", second_source_file),
                            )
                            .field(
                                "duplicate source line",
                                indexed_entity.source_location.line.to_string(),
                            )
                            .fix(format!(
                                "remove or rename one of the duplicate class \"{}\" declarations in the class {}",
                                prev.id,
                                source_kind,
                            ))
                            .build(),
                    );
                } else {
                    entities.insert(key, indexed_entity);
                }
            }

            free_functions.extend(diagram.free_functions.iter().cloned());
        }

        Self {
            entities,
            free_functions,
        }
    }

    pub fn entities(&self) -> impl Iterator<Item = &SimpleEntity> + '_ {
        self.entities.values()
    }

    pub fn find_by_id(&self, id: &str) -> Option<&SimpleEntity> {
        self.entities.get(&id.to_lowercase())
    }

    pub fn free_functions(&self) -> impl Iterator<Item = &FreeFunctionDecl> + '_ {
        self.free_functions.iter()
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
    api_names: BTreeSet<String>, // interface id leaves
}

impl PublicApiIndex {
    /// Build a [`PublicApiIndex`] from public-API class diagram inputs.
    pub fn build_index(diagrams: &[ClassDiagramInput]) -> Self {
        let mut api_names: BTreeSet<String> = BTreeSet::new();

        for diagram in diagrams {
            for entity in &diagram.entities {
                if entity.entity_type != EntityType::Interface {
                    continue;
                }

                let leaf = entity.id.rsplit('.').next().unwrap_or(&entity.id);
                api_names.insert(leaf.to_string());
            }
        }

        Self { api_names }
    }

    pub fn api_names(&self) -> impl Iterator<Item = &String> + '_ {
        self.api_names.iter()
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
        let _index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains(
            "[Class] Class \"Unit.Sample\" is defined more than once in the class design."
        ));
    }

    #[test]
    fn implementation_class_entity_index_ignores_same_source_location() {
        let diagrams = vec![ClassDiagram {
            name: "implementation".to_string(),
            entities: vec![
                entity("Unit.Sample", "implementation.h", 12),
                entity("unit.sample", "implementation.h", 12),
            ],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let index = ClassEntityIndex::build_implementation_index(&diagrams, &mut result);

        assert!(result.failures.is_empty());
        assert_eq!(index.entities().count(), 1);
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
}
