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
use log::debug;
use pest::Parser;
use std::path::PathBuf;
use std::rc::Rc;
use thiserror::Error;

use crate::activity_ast::{
    ActionStmt, ArrowStmt, BackwardStmt, ControlKind, ControlStmt, ElseStmt, EndIfStmt,
    EndWhileStmt, ForkAgainStmt, ForkEndKind, ForkEndStmt, ForkModifier, ForkStartStmt,
    IfStartStmt, RawActivitySourceSpan, RepeatStartStmt, RepeatWhileStmt, StartStmt,
    StopStmt, SwimlaneStmt, TitleStmt, WhileStartStmt,
};
use crate::creole::normalize_creole_text;
use crate::{RawActivityDiagram, RawActivityStmt};
use parser_core::common_parser::{PlantUmlCommonParser, Rule};
use parser_core::{
    format_parse_tree, pest_to_syntax_error, BaseParseError, DiagramParser, ErrorLocation,
};
use puml_utils::LogLevel;

#[derive(Debug, Error)]
pub enum ActivityParserError {
    #[error(transparent)]
    Base(#[from] BaseParseError<Rule>),
    #[error("invalid activity statement: {0}")]
    InvalidStatement(String),
}

impl ErrorLocation for ActivityParserError {
    fn error_location(&self) -> Option<(usize, usize)> {
        match self {
            Self::Base(base) => base.error_location(),
            _ => None,
        }
    }
}

pub struct PumlActivityParser;

impl PumlActivityParser {
    fn pair_source(pair: &pest::iterators::Pair<Rule>) -> RawActivitySourceSpan {
        let span = pair.as_span();
        let (start_line, start_column) = span.start_pos().line_col();
        let (end_line, end_column) = span.end_pos().line_col();

        RawActivitySourceSpan {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    fn parse_startuml(pair: pest::iterators::Pair<Rule>) -> Option<String> {
        pair.into_inner()
            .find(|inner| inner.as_rule() == Rule::puml_name)
            .map(|inner| inner.as_str().trim().to_string())
    }

    fn parse_statement(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<RawActivityStmt>, ActivityParserError> {
        let source = Self::pair_source(&pair);
        let statement = match pair.as_rule() {
            Rule::title_stmt => RawActivityStmt::Title(Self::parse_title_stmt(pair, source)?),
            Rule::action_stmt => RawActivityStmt::Action(Self::parse_action_stmt(pair, source)?),
            Rule::arrow_stmt => RawActivityStmt::Arrow(Self::parse_arrow_stmt(pair, source)?),
            Rule::backward_stmt => {
                RawActivityStmt::Backward(Self::parse_backward_stmt(pair, source)?)
            }
            Rule::control_stmt => {
                RawActivityStmt::Control(Self::parse_control_stmt(pair, source)?)
            }
            Rule::start_stmt => RawActivityStmt::Start(Self::parse_start_stmt(source)?),
            Rule::stop_stmt => RawActivityStmt::Stop(Self::parse_stop_stmt(source)?),
            Rule::if_start_stmt => {
                RawActivityStmt::IfStart(Self::parse_if_start_stmt(pair, source)?)
            }
            Rule::else_stmt => RawActivityStmt::Else(Self::parse_else_stmt(pair, source)?),
            Rule::endif_stmt => RawActivityStmt::EndIf(Self::parse_endif_stmt(source)?),
            Rule::while_start_stmt => {
                RawActivityStmt::WhileStart(Self::parse_while_start_stmt(pair, source)?)
            }
            Rule::endwhile_stmt => {
                RawActivityStmt::EndWhile(Self::parse_endwhile_stmt(pair, source)?)
            }
            Rule::repeat_start_stmt => {
                RawActivityStmt::RepeatStart(Self::parse_repeat_start_stmt(source)?)
            }
            Rule::repeat_while_stmt => {
                RawActivityStmt::RepeatWhile(Self::parse_repeat_while_stmt(pair, source)?)
            }
            Rule::fork_start_stmt => {
                RawActivityStmt::ForkStart(Self::parse_fork_start_stmt(source)?)
            }
            Rule::fork_again_stmt => {
                RawActivityStmt::ForkAgain(Self::parse_fork_again_stmt(source)?)
            }
            Rule::fork_end_stmt => RawActivityStmt::ForkEnd(Self::parse_fork_end_stmt(pair, source)?),
            Rule::swimlane_stmt => RawActivityStmt::Swimlane(Self::parse_swimlane_stmt(pair, source)?),
            _ => return Ok(vec![]),
        };

        Ok(vec![statement])
    }

    fn parse_title_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<TitleStmt, ActivityParserError> {
        let raw = pair.as_str().trim();
        let text = raw
            .get(5..)
            .map(|value| normalize_creole_text(value.trim()))
            .ok_or_else(|| {
                ActivityParserError::InvalidStatement("missing title text".to_string())
            })?;

        Ok(TitleStmt { text, source })
    }

    fn parse_arrow_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<ArrowStmt, ActivityParserError> {
        let mut syntax = None;
        let mut label = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::connection_arrow => {
                    syntax = Some(inner.as_str().trim().to_string());
                }
                Rule::arrow_text => {
                    label = Some(normalize_creole_text(inner.as_str().trim()));
                }
                _ => {}
            }
        }

        Ok(ArrowStmt {
            syntax: syntax.ok_or_else(|| {
                ActivityParserError::InvalidStatement("missing arrow syntax".to_string())
            })?,
            label,
            source,
        })
    }

    fn parse_action_label(
        pair: pest::iterators::Pair<Rule>,
        statement_kind: &str,
    ) -> Result<String, ActivityParserError> {
        pair.into_inner()
            .find(|inner| matches!(inner.as_rule(), Rule::action_text | Rule::action_line_text))
            .map(|inner| normalize_creole_text(inner.as_str().trim()))
            .ok_or_else(|| {
                ActivityParserError::InvalidStatement(format!("missing {} label", statement_kind,))
            })
    }

    fn parse_action_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<ActionStmt, ActivityParserError> {
        let label = Self::parse_action_label(pair, "action")?;

        Ok(ActionStmt { label, source })
    }

    fn parse_backward_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<BackwardStmt, ActivityParserError> {
        let label = Self::parse_action_label(pair, "backward")?;

        Ok(BackwardStmt { label, source })
    }

    fn parse_start_stmt(source: RawActivitySourceSpan) -> Result<StartStmt, ActivityParserError> {
        Ok(StartStmt { source })
    }

    fn parse_stop_stmt(source: RawActivitySourceSpan) -> Result<StopStmt, ActivityParserError> {
        Ok(StopStmt { source })
    }

    fn parse_control_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<ControlStmt, ActivityParserError> {
        let kind = match pair.as_str().trim() {
            "break" => ControlKind::Break,
            "kill" => ControlKind::Kill,
            "detach" => ControlKind::Detach,
            _ => {
                return Err(ActivityParserError::InvalidStatement(format!(
                    "invalid control kind: {}",
                    pair.as_str().trim(),
                )))
            }
        };

        Ok(ControlStmt { kind, source })
    }

    fn parse_if_start_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<IfStartStmt, ActivityParserError> {
        let mut condition = None;
        let mut label = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::condition_text if condition.is_none() => {
                    condition = Some(normalize_creole_text(inner.as_str().trim()));
                }
                Rule::if_pre_label => {
                    label = inner
                        .into_inner()
                        .find(|nested| nested.as_rule() == Rule::condition_text)
                        .map(|nested| normalize_creole_text(nested.as_str().trim()));
                }
                Rule::then_label => {
                    label = inner
                        .into_inner()
                        .find(|nested| nested.as_rule() == Rule::condition_text)
                        .map(|nested| normalize_creole_text(nested.as_str().trim()));
                }
                _ => {}
            }
        }

