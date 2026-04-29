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

/// A single item inside a function/branch/loop body, emitted in execution order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BodyItem {
    /// A cross-class method call.
    Call { callee: String, name: String },
    /// One arm of an if / else-if / else.  The `condition` field is the guard
    /// expression text, or `"else"` for an unconditional else arm.
    Branch {
        condition: String,
        body: Vec<BodyItem>,
    },
    /// A for / while / do-while loop.
    Loop { kind: String, body: Vec<BodyItem> },
}

/// Represents a class method definition extracted from C++ source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub class: String,
    pub name: String,
    pub return_type: String,
    /// Method body items in execution order (calls, branches, loops).
    pub body: Vec<BodyItem>,
}

// ─── PlantUML sequence-diagram logic-tree types ─────────────────────────────

/// The kind of condition / group block in a sequence diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionType {
    Opt,
    Alt,
    Loop,
    Par,
    Par2,
    Break,
    Critical,
    Else,
    Also,
    End,
    Group,
}

/// A condition / group block header in a sequence diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub condition_type: ConditionType,
    pub condition_value: String,
}

/// A method-call interaction between two participants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub caller: String,
    pub callee: String,
    pub method: String,
}

/// A return message between two participants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Return {
    pub caller: String,
    pub callee: String,
    pub return_content: String,
}

/// An event in a sequence diagram: a call, a return, or a condition block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Interaction(Interaction),
    Return(Return),
    Condition(Condition),
}

/// A node in the hierarchical sequence-diagram logic tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceNode {
    pub event: Event,
    pub branches_node: Vec<SequenceNode>,
}

/// Root container for a sequence-diagram logic tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceTree {
    pub name: Option<String>,
    pub root_interactions: Vec<SequenceNode>,
}
