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

//! Reader for class-diagram FlatBuffer exports used by design verification.

use std::fs;

use class_fbs::class_metamodel as fb_class;

use crate::models::{
    ClassDiagramEntityInput, ClassDiagramInput, ClassDiagramInputs, ClassDiagramRelationshipInput,
};
use crate::readers::Reader;

pub struct ClassDiagramReader;

impl ClassDiagramReader {
    /// Read all class-diagram files and convert them into validation-friendly
    /// Rust models.
    pub fn read(paths: &[String]) -> Result<ClassDiagramInputs, String> {
        let mut diagrams = Vec::new();

        for path in paths {
            let data = fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;

            let diagram = flatbuffers::root::<fb_class::ClassDiagram>(&data)
                .map_err(|e| format!("Failed to parse class FlatBuffer {path}: {e}"))?;

            let mut entities = Vec::new();
            if let Some(raw_entities) = diagram.entities() {
                for entity in raw_entities.iter() {
                    // Rehydrate repeated FlatBuffer string vectors into owned
                    // Rust values so validators can work without borrow/lifetime
                    // coupling to the underlying buffer.
                    let template_params = entity
                        .template_parameters()
                        .map(|values| values.iter().map(|p| p.to_string()).collect::<Vec<_>>())
                        .unwrap_or_default();

                    entities.push(ClassDiagramEntityInput {
                        id: entity.id().to_string(),
                        name: Some(entity.name().to_string()),
                        alias: None,
                        parent_id: entity.enclosing_namespace_id().map(|s| s.to_string()),
                        entity_type: format!("{:?}", entity.entity_type()),
                        stereotypes: Vec::new(),
                        template_params,
                        source_file: entity.source_file().map(|s| s.to_string()),
                        source_line: entity.source_line(),
                    });
                }
            }

            let mut relationships = Vec::new();
            if let Some(raw_rels) = diagram.relationships() {
                for rel in raw_rels.iter() {
                    relationships.push(ClassDiagramRelationshipInput {
                        source: rel.source().to_string(),
                        target: rel.target().to_string(),
                        relation_type: format!("{:?}", rel.relation_type()),
                        label: None,
                        stereotype: None,
                        source_multiplicity: rel.source_multiplicity().map(|s| s.to_string()),
                        target_multiplicity: rel.target_multiplicity().map(|s| s.to_string()),
                        source_role: None,
                        target_role: None,
                    });
                }
            }

            let source_files = diagram
                .source_files()
                .map(|values| values.iter().map(|f| f.to_string()).collect::<Vec<_>>())
                .unwrap_or_default();

            diagrams.push(ClassDiagramInput {
                name: diagram.name().to_string(),
                entities,
                relationships,
                source_files,
                version: diagram.version().map(|s| s.to_string()),
            });
        }

        Ok(ClassDiagramInputs { diagrams })
    }
}

impl Reader for ClassDiagramReader {
    type Input = [String];
    type Raw = ClassDiagramInputs;
    type Error = String;

    fn read(input: &Self::Input) -> Result<Self::Raw, Self::Error> {
        ClassDiagramReader::read(input)
    }
}
