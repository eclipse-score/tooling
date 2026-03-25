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

pub mod syntax_ast;
mod syntax_parser;

pub use syntax_ast::{
    ActivateCmd, Arrow, CreateCmd, DeactivateCmd, DestroyCmd, GroupCmd, GroupType, Message,
    MessageContent, ParticipantDef, ParticipantIdentifier, ParticipantType, SeqPumlDocument,
    Statement,
};

pub use syntax_parser::PumlSequenceParser;

/// Parse a PlantUML sequence diagram and return the document name and statements
/// This is a convenience function for backwards compatibility with tests
pub fn parse_sequence_diagram(
    input: &str,
) -> Result<(Option<String>, Vec<Statement>), Box<dyn std::error::Error>> {
    use parser_core::DiagramParser;
    use puml_utils::LogLevel;
    use std::path::PathBuf;
    use std::rc::Rc;

    let mut parser = PumlSequenceParser;
    let dummy_path = Rc::new(PathBuf::from("<input>"));
    let document = parser.parse_file(&dummy_path, input, LogLevel::Error)?;

    Ok((document.name, document.statements))
}
