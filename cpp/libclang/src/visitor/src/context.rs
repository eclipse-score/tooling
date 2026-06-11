///////////////////////////////////////////////////////////////////////////////////
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
////////////////////////////////////////////////////////////////////////////////////
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use class_diagram::SimpleEntity;
use sequence_logic::FunctionDef;

use crate::class_parser_helper::ResolvedType;

pub type TypeMapKey = String;
pub type TypeMapValue = SimpleEntity;
pub type TypeMap = HashMap<TypeMapKey, TypeMapValue>;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct VisitContext {
    pub types: TypeMap,
    pub parsed_class_info: Vec<ParsedClassInfo>,
    pub functions: Vec<FunctionDef>,
    pub is_templated: bool,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ParsedClassInfo {
    pub id: String,                      // class fqn
    pub base_classes: Vec<ResolvedType>, // base classes for inheritance relationships
    pub variable_types: Vec<ParsedVariableType>,
    pub method_types: Vec<ParsedMethodType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedVariableType {
    pub name: String,
    pub resolved_type: ResolvedType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMethodType {
    pub name: String,
    pub return_type: ResolvedType,
    pub parameter_types: Vec<ResolvedType>,
}
