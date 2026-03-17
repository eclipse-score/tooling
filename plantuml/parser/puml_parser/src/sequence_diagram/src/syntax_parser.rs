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
use parser_core::common_parser::parse_arrow as common_parse_arrow;
use parser_core::common_parser::{PlantUmlCommonParser, Rule};
use parser_core::{pest_to_syntax_error, BaseParseError, DiagramParser};
use puml_utils::LogLevel;
use std::path::PathBuf;
use std::rc::Rc;

use crate::syntax_ast::*;

pub struct PumlSequenceParser;

// lobster-trace: Tools.ArchitectureModelingSyntax
// lobster-trace: Tools.ArchitectureModelingSequenceContentActors
// lobster-trace: Tools.ArchitectureModelingSequenceContentSWUnits
// lobster-trace: Tools.ArchitectureModelingSequenceContentMessages
// lobster-trace: Tools.ArchitectureModelingSequenceContentActivity
impl PumlSequenceParser {
    fn parse_startuml(pair: pest::iterators::Pair<Rule>) -> Option<String> {
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::puml_name {
                return Some(inner.as_str().trim().to_string());
            }
        }
        None
    }

    fn parse_statement(pair: pest::iterators::Pair<Rule>) -> Option<Statement> {
        let inner = pair.into_inner().next()?;
        match inner.as_rule() {
            Rule::participant_def => Some(Statement::ParticipantDef(Self::parse_participant_def(
                inner,
            )?)),
            Rule::message => Some(Statement::Message(Self::parse_message(inner)?)),
            Rule::group_cmd => Some(Statement::GroupCmd(Self::parse_group_cmd(inner)?)),
            Rule::destroy_cmd => Some(Statement::DestroyCmd(Self::parse_destroy_cmd(inner)?)),
            Rule::create_cmd => Some(Statement::CreateCmd(Self::parse_create_cmd(inner)?)),
            Rule::activate_cmd => Some(Statement::ActivateCmd(Self::parse_activate_cmd(inner)?)),
            Rule::deactivate_cmd => {
                Some(Statement::DeactivateCmd(Self::parse_deactivate_cmd(inner)?))
            }
            _ => None,
        }
    }

    fn parse_participant_def(pair: pest::iterators::Pair<Rule>) -> Option<ParticipantDef> {
        let mut participant_type: Option<ParticipantType> = None;
        let mut identifier: Option<ParticipantIdentifier> = None;
        let mut stereotype: Option<String> = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::create_kw => {
                    // Handle create keyword if needed
                }
                Rule::participant_type => {
                    participant_type = Self::parse_participant_type(inner);
                }
                Rule::quoted_participant_as_id => {
                    let mut parts = inner.into_inner();
                    let quoted = parts
                        .next()
                        .map(|p| Self::extract_quoted_string(p.as_str()))?;
                    let alias_clause = parts.next()?; // alias_clause
                    let id_pair = alias_clause.into_inner().next()?;
                    let id = match id_pair.as_rule() {
                        Rule::quoted_string => Self::extract_quoted_string(id_pair.as_str()),
                        _ => id_pair.as_str().trim().to_string(),
                    };
                    identifier = Some(ParticipantIdentifier::QuotedAsId { quoted, id });
                }
                Rule::participant_id_as_quoted => {
                    let mut parts = inner.into_inner();
                    let id = parts.next()?.as_str().trim().to_string();
                    let alias_clause = parts.next()?; // alias_clause
                    let quoted_pair = alias_clause.into_inner().next()?;
                    let quoted = Self::extract_quoted_string(quoted_pair.as_str());
                    identifier = Some(ParticipantIdentifier::IdAsQuoted { id, quoted });
                }
                Rule::participant_id_as_id => {
                    let mut parts = inner.into_inner();
                    let id1 = parts.next()?.as_str().trim().to_string();
                    let alias_clause = parts.next()?; // alias_clause
                    let id2_pair = alias_clause.into_inner().next()?;
                    let id2 = id2_pair.as_str().trim().to_string();
                    identifier = Some(ParticipantIdentifier::IdAsId { id1, id2 });
                }
                Rule::quoted_participant => {
                    let quoted = Self::extract_quoted_string(inner.as_str());
                    identifier = Some(ParticipantIdentifier::Quoted(quoted));
                }
                Rule::participant_id => {
                    let id = inner.as_str().trim().to_string();
                    identifier = Some(ParticipantIdentifier::Id(id));
                }
                Rule::stereotype => {
                    stereotype = Some(Self::extract_stereotype(inner.as_str()));
                }
                Rule::order_clause => {
                    // Ignore this for now
                }
                _ => {}
            }
        }

        Some(ParticipantDef {
            participant_type: participant_type?,
            identifier: identifier?,
            stereotype,
        })
    }

    fn parse_participant_type(pair: pest::iterators::Pair<Rule>) -> Option<ParticipantType> {
        let text = pair.as_str().to_lowercase();
        match text.as_str() {
            "participant" => Some(ParticipantType::Participant),
            "actor" => Some(ParticipantType::Actor),
            "boundary" => Some(ParticipantType::Boundary),
            "control" => Some(ParticipantType::Control),
            "entity" => Some(ParticipantType::Entity),
            "queue" => Some(ParticipantType::Queue),
            "database" => Some(ParticipantType::Database),
            "collections" => Some(ParticipantType::Collections),
            _ => None,
        }
    }

    fn parse_message(pair: pest::iterators::Pair<Rule>) -> Option<Message> {
        let mut left: Option<String> = None;
        let mut arrow: Option<Arrow> = None;
        let mut right: Option<String> = None;
        let mut activation_marker: Option<String> = None;
        let mut description: Option<String> = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::message_participant => {
                    let participant = Self::extract_participant_ref(inner);
                    // First participant goes to left, second to right
                    if arrow.is_none() {
                        left = Some(participant);
                    } else {
                        right = Some(participant);
                    }
                }
                Rule::sequence_arrow => {
                    arrow = Self::parse_arrow(inner);
                }
                Rule::activation_marker => {
                    activation_marker = Some(inner.as_str().to_string());
                }
                Rule::sequence_description => {
                    description = Some(inner.into_inner().next()?.as_str().trim().to_string());
                }
                _ => {}
            }
        }

        let content = MessageContent::WithTargets {
            left: left.unwrap_or_default(),
            arrow: arrow?,
            right: right.unwrap_or_default(),
        };

        Some(Message {
            content,
            activation_marker,
            description,
        })
    }

    fn parse_arrow(pair: pest::iterators::Pair<Rule>) -> Option<Arrow> {
        let arrow = common_parse_arrow(pair).ok()?;

        Some(arrow)
    }

    fn parse_group_cmd(pair: pest::iterators::Pair<Rule>) -> Option<GroupCmd> {
        let mut group_type: Option<GroupType> = None;
        let mut text: Option<String> = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::group_type => {
                    group_type = Self::parse_group_type(inner);
                }
                Rule::group_condition => {
                    text = Some(inner.as_str().trim().to_string());
                }
                _ => {}
            }
        }

        Some(GroupCmd {
            group_type: group_type?,
            text,
        })
    }

    fn parse_group_type(pair: pest::iterators::Pair<Rule>) -> Option<GroupType> {
        let text = pair.as_str().to_lowercase();
        match text.as_str() {
            "opt" => Some(GroupType::Opt),
            "alt" => Some(GroupType::Alt),
            "loop" => Some(GroupType::Loop),
            "par" => Some(GroupType::Par),
            "par2" => Some(GroupType::Par2),
            "break" => Some(GroupType::Break),
            "critical" => Some(GroupType::Critical),
            "else" => Some(GroupType::Else),
            "also" => Some(GroupType::Also),
            "end" => Some(GroupType::End),
            "group" => Some(GroupType::Group),
            _ => None,
        }
    }

    fn parse_destroy_cmd(pair: pest::iterators::Pair<Rule>) -> Option<DestroyCmd> {
        let mut participant: Option<String> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::extract_participant_ref(inner));
            }
        }

        Some(DestroyCmd {
            participant: participant?,
        })
    }

    fn parse_create_cmd(pair: pest::iterators::Pair<Rule>) -> Option<CreateCmd> {
        let mut participant: Option<String> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::extract_participant_ref(inner));
            }
        }

        Some(CreateCmd {
            participant: participant?,
        })
    }

    fn parse_activate_cmd(pair: pest::iterators::Pair<Rule>) -> Option<ActivateCmd> {
        let mut participant: Option<String> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::extract_participant_ref(inner));
            }
        }

        Some(ActivateCmd {
            participant: participant?,
        })
    }

    fn parse_deactivate_cmd(pair: pest::iterators::Pair<Rule>) -> Option<DeactivateCmd> {
        let mut participant: Option<String> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::extract_participant_ref(inner));
            }
        }

        Some(DeactivateCmd { participant })
    }

    // Helper functions
    fn extract_quoted_string(s: &str) -> String {
        s.trim()
            .trim_start_matches('"')
            .trim_end_matches('"')
            .trim_start_matches('«')
            .trim_end_matches('»')
            .to_string()
    }

    fn extract_stereotype(s: &str) -> String {
        s.trim()
            .trim_start_matches("<<")
            .trim_end_matches(">>")
            .to_string()
    }

    fn extract_participant_ref(pair: pest::iterators::Pair<Rule>) -> String {
        match pair.as_rule() {
            Rule::message_participant => pair
                .into_inner()
                .next()
                .map(Self::extract_participant_ref)
                .unwrap_or_default(),

            Rule::participant_ref => {
                let fallback = pair.as_str().trim().to_string();

                pair.into_inner()
                    .next()
                    .map(Self::extract_participant_ref)
                    .unwrap_or(fallback)
            }

            Rule::quoted_string => Self::extract_quoted_string(pair.as_str()),

            Rule::quoted_participant_as_id
            | Rule::participant_id_as_quoted
            | Rule::participant_id_as_id => {
                let mut inner = pair.into_inner();

                inner.next(); // skip lhs

                let alias_clause = inner.next().unwrap();

                let target = alias_clause.into_inner().next().unwrap();

                match target.as_rule() {
                    Rule::quoted_string => Self::extract_quoted_string(target.as_str()),
                    _ => target.as_str().trim().to_string(),
                }
            }

            _ => pair.as_str().trim().to_string(),
        }
    }
}

