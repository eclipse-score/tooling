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

pub use parser_core::common_ast::Arrow;

// Document structure representing a complete PlantUML sequence diagram
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeqPumlDocument {
    pub name: Option<String>,
    pub statements: Vec<Statement>,
}

// Statement types used during parsing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Keep `Message` unboxed for now because the statement representation is
// expected to be revisited as the sequence parser/resolver model settles.
#[allow(clippy::large_enum_variant)]
pub enum Statement {
    CreateCmd(CreateCmd),
    DestroyCmd(DestroyCmd),
    ActivateCmd(ActivateCmd),
    DeactivateCmd(DeactivateCmd),
    ParticipantDef(ParticipantDef),
    Message(Message),
    GroupCmd(GroupCmd),
}

// Participant definitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticipantDef {
    pub participant_type: ParticipantType,
    pub identifier: ParticipantIdentifier,
    pub stereotype: Option<String>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateCmd {
    pub participant_type: ParticipantType,
    pub identifier: ParticipantIdentifier,
    pub stereotype: Option<String>,
    pub source_location: SourceLocation,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticipantRef {
    pub identifier: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DestroyCmd {
    pub participant: ParticipantRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivateCmd {
    pub participant: ParticipantRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeactivateCmd {
    pub participant: ParticipantRef,
}

// Messages (internal parsing structure)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub left: MessageEndpoint,
    pub arrow: Arrow,
    pub right: MessageEndpoint,
    pub suffix: Option<MessageSuffix>,
    pub description: Option<String>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageEndpoint {
    Participant(ParticipantIdentifier),
    LostFound(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageSuffix {
    Activate,   // ++
    Deactivate, // --
    Create,     // **
    Destroy,    // !!
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
