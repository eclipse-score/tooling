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

use crate::{BodyItem, ResolvedType};

/// The semantic scope that declares a C++ callable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    /// The C++ global namespace (`::`).
    Global,
    /// A namespace path, such as `company::network`.
    Namespace(Vec<String>),
    /// A type, optionally nested in namespaces and other types.
    Type {
        namespace: Vec<String>,
        type_path: Vec<String>,
    },
}

impl Scope {
    /// Returns this scope's C++ qualified name without a leading `::`.
    pub fn qualified_name(&self) -> String {
        match self {
            Self::Global => String::new(),
            Self::Namespace(path) => path.join("::"),
            Self::Type {
                namespace,
                type_path,
            } => namespace
                .iter()
                .chain(type_path.iter())
                .cloned()
                .collect::<Vec<_>>()
                .join("::"),
        }
    }
}

/// The C++ kind of a callable definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionKind {
    Free,
    Method,
    StaticMethod,
    Constructor,
    Destructor,
    Conversion,
}

/// Stable semantic identity of a C++ callable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionId {
    pub scope: Scope,
    pub name: String,
}

impl FunctionId {
    /// Returns the function's C++ qualified name without a leading `::`.
    pub fn qualified_name(&self) -> String {
        let scope_name = self.scope.qualified_name();
        if scope_name.is_empty() {
            self.name.clone()
        } else {
            format!("{scope_name}::{}", self.name)
        }
    }
}

/// A function definition extracted from C++ source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub id: FunctionId,
    pub kind: FunctionKind,
    /// Constructors and destructors have no ordinary return type.
    pub return_type: Option<ResolvedType>,
    /// Body items in execution order.
    pub body: Vec<BodyItem>,
}
