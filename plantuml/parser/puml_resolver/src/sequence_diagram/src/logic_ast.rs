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
use sequence_parser::syntax_ast::GroupType;
use serde::{Deserialize, Serialize};

// Content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceTree {
    pub name: Option<String>,
    pub root_interactions: Vec<SequenceNode>,
}

// Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Interaction(Interaction),
    Return(Return),
    Condition(Condition),
}

// SequenceNode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceNode {
    pub event: Event,
    pub branches_node: Vec<SequenceNode>,
}

// Interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub caller: String,
    pub callee: String,
    pub method: String,
}

// Return
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Return {
    pub caller: String,
    pub callee: String,
    pub return_content: String,
}

// Condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub condition_type: GroupType,
    pub condition_value: String,
}
