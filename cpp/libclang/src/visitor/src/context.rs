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

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use class_diagram::{FreeFunctionDecl, FunctionArgument, Method, SimpleEntity, SourceLocation};
use cpp_semantics::{FunctionDef, ResolvedType};
use serde::{Deserialize, Serialize};

/// Identifies an AST entity within one parser execution.
///
/// This source-position key deduplicates project header declarations and definitions visible
/// through multiple translation units. It is not stable across source revisions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceEntityKey {
    pub source_file: PathBuf,
    pub source_offset: u32,
}

/// Identifies a free function by its logical signature for class-diagram output.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallableIdentityKey {
    pub owner: CallableOwnerIdentityKey,
    pub name: String,
    pub parameters: Vec<CallableArgumentIdentityKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CallableOwnerIdentityKey {
    FreeFunction {
        enclosing_namespace_id: Option<String>,
    },
    Method {
        class_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallableArgumentIdentityKey {
    pub param_type: Option<String>,
    pub is_variadic: bool,
    pub is_pack_expansion: bool,
}

impl From<&FunctionArgument> for CallableArgumentIdentityKey {
    fn from(argument: &FunctionArgument) -> Self {
        Self {
            param_type: argument.param_type.clone(),
            is_variadic: argument.is_variadic,
            is_pack_expansion: argument.is_pack_expansion,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFunction {
    pub key: SourceEntityKey,
    pub definition: FunctionDef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFreeFunctionDeclaration {
    pub key: SourceEntityKey,
    pub declaration: FreeFunctionDecl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMethodDeclaration {
    pub class_id: String,
    pub method: Method,
    pub method_type: ParsedMethodType,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct VisitContext {
    pub types: BTreeMap<String, SimpleEntity>,
    pub parsed_class_info: HashMap<String, ParsedClassInfo>,
    pub free_function_declarations: Vec<ExtractedFreeFunctionDeclaration>,
    pub functions: Vec<ExtractedFunction>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ParsedClassInfo {
    pub id: String, // class fqn
    pub base_classes: Vec<ParsedBaseClass>,
    pub variable_types: Vec<ParsedVariableType>,
    pub method_types: Vec<ParsedMethodType>,
    pub has_abstract_methods: bool,
    pub has_concrete_methods: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedBaseClass {
    pub resolved_type: ResolvedType,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedVariableType {
    pub name: String,
    pub resolved_type: ResolvedType,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMethodType {
    pub name: String,
    pub return_type: ResolvedType,
    pub parameter_types: Vec<ResolvedType>,
    pub source_location: SourceLocation,
}
