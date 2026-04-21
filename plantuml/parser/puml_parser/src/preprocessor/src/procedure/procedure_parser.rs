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
use std::path::PathBuf;
use std::rc::Rc;

use crate::procedure_ast::{
    Arg, BodyNode, MacroCallDef, ProcedureDef, ProcedureFile, Statement, TextPart,
};
use parser_core::{pest_to_syntax_error, BaseParseError, DiagramParser};
use puml_utils::LogLevel;

#[derive(Parser)]
#[grammar = "../../../grammar/procedure.pest"]
pub struct ProcedureParser;

#[derive(Debug, thiserror::Error)]
pub enum ProcedureParseError {
    #[error(transparent)]
    Base(#[from] BaseParseError<Rule>),
}

#[derive(Default)]
pub struct ProcedureParserService;

impl DiagramParser for ProcedureParserService {
    type Output = ProcedureFile;
    type Error = ProcedureParseError;

    fn parse_file(
        &mut self,
        path: &Rc<PathBuf>,
        content: &str,
        _log_level: LogLevel,
    ) -> Result<Self::Output, Self::Error> {
        let file_pair = ProcedureParser::parse(Rule::file, content)
            .map_err(|e| {
                ProcedureParseError::Base(pest_to_syntax_error(e, path.to_path_buf(), content))
            })?
            .next()
            .unwrap();

        let mut stmts = Vec::new();

        for line in file_pair.into_inner() {
            match line.as_rule() {
                Rule::procedure_line => {
                    let proc_def = parse_procedure(line);
                    stmts.push(Statement::Procedure(proc_def));
                }
                Rule::macro_call_line => {
                    let macro_call = parse_macro_call(line);
                    stmts.push(Statement::MacroCall(macro_call));
                }
                Rule::text_line => {
                    let text = line.as_str().trim().to_string();
                    if !text.is_empty() {
                        stmts.push(Statement::Text(text));
                    }
                }
                _ => {}
            }
        }

        Ok(ProcedureFile { stmts })
    }
}

fn parse_procedure(pair: pest::iterators::Pair<Rule>) -> ProcedureDef {
    let mut name = String::new();
    let mut params = Vec::new();
    let mut body = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::proc_name => {
                name = inner.as_str().to_string();
            }
            Rule::param_list => {
                params = inner.into_inner().map(|p| p.as_str().to_string()).collect();
            }
            Rule::procedure_body => {
                body = parse_body(inner);
            }
            _ => {}
        }
    }

    ProcedureDef { name, params, body }
}

fn parse_body(pair: pest::iterators::Pair<Rule>) -> Vec<BodyNode> {
    pair.into_inner()
        .filter_map(|inner| match inner.as_rule() {
            Rule::macro_call_line => Some(BodyNode::MacroCall(parse_macro_call(inner))),
            Rule::text_line => Some(BodyNode::Text(parse_text_line(inner))),
            _ => None,
        })
        .collect()
}

fn parse_macro_call(pair: pest::iterators::Pair<Rule>) -> MacroCallDef {
    let mut name = String::new();
    let mut args = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::macro_identifier | Rule::identifier => {
                name = inner.as_str().to_string();
            }
            Rule::arg_list => {
                args = parse_arg_list(inner);
            }
            _ => {}
        }
    }

    MacroCallDef { name, args }
}

fn parse_arg_list(pair: Pair<Rule>) -> Vec<Arg> {
    pair.into_inner().map(parse_arg).collect()
}

fn parse_arg(pair: Pair<Rule>) -> Arg {
    let inner = pair.into_inner().next().unwrap();
    let original_text = inner.as_str().to_string();

    match inner.as_rule() {
        Rule::macro_identifier => Arg::Variable(original_text),
        Rule::string => {
            let s = &original_text[1..original_text.len() - 1];
            Arg::String(s.to_string())
        }
        Rule::number => Arg::Number(original_text.parse::<i64>().unwrap()),
        Rule::identifier => Arg::Identifier(original_text),
        _ => unreachable!(),
    }
}

fn parse_text_line(pair: Pair<Rule>) -> Vec<TextPart> {
    let text = pair.as_str();
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();

    // Walk the raw text to split it into literal spans and $variable tokens.
    while let Some(c) = chars.next() {
        if c == '$' {
            // Treat '$' as literal when it appears inside an identifier (e.g., "123$val").
            let prev_is_ident = current
                .chars()
                .last()
                .is_some_and(|ch| ch.is_alphanumeric() || ch == '_');
            if prev_is_ident {
                current.push(c);
                continue;
            }

            if !current.is_empty() {
                // Flush the accumulated literal text before starting a $variable token.
                parts.push(TextPart::Literal(current.clone()));
                current.clear();
            }

            let mut var = String::from("$");
            // Collect the variable name following '$' to build the full $variable token.
            while let Some(&ch) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    var.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            parts.push(TextPart::Variable(var));
        } else {
            current.push(c);
        }
    }

    if !current.is_empty() {
        parts.push(TextPart::Literal(current));
    }

    parts
}
