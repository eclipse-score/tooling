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
use pest::error::Error as PestError;
use pest::error::{ErrorVariant, LineColLocation};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum BaseParseError<Rule> {
    #[error("Failed to read include file {path}: {error}")]
    IoError {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
    },

    #[error("Pest error: {message}")]
    SyntaxError {
        file: PathBuf,
        line: usize,
        column: usize,
        message: String,
        source_line: String,
        #[source]
        cause: Option<Box<pest::error::Error<Rule>>>,
    },
}

pub fn pest_to_syntax_error<Rule>(
    err: PestError<Rule>,
    file: PathBuf,
    source: &str,
) -> BaseParseError<Rule>
where
    Rule: std::fmt::Debug,
{
    let (line, column) = match err.line_col {
        LineColLocation::Pos((l, c)) => (l, c),
        LineColLocation::Span((l, c), _) => (l, c),
    };

    let source_line = source
        .split_inclusive('\n')
        .nth(line - 1)
        .map(|s| s.trim_matches(' ').to_string())
        .unwrap_or("<no source line>\n".to_string());

    let message = match &err.variant {
        ErrorVariant::ParsingError {
            positives,
            negatives,
        } => {
            format!(
                "Parsing error at {:?}, expected {:?}, got {:?}",
                (line, column),
                positives,
                negatives
            )
        }
        ErrorVariant::CustomError { message } => message.clone(),
    };

    BaseParseError::<Rule>::SyntaxError {
        file,
        line,
        column,
        message,
        source_line,
        cause: Some(Box::new(err)),
    }
}
