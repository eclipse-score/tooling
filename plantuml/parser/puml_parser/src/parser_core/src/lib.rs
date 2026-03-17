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
pub mod common_ast;
pub mod common_parser;
pub mod error;

pub use common_ast::*;
pub use common_parser::*;
pub use error::{pest_to_syntax_error, BaseParseError};

pub trait DiagramParser {
    type Output;
    type Error;

    fn parse_file(
        &mut self,
        path: &std::rc::Rc<std::path::PathBuf>,
        content: &str,
        log_level: puml_utils::LogLevel,
    ) -> Result<Self::Output, Self::Error>;
}
