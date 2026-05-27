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
pub struct RawActivityDiagram {
    pub name: Option<String>,
    pub statements: Vec<RawActivityStmt>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawActivitySourceSpan {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RawActivityStmt {
    Title(TitleStmt),
    Action(ActionStmt),
    Arrow(ArrowStmt),
    Backward(BackwardStmt),

    Start(StartStmt),
    Stop(StopStmt),
    Control(ControlStmt),

    // ===== If =====
    IfStart(IfStartStmt),
    Else(ElseStmt),
    EndIf(EndIfStmt),

    // ===== While =====
    WhileStart(WhileStartStmt),
    EndWhile(EndWhileStmt),

    // ===== Repeat =====
    RepeatStart(RepeatStartStmt),
    RepeatWhile(RepeatWhileStmt),

    // ===== Fork =====
    ForkStart(ForkStartStmt),
    ForkAgain(ForkAgainStmt),
    ForkEnd(ForkEndStmt),

    // =====Swimlane =====
    Swimlane(SwimlaneStmt),
}

impl RawActivityStmt {
    pub fn start_location(&self) -> (usize, usize) {
        let source = match self {
            Self::Title(stmt) => stmt.source,
            Self::Action(stmt) => stmt.source,
            Self::Arrow(stmt) => stmt.source,
            Self::Backward(stmt) => stmt.source,
            Self::Start(stmt) => stmt.source,
            Self::Stop(stmt) => stmt.source,
            Self::Control(stmt) => stmt.source,
            Self::IfStart(stmt) => stmt.source,
            Self::Else(stmt) => stmt.source,
            Self::EndIf(stmt) => stmt.source,
            Self::WhileStart(stmt) => stmt.source,
            Self::EndWhile(stmt) => stmt.source,
            Self::RepeatStart(stmt) => stmt.source,
            Self::RepeatWhile(stmt) => stmt.source,
            Self::ForkStart(stmt) => stmt.source,
            Self::ForkAgain(stmt) => stmt.source,
            Self::ForkEnd(stmt) => stmt.source,
            Self::Swimlane(stmt) => stmt.source,
        };

        (source.start_line, source.start_column)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TitleStmt {
    pub text: String,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionStmt {
    pub label: String,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArrowStmt {
    pub syntax: String,
    pub label: Option<String>,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackwardStmt {
    pub label: String,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StartStmt {
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StopStmt {
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlStmt {
    pub kind: ControlKind,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ControlKind {
    Break,
    Kill,
    Detach,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfStartStmt {
    pub condition: String,
    pub label: Option<String>,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElseStmt {
    pub label: Option<String>,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EndIfStmt {
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WhileStartStmt {
    pub condition: String,
    pub label: Option<String>,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EndWhileStmt {
    pub label: Option<String>,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepeatStartStmt {
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepeatWhileStmt {
    pub condition: String,
    pub label: Option<String>,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkStartStmt {
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkAgainStmt {
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkEndStmt {
    pub kind: ForkEndKind,
    pub modifier: Option<ForkModifier>,
    pub source: RawActivitySourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ForkEndKind {
    EndFork,
    EndMerge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ForkModifier {
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwimlaneStmt {
    pub name: String,
    pub source: RawActivitySourceSpan,
}
