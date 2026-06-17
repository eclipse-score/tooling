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

use crate::activity_logic::{
    ActionNode, ActivityDiagram, ActivityStmt, BackwardNode, ControlKind, ControlNode, IfDisplay,
    IfNode, LoopDisplay, RepeatWhileNode, TitleNode, WhileNode,
};
use activity_parser::{RawActivityDiagram, RawActivitySourceSpan, RawActivityStmt, RawControlKind};
use resolver_traits::DiagramResolver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityResolverContext {
    Statement,
    ElseIfChain,
    IfBlock,
    WhileLoop,
    RepeatLoop,
}

impl ActivityResolverContext {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Statement => "statement",
            Self::ElseIfChain => "elseif chain",
            Self::IfBlock => "if block",
            Self::WhileLoop => "while loop",
            Self::RepeatLoop => "repeat loop",
        }
    }
}

impl std::fmt::Display for ActivityResolverContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ActivityResolverError {
    #[error("unexpected end of input while parsing {context} at line {line}, column {column}")]
    UnexpectedEndOfInput {
        context: ActivityResolverContext,
        line: usize,
        column: usize,
    },
    #[error(
        "unexpected statement {statement} while parsing {context} at line {line}, column {column}"
    )]
    UnexpectedStatement {
        context: ActivityResolverContext,
        statement: &'static str,
        line: usize,
        column: usize,
    },
    #[error("unsupported activity statement {statement} at line {line}, column {column}")]
    UnsupportedStatement {
        statement: &'static str,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Default)]
pub struct ActivityResolver {
    cursor: usize,
}

impl ActivityResolver {
    pub fn new() -> Self {
        Self::default()
    }

    fn current_statement_location(
        &self,
        statements: &[RawActivityStmt],
    ) -> Option<RawActivitySourceSpan> {
        self.peek_statement(statements).map(RawActivityStmt::span)
    }

    fn previous_statement_location(
        &self,
        statements: &[RawActivityStmt],
    ) -> Option<RawActivitySourceSpan> {
        self.cursor
            .checked_sub(1)
            .and_then(|index| statements.get(index))
            .map(RawActivityStmt::span)
    }

    fn resolve_error_location(
        &self,
        statements: &[RawActivityStmt],
        fallback: Option<RawActivitySourceSpan>,
    ) -> (usize, usize) {
        fallback
            .or_else(|| self.current_statement_location(statements))
            .or_else(|| self.previous_statement_location(statements))
            .map(|source| (source.start_line, source.start_column))
            .unwrap_or((1, 1))
    }

    fn unexpected_end_of_input(
        &self,
        statements: &[RawActivityStmt],
        context: ActivityResolverContext,
        fallback: Option<RawActivitySourceSpan>,
    ) -> ActivityResolverError {
        let (line, column) = self.resolve_error_location(statements, fallback);

        ActivityResolverError::UnexpectedEndOfInput {
            context,
            line,
            column,
        }
    }

    fn unexpected_statement(
        &self,
        context: ActivityResolverContext,
        statement: &RawActivityStmt,
    ) -> ActivityResolverError {
        let source = statement.span();
        let (line, column) = (source.start_line, source.start_column);

        ActivityResolverError::UnexpectedStatement {
            context,
            statement: raw_statement_name(statement),
            line,
            column,
        }
    }

    fn unsupported_statement(&self, statement: &RawActivityStmt) -> ActivityResolverError {
        let source = statement.span();
        let (line, column) = (source.start_line, source.start_column);

        ActivityResolverError::UnsupportedStatement {
            statement: raw_statement_name(statement),
            line,
            column,
        }
    }

    fn normalize_logic_text(text: &str) -> String {
        text.replace("\\n", "\n")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn normalize_optional_logic_text(text: Option<&str>) -> Option<String> {
        text.map(Self::normalize_logic_text)
    }

    fn visit_statement(
        &mut self,
        statements: &[RawActivityStmt],
    ) -> Result<Option<ActivityStmt>, ActivityResolverError> {
        let statement = self.next_statement(statements).ok_or_else(|| {
            self.unexpected_end_of_input(statements, ActivityResolverContext::Statement, None)
        })?;

        match statement {
            RawActivityStmt::Title(title) => Ok(Some(ActivityStmt::Title(TitleNode {
                text: Self::normalize_logic_text(&title.text),
            }))),
            RawActivityStmt::Arrow(_) | RawActivityStmt::Start(_) => Ok(None),
            RawActivityStmt::Action(action) => Ok(Some(ActivityStmt::Action(ActionNode {
                label: Self::normalize_logic_text(&action.label),
            }))),
            RawActivityStmt::Stop(_) => Ok(Some(ActivityStmt::Control(ControlNode {
                kind: ControlKind::Stop,
            }))),
            RawActivityStmt::Control(control) => Ok(Some(ActivityStmt::Control(ControlNode {
                kind: Self::resolve_control_kind(&control.kind),
            }))),
            RawActivityStmt::IfStart(if_start) => Ok(Some(ActivityStmt::If(self.visit_if(
                statements,
                Self::normalize_logic_text(&if_start.condition),
                Self::normalize_optional_logic_text(if_start.label.as_deref()),
                statement.span(),
            )?))),
            RawActivityStmt::WhileStart(while_start) => {
                Ok(Some(ActivityStmt::While(self.visit_while(
                    statements,
                    Self::normalize_logic_text(&while_start.condition),
                    Self::normalize_optional_logic_text(while_start.label.as_deref()),
                    statement.span(),
                )?)))
            }
            RawActivityStmt::RepeatStart(_) => Ok(Some(ActivityStmt::RepeatWhile(
                self.visit_repeat_while(statements, statement.span())?,
            ))),
            RawActivityStmt::Else(_)
            | RawActivityStmt::EndIf(_)
            | RawActivityStmt::EndWhile(_)
            | RawActivityStmt::RepeatWhile(_)
            | RawActivityStmt::Backward(_) => {
                Err(self.unexpected_statement(ActivityResolverContext::Statement, statement))
            }
            RawActivityStmt::ForkStart(_)
            | RawActivityStmt::ForkAgain(_)
            | RawActivityStmt::ForkEnd(_)
            | RawActivityStmt::Swimlane(_) => Err(self.unsupported_statement(statement)),
        }
    }

    fn visit_block_until<F>(
        &mut self,
        statements: &[RawActivityStmt],
        terminator: F,
    ) -> Result<Vec<ActivityStmt>, ActivityResolverError>
    where
        F: Fn(&RawActivityStmt) -> bool,
    {
        let mut resolved = Vec::new();

        while let Some(statement) = self.peek_statement(statements) {
            if terminator(statement) {
                break;
            }

            if let Some(stmt) = self.visit_statement(statements)? {
                resolved.push(stmt);
            }
        }

        Ok(resolved)
    }

    fn visit_loop_body_until<F>(
        &mut self,
        statements: &[RawActivityStmt],
        terminator: F,
    ) -> Result<(Vec<ActivityStmt>, Option<BackwardNode>), ActivityResolverError>
    where
        F: Fn(&RawActivityStmt) -> bool,
    {
        let mut body = Vec::new();
        let mut backward = None;

        while let Some(statement) = self.peek_statement(statements) {
            if terminator(statement) {
                break;
            }

            if let RawActivityStmt::Backward(backward_stmt) = statement {
                // Match PlantUML preview behavior: the last backward in the loop wins.
                backward = Some(BackwardNode {
                    label: Self::normalize_logic_text(&backward_stmt.label),
                });
                self.cursor += 1;
                continue;
            }

            if let Some(stmt) = self.visit_statement(statements)? {
                body.push(stmt);
            }
        }

        Ok((body, backward))
    }

    fn visit_if(
        &mut self,
        statements: &[RawActivityStmt],
        condition: String,
        then_label: Option<String>,
        if_location: RawActivitySourceSpan,
    ) -> Result<IfNode, ActivityResolverError> {
        let body = self.visit_block_until(statements, |statement| {
            matches!(
                statement,
                RawActivityStmt::Else(_) | RawActivityStmt::EndIf(_)
            )
        })?;

        let mut else_label = None;
        let mut else_branch = Vec::new();
        let mut endif_consumed_by_elseif = false;

        if let Some(RawActivityStmt::Else(else_stmt)) = self.peek_statement(statements) {
            else_label = Self::normalize_optional_logic_text(else_stmt.label.as_deref());
            self.cursor += 1;

            if matches!(
                self.peek_statement(statements),
                Some(RawActivityStmt::IfStart(_))
            ) {
                else_branch.push(ActivityStmt::If(self.visit_elseif_chain(statements)?));
                endif_consumed_by_elseif = true;
            } else {
                else_branch = self.visit_block_until(statements, |statement| {
                    matches!(statement, RawActivityStmt::EndIf(_))
                })?;
            }
        }

        if !endif_consumed_by_elseif {
            self.consume_endif(statements, if_location)?;
        }

        let display = if then_label.is_some() || else_label.is_some() {
            Some(IfDisplay {
                then_label,
                else_label,
            })
        } else {
            None
        };

        Ok(IfNode {
            condition,
            body,
            else_branch,
            display,
        })
    }

    fn visit_elseif_chain(
        &mut self,
        statements: &[RawActivityStmt],
    ) -> Result<IfNode, ActivityResolverError> {
        let statement = self.next_statement(statements).ok_or_else(|| {
            self.unexpected_end_of_input(statements, ActivityResolverContext::ElseIfChain, None)
        })?;

        match statement {
            RawActivityStmt::IfStart(if_start) => self.visit_if(
                statements,
                Self::normalize_logic_text(&if_start.condition),
                Self::normalize_optional_logic_text(if_start.label.as_deref()),
                statement.span(),
            ),
            other => Err(self.unexpected_statement(ActivityResolverContext::ElseIfChain, other)),
        }
    }

    fn visit_while(
        &mut self,
        statements: &[RawActivityStmt],
        condition: String,
        continue_label: Option<String>,
        while_location: RawActivitySourceSpan,
    ) -> Result<WhileNode, ActivityResolverError> {
        let (body, backward) = self.visit_loop_body_until(statements, |statement| {
            matches!(statement, RawActivityStmt::EndWhile(_))
        })?;

        let exit_label = self.consume_endwhile_label(statements, while_location)?;
        let display = if continue_label.is_some() || exit_label.is_some() {
            Some(LoopDisplay {
                continue_label,
                exit_label,
            })
        } else {
            None
        };

        Ok(WhileNode {
            condition,
            body,
            backward,
            display,
        })
    }

    fn visit_repeat_while(
        &mut self,
        statements: &[RawActivityStmt],
        repeat_location: RawActivitySourceSpan,
    ) -> Result<RepeatWhileNode, ActivityResolverError> {
        let (body, backward) = self.visit_loop_body_until(statements, |statement| {
            matches!(statement, RawActivityStmt::RepeatWhile(_))
        })?;

        let (condition, continue_label) =
            self.consume_repeat_while_data(statements, repeat_location)?;
        let display = continue_label.as_ref().map(|label| LoopDisplay {
            continue_label: Some(label.clone()),
            exit_label: None,
        });

        Ok(RepeatWhileNode {
            body,
            condition,
            backward,
            display,
        })
    }

    fn consume_endif(
        &mut self,
        statements: &[RawActivityStmt],
        if_location: RawActivitySourceSpan,
    ) -> Result<(), ActivityResolverError> {
        match self.next_statement(statements) {
            Some(RawActivityStmt::EndIf(_)) => Ok(()),
            Some(statement) => {
                Err(self.unexpected_statement(ActivityResolverContext::IfBlock, statement))
            }
            None => Err(self.unexpected_end_of_input(
                statements,
                ActivityResolverContext::IfBlock,
                Some(if_location),
            )),
        }
    }

    fn consume_endwhile_label(
        &mut self,
        statements: &[RawActivityStmt],
        while_location: RawActivitySourceSpan,
    ) -> Result<Option<String>, ActivityResolverError> {
        match self.next_statement(statements) {
            Some(RawActivityStmt::EndWhile(endwhile)) => Ok(Self::normalize_optional_logic_text(
                endwhile.label.as_deref(),
            )),
            Some(statement) => {
                Err(self.unexpected_statement(ActivityResolverContext::WhileLoop, statement))
            }
            None => Err(self.unexpected_end_of_input(
                statements,
                ActivityResolverContext::WhileLoop,
                Some(while_location),
            )),
        }
    }

    fn consume_repeat_while_data(
        &mut self,
        statements: &[RawActivityStmt],
        repeat_location: RawActivitySourceSpan,
    ) -> Result<(String, Option<String>), ActivityResolverError> {
        match self.next_statement(statements) {
            Some(RawActivityStmt::RepeatWhile(repeat_while)) => Ok((
                Self::normalize_logic_text(&repeat_while.condition),
                Self::normalize_optional_logic_text(repeat_while.label.as_deref()),
            )),
            Some(statement) => {
                Err(self.unexpected_statement(ActivityResolverContext::RepeatLoop, statement))
            }
            None => Err(self.unexpected_end_of_input(
                statements,
                ActivityResolverContext::RepeatLoop,
                Some(repeat_location),
            )),
        }
    }

    fn peek_statement<'a>(&self, statements: &'a [RawActivityStmt]) -> Option<&'a RawActivityStmt> {
        statements.get(self.cursor)
    }

    fn next_statement<'a>(
        &mut self,
        statements: &'a [RawActivityStmt],
    ) -> Option<&'a RawActivityStmt> {
        let statement = statements.get(self.cursor)?;
        self.cursor += 1;
        Some(statement)
    }

    fn resolve_control_kind(kind: &RawControlKind) -> ControlKind {
        match kind {
            RawControlKind::Break => ControlKind::Break,
            RawControlKind::Kill => ControlKind::Kill,
            RawControlKind::Detach => ControlKind::Detach,
        }
    }
}

