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
pub struct ProcedureFile {
    pub stmts: Vec<Statement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Statement {
    Procedure(ProcedureDef),
    MacroCall(MacroCallDef),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroCallDef {
    pub name: String,
    pub args: Vec<Arg>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcedureDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<BodyNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BodyNode {
    MacroCall(MacroCallDef),
    Text(Vec<TextPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Arg {
    /// macro: $alias
    Variable(String),
    /// string: "G1"
    String(String),
    /// number: 123, -42
    Number(i64),
    /// identifier: foo
    Identifier(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TextPart {
    /// literal: plain text
    Literal(String),
    /// variable: $alias
    Variable(String),
}
