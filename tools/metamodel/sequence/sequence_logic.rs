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
pub use source_location::SourceLocation;
use std::sync::Arc;

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

/// For a PlantUML sequence diagram, this is the resolved participant identifier
/// (typically the alias if present, otherwise the display name).
///
/// For C++ code, this is typically the object/class identifier resolved from
/// the call site.
pub type ParticipantId = Arc<str>;

/// A reference fragment.
///
/// PlantUML: ref over A,B : Authentication
/// C++: Optional for the first version
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    pub participants: Vec<ParticipantId>,
    pub text: Option<String>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleAction {
    Create,
    Activate,
    Deactivate,
    Destroy,
}

/// Participant lifecycle.
///
/// create: auto foo = std::make_shared<Foo>();
/// activate:
/// deactivate
/// destroy: delete foo;
/// C++: Optional for the first version
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticipantLifecycle {
    pub participant: ParticipantId,
    pub action: LifecycleAction,
    pub source_location: SourceLocation,
}

/// Early exit from the current interaction.
///
/// PlantUML: break
/// C++: return
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EarlyExit {
    pub reason: Option<String>,
    pub block: Block,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParallelBranch {
    pub label: Option<String>,
    pub block: Block,
    pub source_location: SourceLocation,
}

/// Parallel execution.
///
/// PlantUML: par-else
/// C++: std::thread / std::async / co_await
///
/// Note: First version can ignore the C++ mapping, as it is not a direct equivalent
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parallel {
    pub branches: Vec<ParallelBranch>,
}

/// Loop execution.
///
/// PlantUML: loop ...
/// C++: while (...) / do-while (...) / for (...)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loop {
    pub condition: Option<String>,
    pub block: Block,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchCase {
    pub condition: Option<String>,
    pub block: Block,
    pub source_location: SourceLocation,
}

/// Conditional execution.
///
/// PlantUML: alt-else, opt
/// C++: `if` / `else if` / `else`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Branch {
    pub cases: Vec<BranchCase>,
}

/// A message between two participants.
///
/// PlantUML:
///     1) A -> B : foo()
///     2) return xxx
/// C++:
///     class A {
///         void func(B& b) { b.foo(); }
///     }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interaction {
    /// None represents a PlantUML lost/found endpoint.
    pub sender: Option<ParticipantId>,
    /// None represents a PlantUML lost/found endpoint.
    pub receiver: Option<ParticipantId>,
    pub message: Option<String>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Node {
    Interaction(Interaction),
    Branch(Branch),
    Loop(Loop),
    Parallel(Parallel),
    EarlyExit(EarlyExit),
    Lifecycle(ParticipantLifecycle),
    Reference(Reference),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Block {
    pub items: Vec<Node>,
}

/// A participant in a sequence diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ParticipantType {
    Participant,
    Actor,
    Boundary,
    Control,
    Entity,
    Queue,
    Database,
    Collections,
}

/// A participant in a sequence diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceParticipant {
    pub display_name: String,
    pub alias: Option<String>,
    pub participant_type: ParticipantType,
    pub source_location: SourceLocation,
    pub stereotype: Option<String>,
}

/// Root of a resolved sequence behavior tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceTree {
    pub name: Option<String>,
    #[serde(default)]
    pub participants: Vec<SequenceParticipant>,
    pub root: Block,
}

impl SequenceTree {
    /// Return owned names used to reference this tree's participants.
    ///
    /// PlantUML references a participant by its alias when one exists;
    /// otherwise it uses the participant's display name. Callers choose their
    /// own collection type so they can preserve the ordering and deduplication
    /// semantics needed by their use case.
    pub fn participant_reference_names(&self) -> impl Iterator<Item = String> + '_ {
        self.participants.iter().map(|participant| {
            participant
                .alias
                .as_deref()
                .unwrap_or(&participant.display_name)
                .to_string()
        })
    }
}