impl DiagramResolver for ActivityResolver {
    type Document = RawActivityDiagram;
    type Output = ActivityDiagram;
    type Error = ActivityResolverError;

    fn resolve(&mut self, document: &Self::Document) -> Result<Self::Output, Self::Error> {
        self.cursor = 0;
        let statements = self.visit_block_until(&document.statements, |_| false)?;

        Ok(ActivityDiagram {
            name: document.name.clone(),
            statements,
        })
    }
}

fn raw_statement_name(statement: &RawActivityStmt) -> &'static str {
    match statement {
        RawActivityStmt::Title(_) => "title",
        RawActivityStmt::Action(_) => "action",
        RawActivityStmt::Arrow(_) => "arrow",
        RawActivityStmt::Backward(_) => "backward",
        RawActivityStmt::Start(_) => "start",
        RawActivityStmt::Stop(_) => "stop",
        RawActivityStmt::Control(_) => "control",
        RawActivityStmt::IfStart(_) => "if",
        RawActivityStmt::Else(_) => "else",
        RawActivityStmt::EndIf(_) => "endif",
        RawActivityStmt::WhileStart(_) => "while",
        RawActivityStmt::EndWhile(_) => "endwhile",
        RawActivityStmt::RepeatStart(_) => "repeat",
        RawActivityStmt::RepeatWhile(_) => "repeat while",
        RawActivityStmt::ForkStart(_) => "fork",
        RawActivityStmt::ForkAgain(_) => "fork again",
        RawActivityStmt::ForkEnd(_) => "end fork",
        RawActivityStmt::Swimlane(_) => "swimlane",
    }
}
