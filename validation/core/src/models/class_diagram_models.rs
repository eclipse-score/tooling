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

use std::collections::BTreeSet;

use class_diagram::{ClassDiagram as ClassDiagramInput, EntityType};

use super::Errors;

/// Collection of class diagrams loaded from one or more FlatBuffer files.
pub type ClassDiagramInputs = Vec<ClassDiagramInput>;

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
    pub fn build_index(diagrams: &[ClassDiagramInput], _errors: &mut Errors) -> Self {
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
    use class_diagram::{ClassDiagram, Method, SimpleEntity, Visibility};

    fn method(name: &str) -> Method {
        Method {
            name: name.to_string(),
            return_type: None,
            visibility: Visibility::Public,
            parameters: Vec::new(),
            template_parameters: None,
            modifiers: Vec::new(),
        }
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
                    source_file: None,
                    source_line: None,
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
                    source_file: None,
                    source_line: None,
                },
            ],
            relationships: Vec::new(),
            source_files: Vec::new(),
            version: None,
        }];

        let mut errors = Errors::default();
        let index = InternalApiIndex::build_index(&diagrams, &mut errors);

        assert!(errors.is_empty());
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
                    source_file: None,
                    source_line: None,
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
                    source_file: None,
                    source_line: None,
                },
            ],
            relationships: Vec::new(),
            source_files: Vec::new(),
            version: None,
        }];

        let mut errors = Errors::default();
        let index = InternalApiIndex::build_index(&diagrams, &mut errors);

        assert!(errors.is_empty());
        let interface_ids: BTreeSet<&str> = index
            .interfaces()
            .map(|interface| interface.id.as_str())
            .collect();

        assert_eq!(interface_ids.len(), 2);
        assert!(interface_ids.contains("InternalAPI.InternalInterfaceA"));
        assert!(interface_ids.contains("InternalAPI.InternalInterfaceB"));
    }
}
