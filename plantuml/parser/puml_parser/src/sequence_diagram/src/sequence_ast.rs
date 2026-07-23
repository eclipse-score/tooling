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
// AST types for PlantUML Sequence Diagram Parser

use serde::{Deserialize, Serialize};
use source_location::SourceLocation;
use std::str::FromStr;

pub use parser_core::common_ast::Arrow;

// Document structure representing a complete PlantUML sequence diagram
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeqPumlDocument {
    pub name: Option<String>,
    pub statements: Vec<Statement>,
}

// Statement types used during parsing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    DestroyCmd(DestroyCmd),
    CreateCmd(CreateCmd),
    ActivateCmd(ActivateCmd),
    DeactivateCmd(DeactivateCmd),
    ParticipantDef(ParticipantDef),
    Message(Message),
    GroupCmd(GroupCmd),
}

// Participant definitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticipantDef {
    #[serde(default)]
    pub is_create: bool,
    pub participant_type: ParticipantType,
    pub identifier: ParticipantIdentifier,
    pub stereotype: Option<String>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalEndpoint;

impl FromStr for ExternalEndpoint {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "[" | "[o" | "[x" | "]" | "o]" | "x]" => Ok(ExternalEndpoint),
            _ => Err(()),
        }
    }
}

impl ExternalEndpoint {
    pub fn as_name(self) -> &'static str {
        "ExternalEndpoint"
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticipantIdentifier {
    pub display_name: String,
    pub alias: Option<String>,
}

// Destroy/Create commands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DestroyCmd {
    pub participant: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateCmd {
    pub participant: String,
}

// Activate/Deactivate commands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivateCmd {
    pub participant: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeactivateCmd {
    pub participant: Option<String>,
}

// Messages (internal parsing structure)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_marker: Option<String>,
    pub description: Option<String>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageContent {
    WithTargets {
        left: String,
        arrow: Arrow,
        right: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActivationType {
    Activate,   // ++
    Deactivate, // --
}

// Group commands (alt, opt, loop, etc.) - internal parsing structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupCmd {
    pub group_type: GroupType,
    pub text: Option<String>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GroupType {
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
