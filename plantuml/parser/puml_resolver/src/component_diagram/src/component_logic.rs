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
//     pub elements: Vec<LogicElement>,
// }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogicElement {
    pub id: String, //FQN
    pub name: Option<String>,
    pub alias: Option<String>,
    pub parent_id: Option<String>, // FQN of parent
    #[serde(rename = "element_type", alias = "comp_type")]
    pub element_type: ElementType, // e.g., package, component, etc.
    pub stereotype: Option<String>, // e.g., component, unit, etc.
    pub relations: Vec<LogicRelation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ElementType {
    Artifact,
    Actor,
    Agent,
    Boundary,
    Card,
    Cloud,
    Component,
    Control,
    Database,
    Entity,
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
    Usecase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogicRelation {
    pub target: String, // FQN
    pub annotation: Option<String>,
    pub relation_type: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ElementResolverError {
    #[error("Element Resolver: UnresolvedReference: {reference}")]
    UnresolvedReference { reference: String },

    #[error("Duplicate element id: {element_id}")]
    DuplicateElement { element_id: String },

    #[error("Unknown element type: {element_type}")]
    UnknownElementType { element_type: String },
}

pub type LogicComponent = LogicElement;
pub type ComponentType = ElementType;
pub type ComponentResolverError = ElementResolverError;
