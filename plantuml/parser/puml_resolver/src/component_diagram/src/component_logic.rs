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
use serde::{Deserialize, Serialize};

// #[derive(Debug, Clone)]
// pub struct Package {
//     pub components: Vec<LogicComponent>,
// }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogicComponent {
    pub id: String, //FQN
    pub name: Option<String>,
    pub alias: Option<String>,
    pub parent_id: Option<String>,  // FQN of parent
    pub comp_type: ComponentType,   // e.g., package, component, etc.
    pub stereotype: Option<String>, // e.g., component, unit, etc.
    pub relations: Vec<LogicRelation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ComponentType {
    Artifact,
    Card,
    Cloud,
    Component,
    Database,
    File,
    Folder,
    Frame,
    Hexagon,
    Interface,
    Node,
    Package,
    Queue,
    Rectangle,
    Stack,
    Storage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogicRelation {
    pub target: String, // FQN
    pub annotation: Option<String>,
    pub relation_type: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ComponentResolverError {
    #[error("Component Resolver: UnresolvedReference: {reference}")]
    UnresolvedReference { reference: String },

    #[error("Duplicate component id: {component_id}")]
    DuplicateComponent { component_id: String },

    #[error("Unknown component type: {component_type}")]
    UnknownComponentType { component_type: String },
}