        Ok(IfStartStmt {
            condition: condition.ok_or_else(|| {
                ActivityParserError::InvalidStatement("missing if condition".to_string())
            })?,
            label,
            source,
        })
    }

    fn parse_else_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<ElseStmt, ActivityParserError> {
        let label = pair
            .into_inner()
            .find(|inner| inner.as_rule() == Rule::condition_text)
            .map(|inner| normalize_creole_text(inner.as_str().trim()));

        Ok(ElseStmt { label, source })
    }

    fn parse_endif_stmt(source: RawActivitySourceSpan) -> Result<EndIfStmt, ActivityParserError> {
        Ok(EndIfStmt { source })
    }

    fn parse_while_start_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<WhileStartStmt, ActivityParserError> {
        let mut condition = None;
        let mut label = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::condition_text if condition.is_none() => {
                    condition = Some(normalize_creole_text(inner.as_str().trim()));
                }
                Rule::while_label => {
                    label = inner
                        .into_inner()
                        .find(|nested| nested.as_rule() == Rule::condition_text)
                        .map(|nested| normalize_creole_text(nested.as_str().trim()));
                }
                _ => {}
            }
        }

        Ok(WhileStartStmt {
            condition: condition.ok_or_else(|| {
                ActivityParserError::InvalidStatement("missing while condition".to_string())
            })?,
            label,
            source,
        })
    }

    fn parse_endwhile_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<EndWhileStmt, ActivityParserError> {
        let label = pair
            .into_inner()
            .find(|inner| inner.as_rule() == Rule::endwhile_label)
            .and_then(|inner| {
                inner
                    .into_inner()
                    .find(|nested| nested.as_rule() == Rule::condition_text)
            })
            .map(|inner| normalize_creole_text(inner.as_str().trim()));

        Ok(EndWhileStmt { label, source })
    }

    fn parse_repeat_start_stmt(
        source: RawActivitySourceSpan,
    ) -> Result<RepeatStartStmt, ActivityParserError> {
        Ok(RepeatStartStmt { source })
    }

    fn parse_repeat_while_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<RepeatWhileStmt, ActivityParserError> {
        let mut condition = None;
        let mut label = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::condition_text if condition.is_none() => {
                    condition = Some(normalize_creole_text(inner.as_str().trim()));
                }
                Rule::repeat_label => {
                    label = inner
                        .into_inner()
                        .find(|nested| nested.as_rule() == Rule::condition_text)
                        .map(|nested| normalize_creole_text(nested.as_str().trim()));
                }
                _ => {}
            }
        }

        Ok(RepeatWhileStmt {
            condition: condition.ok_or_else(|| {
                ActivityParserError::InvalidStatement("missing repeat while condition".to_string())
            })?,
            label,
            source,
        })
    }

    fn parse_fork_start_stmt(
        source: RawActivitySourceSpan,
    ) -> Result<ForkStartStmt, ActivityParserError> {
        Ok(ForkStartStmt { source })
    }

    fn parse_fork_again_stmt(
        source: RawActivitySourceSpan,
    ) -> Result<ForkAgainStmt, ActivityParserError> {
        Ok(ForkAgainStmt { source })
    }

    fn parse_fork_end_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<ForkEndStmt, ActivityParserError> {
        let kind = if pair.as_str().contains("merge") {
            ForkEndKind::EndMerge
        } else {
            ForkEndKind::EndFork
        };
        let mut modifier = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::fork_modifier {
                let text = inner.as_str();
                modifier = if text.contains("and") {
                    Some(ForkModifier::And)
                } else if text.contains("or") {
                    Some(ForkModifier::Or)
                } else {
                    return Err(ActivityParserError::InvalidStatement(format!(
                        "invalid fork modifier: {}",
                        text,
                    )));
                };
            }
        }

        Ok(ForkEndStmt {
            kind,
            modifier,
            source,
        })
    }

    fn parse_swimlane_stmt(
        pair: pest::iterators::Pair<Rule>,
        source: RawActivitySourceSpan,
    ) -> Result<SwimlaneStmt, ActivityParserError> {
        let name = pair
            .into_inner()
            .find(|inner| inner.as_rule() == Rule::swimlane_text)
            .map(|inner| normalize_creole_text(inner.as_str().trim()))
            .ok_or_else(|| {
                ActivityParserError::InvalidStatement("missing swimlane name".to_string())
            })?;

        Ok(SwimlaneStmt { name, source })
    }
}

