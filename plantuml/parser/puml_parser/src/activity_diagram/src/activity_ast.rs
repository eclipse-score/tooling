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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RawActivityStmt {
    Action(ActionStmt),

    Start(StartStmt),
    Stop(StopStmt),

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionStmt {
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StartStmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StopStmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfStartStmt {
    pub condition: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElseStmt {
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EndIfStmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WhileStartStmt {
    pub condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EndWhileStmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepeatStartStmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepeatWhileStmt {
    pub condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkStartStmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkAgainStmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkEndStmt {
    pub kind: ForkEndKind,
    pub modifier: Option<ForkModifier>,
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
}
