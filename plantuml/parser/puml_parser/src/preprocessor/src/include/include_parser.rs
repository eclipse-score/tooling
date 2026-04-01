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
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::fs;
use std::path::{Path, PathBuf};

use crate::include_ast::{IncludeFile, IncludeKind, IncludeStmt, IncludeSuffix, SubBlock};
use parser_core::{pest_to_syntax_error, BaseParseError};

#[derive(Parser)]
#[grammar = "../../../grammar/include.pest"]
pub struct IncludeParser;

const INCLUDE_KW: &[&str] = &["!include", "!include_once", "!include_many", "!includesub"];

#[derive(Debug, thiserror::Error)]
pub enum IncludeParseError {
    #[error(transparent)]
    Base(#[from] BaseParseError<Rule>),

    #[error("Invalid text line: {line}, ref to file {file}")]
    InvalidTextLine { line: String, file: PathBuf },
}

#[derive(Default)]
pub struct IncludeParserService;

impl IncludeParserService {
    // -------------------- Parser Entry --------------------
    pub fn parse_file(&mut self, file: &Path) -> Result<Vec<IncludeFile>, IncludeParseError> {
        let content = fs::read_to_string(file).map_err(|e| {
            IncludeParseError::Base(BaseParseError::IoError {
                path: file.to_path_buf(),
                error: Box::new(e),
            })
        })?;
        let mut stmts = Vec::new();
        let file_pairs = IncludeParser::parse(Rule::file, &content).map_err(|e| {
            IncludeParseError::Base(pest_to_syntax_error(e, file.to_path_buf(), &content))
        })?;

        for pair in file_pairs {
            for line in pair.into_inner() {
                match line.as_rule() {
                    Rule::include_line => {
                        let include = self.parse_include_line(line);
                        stmts.push(IncludeFile::Include(include));
                    }
                    Rule::includesub_line => {
                        let include_sub = self.parse_includesub_line(line);
                        stmts.push(IncludeFile::Include(include_sub));
                    }
                    Rule::sub_block => {
                        let sub_block = self.parse_sub_block(line);
                        stmts.push(IncludeFile::SubBlock(sub_block));
                    }
                    Rule::text_line => {
                        let text = line.as_str().to_string();
                        let trimmed = text.trim_start();
                        if INCLUDE_KW.iter().any(|kw| trimmed.starts_with(kw)) {
                            return Err(IncludeParseError::InvalidTextLine {
                                line: text,
                                file: file.to_path_buf(),
                            });
                        }
                        if !text.trim().is_empty() {
                            stmts.push(IncludeFile::Text(text));
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(stmts)
    }

    fn parse_include_line(&self, pair: Pair<Rule>) -> IncludeStmt {
        let directive = pair
            .into_inner()
            .find(|p| p.as_rule() == Rule::include_directive)
            .unwrap();

        let (kind, path) = self.parse_include_directive(directive);

        IncludeStmt::Include { kind, path }
    }

    fn parse_includesub_line(&self, pair: Pair<Rule>) -> IncludeStmt {
        let directive = pair
            .into_inner()
            .find(|p| p.as_rule() == Rule::includesub_directive)
            .unwrap();

        let (path, suffix) = self.parse_includesub_directive(directive);

        IncludeStmt::IncludeSub { path, suffix }
    }

    fn parse_include_directive(&self, pair: Pair<Rule>) -> (IncludeKind, String) {
        let mut kind = None;
        let mut path = None;

        for inner in pair.clone().into_inner() {
            match inner.as_rule() {
                Rule::include_keyword => {
                    kind = Some(match inner.as_str() {
                        "!include" => IncludeKind::Include,
                        "!include_once" => IncludeKind::IncludeOnce,
                        "!include_many" => IncludeKind::IncludeMany,
                        _ => unreachable!(),
                    })
                }
                Rule::include_path => {
                    path = Some(self.extract_path(inner));
                }
                _ => unreachable!(),
            }
        }

        (kind.unwrap(), path.unwrap())
    }

    fn parse_includesub_directive(&self, pair: Pair<Rule>) -> (String, IncludeSuffix) {
        let mut path = None;
        let mut suffix = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::include_path => {
                    path = Some(self.extract_path(inner));
                }
                Rule::include_suffix => {
                    suffix = Some(self.extract_suffix(inner));
                }
                _ => unreachable!(),
            }
        }

        (path.unwrap(), suffix.unwrap())
    }

    fn extract_path(&self, pair: Pair<Rule>) -> String {
        pair.as_str().trim().to_string()
    }

    fn extract_suffix(&self, pair: Pair<Rule>) -> IncludeSuffix {
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::include_index => IncludeSuffix::Index(inner.as_str().parse().unwrap()),
            Rule::include_label => IncludeSuffix::Label(inner.as_str().to_string()),
            _ => unreachable!(),
        }
    }

    fn parse_sub_block(&self, pair: Pair<Rule>) -> SubBlock {
        let mut inner = pair.into_inner();
        let startsub_directive = inner.next().unwrap();
        let name = self.extract_suffix(startsub_directive);

        let mut content: Vec<IncludeFile> = Vec::new();
        for line in inner {
            match line.as_rule() {
                Rule::include_line => {
                    let include = self.parse_include_line(line);
                    content.push(IncludeFile::Include(include));
                }
                Rule::includesub_line => {
                    let include_sub = self.parse_includesub_line(line);
                    content.push(IncludeFile::Include(include_sub));
                }
                Rule::text_line => {
                    let text = line.as_str().to_string();
                    if !text.trim().is_empty() {
                        content.push(IncludeFile::Text(text));
                    }
                }
                _ => {}
            }
        }

        SubBlock { name, content }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;

    static INCLUDE_PARSER_SERVICE: Lazy<IncludeParserService> = Lazy::new(|| IncludeParserService);

    #[test]
    fn test_extract_path() {
        let pair = IncludeParser::parse(Rule::include_path, "path/to/file")
            .unwrap()
            .next()
            .unwrap();
        let path = INCLUDE_PARSER_SERVICE.extract_path(pair);
        assert_eq!(path, "path/to/file");
    }

    #[test]
    fn test_extract_suffix_index() {
        let pair = IncludeParser::parse(Rule::include_suffix, "!1")
            .unwrap()
            .next()
            .unwrap();
        let suffix = INCLUDE_PARSER_SERVICE.extract_suffix(pair);
        assert!(matches!(suffix, IncludeSuffix::Index(1)));
    }

    #[test]
    fn test_extract_suffix_label() {
        let pair = IncludeParser::parse(Rule::include_suffix, "!label1")
            .unwrap()
            .next()
            .unwrap();
        let suffix = INCLUDE_PARSER_SERVICE.extract_suffix(pair);
        assert!(matches!(suffix, IncludeSuffix::Label(ref l) if l == "label1"));
    }

    #[test]
    fn test_parse_include_directive() {
        let input = "!include path/to/file";
        let pair = IncludeParser::parse(Rule::include_directive, input)
            .unwrap()
            .next()
            .unwrap();
        let (kind, path) = INCLUDE_PARSER_SERVICE.parse_include_directive(pair);
        assert!(matches!(kind, IncludeKind::Include));
        assert_eq!(path, "path/to/file");
    }

    #[test]
    fn test_parse_includesub_directive() {
        let input = "!includesub path/to/file!2";
        let pair = IncludeParser::parse(Rule::includesub_directive, input)
            .unwrap()
            .next()
            .unwrap();
        let (path, suffix) = INCLUDE_PARSER_SERVICE.parse_includesub_directive(pair);
        assert_eq!(path, "path/to/file");
        assert!(matches!(suffix, IncludeSuffix::Index(2)));
    }

    #[test]
    fn test_parse_file_io_error() {
        let mut service = IncludeParserService;
        let missing = PathBuf::from("does-not-exist-include-test.puml");
        let result = service.parse_file(&missing);
        assert!(matches!(
            result,
            Err(IncludeParseError::Base(BaseParseError::IoError { .. }))
        ));
    }
}