impl DiagramParser for PumlActivityParser {
    type Output = RawActivityDiagram;
    type Error = ActivityParserError;

    fn parse_file(
        &mut self,
        path: &Rc<PathBuf>,
        content: &str,
        log_level: LogLevel,
    ) -> Result<Self::Output, Self::Error> {
        let pairs = PlantUmlCommonParser::parse(Rule::activity_diagram, content)
            .map_err(|error| pest_to_syntax_error(error, path.as_ref().clone(), content))?;

        #[cfg(not(coverage))]
        if matches!(log_level, LogLevel::Debug | LogLevel::Trace) {
            let mut tree_output = String::new();
            format_parse_tree(pairs.clone(), 0, &mut tree_output);

            debug!(
                "\n=== Parse Tree for {} ===\n{}=== End Parse Tree ===",
                path.display(),
                tree_output
            );
        }

        let mut document = RawActivityDiagram {
            name: None,
            statements: Vec::new(),
        };

        for pair in pairs {
            for inner_pair in pair.into_inner() {
                match inner_pair.as_rule() {
                    Rule::startuml => {
                        document.name = Self::parse_startuml(inner_pair);
                    }
                    Rule::title_stmt
                    | Rule::arrow_stmt
                    | Rule::backward_stmt
                    | Rule::action_stmt
                    | Rule::control_stmt
                    | Rule::start_stmt
                    | Rule::stop_stmt
                    | Rule::if_start_stmt
                    | Rule::else_stmt
                    | Rule::endif_stmt
                    | Rule::while_start_stmt
                    | Rule::endwhile_stmt
                    | Rule::repeat_start_stmt
                    | Rule::repeat_while_stmt
                    | Rule::fork_start_stmt
                    | Rule::fork_again_stmt
                    | Rule::fork_end_stmt
                    | Rule::swimlane_stmt => {
                        let mut statements = Self::parse_statement(inner_pair)?;
                        document.statements.append(&mut statements);
                    }
                    _ => {}
                }
            }
        }

        Ok(document)
    }
}
