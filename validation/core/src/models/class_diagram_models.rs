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

//! Models for class-diagram FlatBuffer inputs used by design verification.

use std::collections::BTreeSet;

use super::Errors;

/// A single class-diagram entity such as a class, struct, enum, or interface.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDiagramEntityInput {
    pub id: String,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub parent_id: Option<String>,
    pub entity_type: String,
    pub stereotypes: Vec<String>,
    pub template_params: Vec<String>,
    pub source_file: Option<String>,
    pub source_line: u32,
}

/// A relationship edge between two class-diagram entities.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDiagramRelationshipInput {
    pub source: String,
    pub target: String,
    pub relation_type: String,
    pub label: Option<String>,
    pub stereotype: Option<String>,
    pub source_multiplicity: Option<String>,
    pub target_multiplicity: Option<String>,
    pub source_role: Option<String>,
    pub target_role: Option<String>,
}

/// One parsed class diagram, including entities, containers, and
/// relationships.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDiagramInput {
    pub name: String,
    pub entities: Vec<ClassDiagramEntityInput>,
    pub relationships: Vec<ClassDiagramRelationshipInput>,
    pub source_files: Vec<String>,
    pub version: Option<String>,
}

/// Collection of class diagrams loaded from one or more FlatBuffer files.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDiagramInputs {
    pub diagrams: Vec<ClassDiagramInput>,
}

impl ClassDiagramInputs {
    /// Build a [`ClassDiagramIndex`] from class diagram inputs.
    pub fn to_class_diagram_index(&self, _errors: &mut Errors) -> ClassDiagramIndex {
        let observed_namespace_names = self
            .diagrams
            .iter()
            .flat_map(|diagram| diagram.entities.iter())
            .filter_map(|entity| entity.parent_id.clone())
            .filter(|parent_id| !parent_id.is_empty())
            .collect();

        ClassDiagramIndex {
            observed_namespace_names,
        }
    }
}

/// Indexed names derived from class-diagram entities.
#[derive(Clone)]
pub struct ClassDiagramIndex {
    pub observed_namespace_names: BTreeSet<String>,
}
