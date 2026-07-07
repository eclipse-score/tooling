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
pub use source_location::SourceLocation;

// #[derive(Debug, Clone)]
// pub struct Package {
//     pub elements: Vec<LogicElement>,
// }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogicComponent {
    pub id: String, //FQN
    pub name: Option<String>,
    pub alias: Option<String>,
    pub parent_id: Option<String>, // FQN of parent
    #[serde(rename = "element_type", alias = "comp_type")]
    pub element_type: ComponentType, // e.g., package, component, etc.
    pub stereotype: Option<String>, // e.g., component, unit, etc.
    pub relations: Vec<LogicRelation>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ComponentType {
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
    #[serde(default)]
    pub relation_type: ComponentRelationType,
    /// Role of source component w.r.t. target interface.
    #[serde(default)]
    pub source_role: EndpointRole,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum ComponentRelationType {
    #[default]
    #[serde(alias = "None")]
    /// Association or Connected, `--` or `..`
    Association,
    /// Dependency (uses/calls) `..>`, `-->`
    Dependency,
    /// Interface, `port --() Interface`, `-(`, `)-`
    InterfaceBinding,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum EndpointRole {
    #[default]
    None,
    Provided,
    Required,
}
