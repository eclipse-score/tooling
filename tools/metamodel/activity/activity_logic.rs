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
pub struct ActivityDiagram {
    pub name: Option<String>,
    pub statements: Vec<ActivityStmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct IfDisplay {
    pub then_label: Option<String>,
    pub else_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LoopDisplay {
    pub continue_label: Option<String>,
    pub exit_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActivityStmt {
    Title(TitleNode),
    Action(ActionNode),

    If(IfNode),
    While(WhileNode),
    RepeatWhile(RepeatWhileNode),

    Control(ControlNode),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TitleNode {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlNode {
    pub kind: ControlKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ControlKind {
    /// stop current path (end this execution branch)
    Stop,

    /// terminate entire activity (kill all paths)
    Kill,

    /// detach current flow (split into independent execution flow)
    Detach,

    /// break out of nearest loop (while/repeat)
    Break,

    /// continue to next iteration of loop
    Continue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionNode {
    pub label: String,
}

/// Represents a PlantUML `backward` action on a loop return path.
///
/// This is not a regular sequential statement in the loop body. Instead, it
/// executes only when control flows from the end of the loop body back to the
/// next loop iteration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackwardNode {
    pub label: String,
}

/// Represents a structured `if` control-flow node.
///
/// This node follows a Python-style AST design:
///
/// - `elseif` is NOT represented as an independent node.
/// - Instead, `elseif` is lowered into:
///
///   `else { if (...) { ... } }`
///
/// Example:
///
/// PlantUML source:
/// if A
///   :foo;
/// elseif B
///   :bar;
/// endif
///
/// Semantic AST:
/// IfNode {
///     condition: "A",
///     body: vec![
///         Action(foo)
///     ],
///     else_branch: vec![
///         ActivityStmt::If(
///             IfNode {
///                 condition: "B",
///                 body: vec![
///                     Action(bar)
///                 ],
///                 else_branch: vec![],
///             }
///         )
///     ],
/// }
///
/// Advantages of this design:
///
/// - Keeps the control-flow model simple and uniform
/// - Eliminates special handling for `elseif`
/// - Makes CFG generation easier
/// - Matches how Python's AST represents `elif`
///
/// An empty `else_branch` means there is no `else` branch.
///
/// Optional display labels are stored separately in `display` so they do not
/// affect the core control-flow semantics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfNode {
    pub condition: String,
    pub body: Vec<ActivityStmt>,
    pub else_branch: Vec<ActivityStmt>,
    pub display: Option<IfDisplay>,
}

/// Represents a structured `while` loop node.
///
/// Example:
///
/// PlantUML source:
/// while (A?) is (yes)
///   :work;
/// endwhile (no)
///
/// Semantic AST:
/// WhileNode {
///     condition: "A?",
///     body: vec![
///         ActivityStmt::Action(
///             ActionNode {
///                 label: "work",
///             }
///         )
///     ],
///     backward: None,
///     display: None,
/// }
///
/// Notes:
///
/// - `condition` represents the loop condition
/// - `body` contains the loop statements
/// - `backward` is an optional action on the loop return path
/// - `display` stores optional loop edge labels separately from the core
///   semantics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WhileNode {
    pub condition: String,
    pub body: Vec<ActivityStmt>,
    pub backward: Option<BackwardNode>,
    pub display: Option<LoopDisplay>,
}

/// Represents a structured `repeat ... repeat while` loop.
///
/// `backward` captures a PlantUML `backward` action on the loop return path.
/// It is separate from `body` because it does not run as part of the forward,
/// sequential loop body. It runs only on the back-edge when the repeat loop
/// continues.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepeatWhileNode {
    pub body: Vec<ActivityStmt>,
    pub condition: String,
    pub backward: Option<BackwardNode>,
    pub display: Option<LoopDisplay>,
}
