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
pub use parser_core::common_ast::Arrow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompPumlDocument {
    pub name: Option<String>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Statement {
    Component(Component),
    Relation(Relation),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Component {
    pub component_type: String,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub stereotype: Option<String>,
    pub style: Option<ComponentStyle>,
    pub statements: Vec<Statement>, // For nested components
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relation {
    pub lhs: String,
    pub arrow: Arrow,
    pub rhs: String,
    pub style: Option<ComponentStyle>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentStyle {
    pub color: Option<String>,
    pub attributes: Vec<String>,
}
