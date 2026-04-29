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
use log::{debug, trace};
use parser_core::common_parser::parse_arrow as common_parse_arrow;
use parser_core::common_parser::{PlantUmlCommonParser, Rule};
use parser_core::{format_parse_tree, pest_to_syntax_error, BaseParseError, DiagramParser};
use puml_utils::LogLevel;
use std::path::PathBuf;
use std::rc::Rc;
use thiserror::Error;

use crate::syntax_ast::*;

#[derive(Debug, Error)]
pub enum SequenceError {
    #[error(transparent)]
    Base(#[from] BaseParseError<Rule>),
    #[error("invalid sequence statement: {0}")]
    InvalidStatement(String),
}

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

    fn parse_statement(pair: pest::iterators::Pair<Rule>) -> Result<Vec<Statement>, SequenceError> {
        let inner = pair.into_inner().next().ok_or_else(|| {
            SequenceError::InvalidStatement("empty statement".to_string())
        })?;
        match inner.as_rule() {
            Rule::participant_def => Ok(vec![Statement::ParticipantDef(
                Self::parse_participant_def(inner)?,
            )]),
            Rule::message => Ok(vec![Statement::Message(Self::parse_message(inner)?)]),
            Rule::group_cmd => Ok(vec![Statement::GroupCmd(Self::parse_group_cmd(inner)?)]),
            Rule::destroy_cmd => Ok(vec![Statement::DestroyCmd(Self::parse_destroy_cmd(inner)?)]),
            Rule::create_cmd => Ok(vec![Statement::CreateCmd(Self::parse_create_cmd(inner)?)]),
            Rule::activate_cmd => Ok(vec![Statement::ActivateCmd(Self::parse_activate_cmd(inner)?)]),
            Rule::deactivate_cmd => {
                Ok(vec![Statement::DeactivateCmd(Self::parse_deactivate_cmd(inner)?)])
            }
            // Grammar-valid directives that are intentionally not modeled as statements
            _ => Ok(vec![]),
        }
    }

    fn parse_participant_def(pair: pest::iterators::Pair<Rule>) -> Result<ParticipantDef, SequenceError> {
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
                        .map(|p| Self::extract_quoted_string(p.as_str()))
                        .ok_or_else(|| SequenceError::InvalidStatement("missing quoted participant".to_string()))?;
                    let alias_clause = parts.next().ok_or_else(|| SequenceError::InvalidStatement("missing alias clause".to_string()))?;
                    let id_pair = alias_clause.into_inner().next().ok_or_else(|| SequenceError::InvalidStatement("missing alias id".to_string()))?;
                    let id = match id_pair.as_rule() {
                        Rule::quoted_string => Self::extract_quoted_string(id_pair.as_str()),
                        _ => id_pair.as_str().trim().to_string(),
                    };
                    identifier = Some(ParticipantIdentifier::QuotedAsId { quoted, id });
                }
                Rule::participant_id_as_quoted => {
                    let mut parts = inner.into_inner();
                    let id = parts.next().ok_or_else(|| SequenceError::InvalidStatement("missing participant id".to_string()))?.as_str().trim().to_string();
                    let alias_clause = parts.next().ok_or_else(|| SequenceError::InvalidStatement("missing alias clause".to_string()))?;
                    let quoted_pair = alias_clause.into_inner().next().ok_or_else(|| SequenceError::InvalidStatement("missing quoted alias".to_string()))?;
                    let quoted = Self::extract_quoted_string(quoted_pair.as_str());
                    identifier = Some(ParticipantIdentifier::IdAsQuoted { id, quoted });
                }
                Rule::participant_id_as_id => {
                    let mut parts = inner.into_inner();
                    let id1 = parts.next().ok_or_else(|| SequenceError::InvalidStatement("missing participant id1".to_string()))?.as_str().trim().to_string();
                    let alias_clause = parts.next().ok_or_else(|| SequenceError::InvalidStatement("missing alias clause".to_string()))?;
                    let id2_pair = alias_clause.into_inner().next().ok_or_else(|| SequenceError::InvalidStatement("missing alias id2".to_string()))?;
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

        Ok(ParticipantDef {
            participant_type: participant_type.ok_or_else(|| SequenceError::InvalidStatement("missing participant type".to_string()))?,
            identifier: identifier.ok_or_else(|| SequenceError::InvalidStatement("missing participant identifier".to_string()))?,
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

    fn parse_message(pair: pest::iterators::Pair<Rule>) -> Result<Message, SequenceError> {
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
                    arrow = Some(Self::parse_arrow(inner)?);
                }
                Rule::activation_marker => {
                    activation_marker = Some(inner.as_str().to_string());
                }
                Rule::sequence_description => {
                    description = inner.into_inner().next().map(|p| p.as_str().trim().to_string());
                }
                _ => {}
            }
        }

        let content = MessageContent::WithTargets {
            left: left.unwrap_or_default(),
            arrow: arrow.ok_or_else(|| SequenceError::InvalidStatement("missing arrow in message".to_string()))?,
            right: right.unwrap_or_default(),
        };

        Ok(Message {
            content,
            activation_marker,
            description,
        })
    }

