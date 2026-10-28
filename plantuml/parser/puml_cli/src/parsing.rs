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

//! Parsing of one preprocessed diagram into its raw AST.

use clap::ValueEnum;
use log::debug;
use serde::Serialize;
use std::path::PathBuf;
use std::rc::Rc;

use puml_parser::{
    DiagramParser, ErrorLocation, PumlActivityParser, PumlClassParser, PumlComponentParser,
    PumlSequenceParser,
};
use puml_utils::LogLevel;

/// Grammar / diagram type selected with `--diagram-type`.
#[derive(Copy, Clone, ValueEnum, Debug)]
pub enum DiagramType {
    None,
    Activity,
    Component,
    Deployment,
    Class,
    Sequence,
}

#[derive(Debug, Serialize)]
pub enum ParsedDiagram {
    Activity(puml_parser::RawActivityDiagram),
    Component(puml_parser::CompPumlDocument),
    Class(puml_parser::ClassUmlFile),
    Sequence(puml_parser::SeqPumlDocument),
}

/// A kind of diagram with a parser of its own.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DiagramKind {
    Component,
    Activity,
    Class,
    Sequence,
}

impl DiagramKind {
    /// Order in which `--diagram-type none` tries the kinds.
    const DETECTION_ORDER: [DiagramKind; 4] = [
        DiagramKind::Component,
        DiagramKind::Activity,
        DiagramKind::Class,
        DiagramKind::Sequence,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Component => "Component",
            Self::Activity => "Activity",
            Self::Class => "Class",
            Self::Sequence => "Sequence",
        }
    }

    fn parse(
        self,
        path: &Rc<PathBuf>,
        content: &str,
        log_level: LogLevel,
    ) -> Result<ParsedDiagram, ParseFailure> {
        match self {
            Self::Component => attempt(
                PumlComponentParser,
                path,
                content,
                log_level,
                ParsedDiagram::Component,
            ),
            Self::Activity => attempt(
                PumlActivityParser,
                path,
                content,
                log_level,
                ParsedDiagram::Activity,
            ),
            Self::Class => attempt(
                PumlClassParser,
                path,
                content,
                log_level,
                ParsedDiagram::Class,
            ),
            Self::Sequence => attempt(
                PumlSequenceParser,
                path,
                content,
                log_level,
                ParsedDiagram::Sequence,
            ),
        }
    }
}

impl DiagramType {
    /// The kind to parse as, or `None` to detect it.
    fn kind(self) -> Option<DiagramKind> {
        match self {
            Self::None => None,
            Self::Activity => Some(DiagramKind::Activity),
            Self::Component | Self::Deployment => Some(DiagramKind::Component),
            Self::Class => Some(DiagramKind::Class),
            Self::Sequence => Some(DiagramKind::Sequence),
        }
    }
}

/// A failed parse: the parser's error and, when it knows, where it stopped
/// (line, column).
struct ParseFailure {
    error: Box<dyn std::error::Error>,
    location: Option<(usize, usize)>,
}

fn attempt<P>(
    mut parser: P,
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
    wrap: fn(P::Output) -> ParsedDiagram,
) -> Result<ParsedDiagram, ParseFailure>
where
    P: DiagramParser,
    P::Error: ErrorLocation + std::error::Error + 'static,
{
    parser
        .parse_file(path, content, log_level)
        .map(wrap)
        .map_err(|error| ParseFailure {
            location: error.error_location(),
            error: Box::new(error),
        })
}

// lobster-trace: Tools.ArchitectureModelingSyntax
pub fn parse_puml_file(
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
    diagram_type: DiagramType,
) -> Result<ParsedDiagram, Box<dyn std::error::Error>> {
    match diagram_type.kind() {
        Some(kind) => parse_as(kind, path, content, log_level),
        None => parse_in_order(path, content, log_level),
    }
}

/// Parses as `kind`; a file of another kind is named as such, followed by
/// the `kind` parser's own error.
fn parse_as(
    kind: DiagramKind,
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
) -> Result<ParsedDiagram, Box<dyn std::error::Error>> {
    match kind.parse(path, content, log_level) {
        Ok(parsed) => {
            debug!("Successfully parsed PlantUML document: {}", path.display());
            Ok(parsed)
        }
        Err(failure) => Err(match detect_other_kind(kind, path, content, log_level) {
            Some(found) => format!(
                "{}: expected a {} diagram, found a {} diagram\n  {} parser error: {}",
                path.display(),
                kind.name(),
                found.name(),
                kind.name(),
                failure.error
            )
            .into(),
            None => failure.error,
        }),
    }
}

/// The first kind other than `expected` whose parser accepts the file.
fn detect_other_kind(
    expected: DiagramKind,
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
) -> Option<DiagramKind> {
    DiagramKind::DETECTION_ORDER
        .into_iter()
        .filter(|&kind| kind != expected)
        .find(|kind| kind.parse(path, content, log_level).is_ok())
}

/// Parses as the first kind of [`DiagramKind::DETECTION_ORDER`] that accepts
/// the file; otherwise reports the parsers' errors.
fn parse_in_order(
    path: &Rc<PathBuf>,
    content: &str,
    log_level: LogLevel,
) -> Result<ParsedDiagram, Box<dyn std::error::Error>> {
    let mut failures: Vec<(DiagramKind, ParseFailure)> = Vec::new();

    for kind in DiagramKind::DETECTION_ORDER {
        match kind.parse(path, content, log_level) {
            Ok(parsed) => {
                debug!("Successfully detected as {} diagram", kind.name());
                return Ok(parsed);
            }
            Err(failure) => {
                debug!(
                    "{} parser failed at {:?}: {}",
                    kind.name(),
                    failure.location,
                    failure.error
                );
                failures.push((kind, failure));
            }
        }
    }

    // The parser that reached the furthest line is the most informative one.
    let best = failures
        .iter()
        .max_by_key(|(_, failure)| failure.location.map_or(0, |(line, _)| line));

    let tried_names: Vec<&str> = failures.iter().map(|(kind, _)| kind.name()).collect();

    let detail = match best {
        Some((kind, failure)) => match failure.location {
            Some((line_no, _col)) => {
                let source_line = content
                    .lines()
                    .nth(line_no - 1)
                    .unwrap_or("<unknown>")
                    .trim();
                format!(
                    "\n  Parsers tried: {}\n  Parser with longest match: {}\n  Failed at line {}: {}\n  Error: {}",
                    tried_names.join(", "),
                    kind.name(),
                    line_no,
                    source_line,
                    failure.error,
                )
            }
            None => format!(
                "\n  Parsers tried: {}\n  Parser with longest match: {}\n  Error: {}",
                tried_names.join(", "),
                kind.name(),
                failure.error,
            ),
        },
        None => String::new(),
    };

    Err(format!(
        "Failed to parse {} with any available parser{}",
        path.display(),
        detail,
    )
    .into())
}
