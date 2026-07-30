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

pub mod sequence_ast;
mod sequence_parser;

pub use sequence_ast::{
    ActivateCmd, Arrow, CreateCmd, DeactivateCmd, DestroyCmd, GroupCmd, GroupType, Message,
    MessageEndpoint, MessageSuffix, ParticipantIdentifier, ParticipantRef, ParticipantType,
    RefCmd, SeqPumlDocument, Statement,
};

pub use sequence_parser::{PumlSequenceParser, SequenceError};
