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
pub mod class_ast;
mod class_parser;
mod class_traits;

pub use class_ast::{
    Attribute, ClassDef, ClassUmlFile, ClassUmlTopLevel, Element, EnumDef, EnumItem, EnumValue,
    Method, Name, Namespace, Package, Param, Relationship, Visibility,
};
pub use class_parser::{ClassError, PumlClassParser};

/// Parse a PlantUML class diagram and return the parsed structure
/// This is a convenience function for backwards compatibility with tests
pub fn parse_class_diagram(input: &str) -> Result<ClassUmlFile, Box<dyn std::error::Error>> {
    use parser_core::DiagramParser;
    use puml_utils::LogLevel;
    use std::path::PathBuf;
    use std::rc::Rc;

    let mut parser = PumlClassParser;
    let dummy_path = Rc::new(PathBuf::from("<input>"));
    let document = parser.parse_file(&dummy_path, input, LogLevel::Error)?;

    Ok(document)
}
