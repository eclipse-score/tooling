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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QualifiedTypeName {
    pub segments: Vec<TypeNameSegment>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_global: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TypeNameSegment {
    Identifier(String),
    Template {
        name: String,
        arguments: Vec<TypeArgument>,
    },
}

impl From<String> for TypeNameSegment {
    fn from(name: String) -> Self {
        Self::Identifier(name)
    }
}

impl From<&str> for TypeNameSegment {
    fn from(name: &str) -> Self {
        Self::Identifier(name.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TypeExpr {
    Builtin(String),
    NamedType(QualifiedTypeName),
    Template {
        name: QualifiedTypeName,
        arguments: Vec<TypeArgument>,
    },
    Array {
        element: Box<TypeExpr>,
        size: Option<String>,
    },
    Function {
        return_type: Box<TypeExpr>,
        parameter_types: Vec<TypeExpr>,
        is_variadic: bool,
        is_noexcept: bool,
    },
    Pointer(Box<TypeExpr>),
    Reference(Box<TypeExpr>),
    RValueReference(Box<TypeExpr>),
    Const(Box<TypeExpr>),
    Volatile(Box<TypeExpr>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TypeArgument {
    Type(TypeExpr),
    Value(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TypeDescription {
    pub raw_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<TypeExpr>,
}

impl From<String> for TypeDescription {
    fn from(raw_text: String) -> Self {
        Self {
            raw_text,
            expression: None,
        }
    }
}

impl From<&str> for TypeDescription {
    fn from(raw_text: &str) -> Self {
        raw_text.to_string().into()
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}
