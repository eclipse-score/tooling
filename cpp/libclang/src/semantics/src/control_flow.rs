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
use source_location::SourceLocation;

/// A single item inside a function, branch, or loop body in execution order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BodyItem {
    /// A resolved callee's qualified name, without a leading `::`.
    Call {
        target: String,
        source_location: SourceLocation,
    },
    Branch {
        cases: Vec<BranchCase>,
    },
    Loop {
        kind: LoopKind,
        body: Vec<BodyItem>,
        source_location: SourceLocation,
    },
}

/// The control-flow form of a loop statement.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopKind {
    For,
    While,
    DoWhile,
}

/// One arm of a conditional branch. `None` represents a final `else` arm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchCase {
    /// `None` represents a final `else` arm.
    pub guard: Option<GuardExpression>,
    pub body: Vec<BodyItem>,
    pub source_location: SourceLocation,
}

/// The condition that controls whether a branch case's body executes.
///
/// Logical nodes preserve C++ short-circuit structure. Other expressions are
/// retained as opaque source text until they receive dedicated semantic nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GuardExpression {
    Opaque {
        text: String,
        source_location: SourceLocation,
    },
    Call {
        /// A resolved callee's qualified name, without a leading `::`.
        target: String,
        text: String,
        source_location: SourceLocation,
    },
    Not {
        expression: Box<GuardExpression>,
    },
    And {
        expressions: Vec<GuardExpression>,
    },
    Or {
        expressions: Vec<GuardExpression>,
    },
}