    fn parse_arrow(pair: pest::iterators::Pair<Rule>) -> Result<Arrow, SequenceError> {
        common_parse_arrow(pair).map_err(|e| {
            SequenceError::InvalidStatement(format!("invalid arrow: {}", e))
        })
    }

    fn parse_group_cmd(pair: pest::iterators::Pair<Rule>) -> Result<GroupCmd, SequenceError> {
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

        Ok(GroupCmd {
            group_type: group_type.ok_or_else(|| SequenceError::InvalidStatement("missing group type".to_string()))?,
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

    fn parse_destroy_cmd(pair: pest::iterators::Pair<Rule>) -> Result<DestroyCmd, SequenceError> {
        let mut participant: Option<String> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::extract_participant_ref(inner));
            }
        }

        Ok(DestroyCmd {
            participant: participant.ok_or_else(|| SequenceError::InvalidStatement("missing participant in destroy".to_string()))?,
        })
    }

    fn parse_create_cmd(pair: pest::iterators::Pair<Rule>) -> Result<CreateCmd, SequenceError> {
        let mut participant: Option<String> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::extract_participant_ref(inner));
            }
        }

        Ok(CreateCmd {
            participant: participant.ok_or_else(|| SequenceError::InvalidStatement("missing participant in create".to_string()))?,
        })
    }

    fn parse_activate_cmd(pair: pest::iterators::Pair<Rule>) -> Result<ActivateCmd, SequenceError> {
        let mut participant: Option<String> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::extract_participant_ref(inner));
            }
        }

        Ok(ActivateCmd {
            participant: participant.ok_or_else(|| SequenceError::InvalidStatement("missing participant in activate".to_string()))?,
        })
    }

    fn parse_deactivate_cmd(pair: pest::iterators::Pair<Rule>) -> Result<DeactivateCmd, SequenceError> {
        let mut participant: Option<String> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::extract_participant_ref(inner));
            }
        }

        Ok(DeactivateCmd { participant })
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
    type Error = SequenceError;

    fn parse_file(
        &mut self,
        path: &Rc<PathBuf>,
        content: &str,
        log_level: LogLevel,
    ) -> Result<Self::Output, Self::Error> {
        use pest::Parser;

        // Log file content at trace level
        if matches!(log_level, LogLevel::Trace) {
            trace!("{}:\n{}\n{}", path.display(), content, "=".repeat(30));
        }

        let pairs = PlantUmlCommonParser::parse(Rule::sequence_start, content)
            .map_err(|e| pest_to_syntax_error(e, path.as_ref().clone(), content))?;

        // Debug-only, excluded to keep coverage focused on parser logic.
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
                            let mut stmts = Self::parse_statement(inner_pair)?;
                            document.statements.append(&mut stmts);
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

#[cfg(test)]
mod error_handling_tests {
    use super::*;
    use parser_core::DiagramParser;
    use puml_utils::LogLevel;
    use std::path::PathBuf;
    use std::rc::Rc;

    fn path() -> Rc<PathBuf> {
        Rc::new(PathBuf::from("test.puml"))
    }

    /// A diagram with a known-good participant type must not lose the definition.
    #[test]
    fn test_valid_participant_is_present_in_output() {
        let input = "@startuml\nparticipant Alice\nparticipant Bob\nAlice -> Bob : hello\n@enduml";
        let mut parser = PumlSequenceParser;
        let doc = parser
            .parse_file(&path(), input, LogLevel::Info)
            .expect("valid diagram must parse");

        // 2 participant defs + 1 message = 3 statements
        assert_eq!(
            doc.statements.len(),
            3,
            "all statements must be present; none may be silently dropped"
        );
    }

    /// parse_file must return Err (or log a warning) rather than return an
    /// empty document when the content is semantically malformed.
    #[test]
    fn test_empty_document_on_grammar_failure_is_not_silently_ok() {
        // Completely invalid PlantUML – the grammar must reject it.
        let input = "@startuml\n$$$$invalid$$$$\n@enduml";
        let mut parser = PumlSequenceParser;
        let result = parser.parse_file(&path(), input, LogLevel::Info);
        // Grammar-level rejection must surface as Err, not Ok(empty doc).
        assert!(
            result.is_err(),
            "invalid syntax must produce an error, not a silently-empty document"
        );
    }
}

#[cfg(test)]
mod dispatch_style_tests {
    use super::*;
    use parser_core::DiagramParser;
    use puml_utils::LogLevel;
    use std::path::PathBuf;
    use std::rc::Rc;

    /// Smoke test: the statement count from a two-participant, one-message diagram
    /// must be exactly 3 for the sequence parser.
    #[test]
    fn test_sequence_statement_count() {
        let input = "@startuml\nparticipant A\nparticipant B\nA -> B : call\n@enduml";
        let mut parser = PumlSequenceParser;
        let doc = parser
            .parse_file(&Rc::new(PathBuf::from("t.puml")), input, LogLevel::Info)
            .expect("valid input must parse");
        assert_eq!(doc.statements.len(), 3);
    }
}