impl DiagramParser for PumlSequenceParser {
    type Output = SeqPumlDocument;
    type Error = BaseParseError<Rule>;

    fn parse_file(
        &mut self,
        path: &Rc<PathBuf>,
        content: &str,
        log_level: LogLevel,
    ) -> Result<Self::Output, Self::Error> {
        use pest::Parser;

        // Log file content at trace level
        if matches!(log_level, LogLevel::Trace) {
            eprintln!("{}:\n{}\n{}", path.display(), content, "=".repeat(30));
        }

        let pairs = PlantUmlCommonParser::parse(Rule::sequence_start, content)
            .map_err(|e| pest_to_syntax_error(e, path.as_ref().clone(), content))?;

        let mut document = SeqPumlDocument {
            name: None,
            statements: Vec::new(),
        };

        for pair in pairs {
            if pair.as_rule() == Rule::sequence_start {
                for inner_pair in pair.into_inner() {
                    match inner_pair.as_rule() {
                        Rule::startuml => {
                            document.name = Self::parse_startuml(inner_pair);
                        }
                        Rule::sequence_statement => {
                            if let Some(stmt) = Self::parse_statement(inner_pair) {
                                document.statements.push(stmt);
                            }
                        }
                        Rule::empty_line => {
                            // Skip empty lines
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(document)
    }
}
