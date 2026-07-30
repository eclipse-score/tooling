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
use log::{debug, trace, warn};
use parser_core::common_parser::parse_arrow as common_parse_arrow;
use parser_core::common_parser::{PlantUmlCommonParser, Rule};
use parser_core::{
    format_parse_tree, pest_to_syntax_error, BaseParseError, DiagramParser, ErrorLocation,
};
use puml_utils::LogLevel;
use source_location::SourceLocation;
use std::path::PathBuf;
use std::rc::Rc;
use thiserror::Error;

use crate::sequence_ast::*;

#[derive(Debug, Error)]
pub enum SequenceError {
    #[error(transparent)]
    Base(#[from] BaseParseError<Rule>),
    #[error("invalid sequence statement: {0}")]
    InvalidStatement(String),
}

impl ErrorLocation for SequenceError {
    fn error_location(&self) -> Option<(usize, usize)> {
        match self {
            Self::Base(b) => b.error_location(),
            _ => None,
        }
    }
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

    fn parse_statement(
        pair: pest::iterators::Pair<Rule>,
        source_path: &str,
    ) -> Result<Vec<Statement>, SequenceError> {
        let source_location = SourceLocation::new(source_path, pair.line_col().0 as u32);
        let inner = pair
            .into_inner()
            .next()
            .ok_or_else(|| SequenceError::InvalidStatement("empty statement".to_string()))?;

        match inner.as_rule() {
            Rule::participant_def => Ok(vec![Statement::ParticipantDef(
                Self::parse_participant_def(inner, source_location)?,
            )]),
            Rule::lifecycle_cmd => Self::parse_lifecycle_cmd(inner, source_location),
            Rule::message => Ok(vec![Statement::Message(Self::parse_message(
                inner,
                source_location,
            )?)]),
            Rule::group_cmd => Ok(vec![Statement::GroupCmd(Self::parse_group_cmd(
                inner,
                source_location,
            )?)]),
            Rule::ref_stmt => Ok(vec![Statement::RefCmd(Self::parse_ref_cmd(
                inner,
                source_location,
            ))]),
            Rule::return_cmd => Ok(vec![Statement::ReturnCmd(Self::parse_return_cmd(
                inner,
                source_location,
            ))]),
            // Grammar-valid directives that are intentionally not modeled as statements
            _ => Ok(vec![]),
        }
    }

    fn parse_lifecycle_cmd(
        pair: pest::iterators::Pair<Rule>,
        source_location: SourceLocation,
    ) -> Result<Vec<Statement>, SequenceError> {
        let inner = pair.into_inner().next().ok_or_else(|| {
            SequenceError::InvalidStatement("empty lifecycle command".to_string())
        })?;

        match inner.as_rule() {
            Rule::create_cmd => Ok(vec![Statement::CreateCmd(Self::parse_create_cmd(
                inner,
                source_location,
            )?)]),
            Rule::destroy_cmd => Ok(vec![Statement::DestroyCmd(Self::parse_destroy_cmd(inner)?)]),
            Rule::activate_cmd => Ok(vec![Statement::ActivateCmd(Self::parse_activate_cmd(
                inner,
            )?)]),
            Rule::deactivate_cmd => Ok(vec![Statement::DeactivateCmd(Self::parse_deactivate_cmd(
                inner,
            )?)]),
            Rule::activation_short => Self::parse_activation_short(inner),
            _ => Err(SequenceError::InvalidStatement(format!(
                "unsupported lifecycle command: {:?}",
                inner.as_rule()
            ))),
        }
    }

    fn parse_participant_def(
        pair: pest::iterators::Pair<Rule>,
        source_location: SourceLocation,
    ) -> Result<ParticipantDef, SequenceError> {
        let mut participant_type: Option<ParticipantType> = None;
        let mut identifier: Option<ParticipantIdentifier> = None;
        let mut stereotype: Option<String> = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::participant_type => {
                    participant_type = Some(Self::parse_participant_type(inner)?);
                }
                Rule::participant_identifier => {
                    identifier = Some(Self::parse_participant_identifier(inner)?);
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
            participant_type: participant_type.ok_or_else(|| {
                SequenceError::InvalidStatement("missing participant type".to_string())
            })?,
            identifier: identifier.ok_or_else(|| {
                SequenceError::InvalidStatement("missing participant identifier".to_string())
            })?,
            stereotype,
            source_location,
        })
    }

    fn parse_participant_identifier(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ParticipantIdentifier, SequenceError> {
        let participant = pair.into_inner().next().ok_or_else(|| {
            SequenceError::InvalidStatement(
                "participant_identifier must contain a participant identifier".to_string(),
            )
        })?;
        let participant_rule = participant.as_rule();

        Ok(match participant_rule {
            Rule::quoted_display_with_alias => {
                match Self::participant_parts(participant).as_slice() {
                    [display_name, alias] => ParticipantIdentifier {
                        display_name: Self::extract_quoted_string(display_name),
                        alias: Some(alias.to_string()),
                    },
                    _ => {
                        return Self::invalid_participant_identifier(
                            "quoted_display_with_alias grammar shape changed",
                        );
                    }
                }
            }
            Rule::display_with_alias => match Self::participant_parts(participant).as_slice() {
                [display_name, alias] => ParticipantIdentifier {
                    display_name: display_name.to_string(),
                    alias: Some(alias.to_string()),
                },
                _ => {
                    return Self::invalid_participant_identifier(
                        "display_with_alias grammar shape changed",
                    );
                }
            },
            Rule::alias_with_quoted_display => {
                match Self::participant_parts(participant).as_slice() {
                    [alias, display_name] => ParticipantIdentifier {
                        display_name: Self::extract_quoted_string(display_name),
                        alias: Some(alias.to_string()),
                    },
                    _ => {
                        return Self::invalid_participant_identifier(
                            "alias_with_quoted_display grammar shape changed",
                        );
                    }
                }
            }
            Rule::quoted_display => ParticipantIdentifier {
                display_name: Self::extract_quoted_string(participant.as_str()),
                alias: None,
            },
            Rule::alias_only => ParticipantIdentifier {
                display_name: participant.as_str().trim().to_string(),
                alias: None,
            },
            _ => {
                warn!(
                    "participant_identifier grammar produced unsupported value: {:?}",
                    participant_rule
                );
                return Err(SequenceError::InvalidStatement(format!(
                    "unsupported participant_identifier grammar value: {:?}",
                    participant_rule
                )));
            }
        })
    }

    fn invalid_participant_identifier(
        reason: &str,
    ) -> Result<ParticipantIdentifier, SequenceError> {
        warn!("{reason}");
        Err(SequenceError::InvalidStatement(reason.to_string()))
    }

    fn participant_parts(participant: pest::iterators::Pair<Rule>) -> Vec<String> {
        participant
            .into_inner()
            .map(|part| part.as_str().trim().to_string())
            .collect()
    }

    fn parse_participant_type(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<ParticipantType, SequenceError> {
        let text = pair.as_str().to_lowercase();
        Ok(match text.as_str() {
            "participant" => ParticipantType::Participant,
            "actor" => ParticipantType::Actor,
            "boundary" => ParticipantType::Boundary,
            "control" => ParticipantType::Control,
            "entity" => ParticipantType::Entity,
            "queue" => ParticipantType::Queue,
            "database" => ParticipantType::Database,
            "collections" => ParticipantType::Collections,
            _ => {
                warn!("participant_type grammar produced unsupported value: {text}");
                return Err(SequenceError::InvalidStatement(format!(
                    "unsupported participant type: {text}"
                )));
            }
        })
    }

    fn parse_message(
        pair: pest::iterators::Pair<Rule>,
        source_location: SourceLocation,
    ) -> Result<Message, SequenceError> {
        let mut body: Option<pest::iterators::Pair<Rule>> = None;
        let mut suffix: Option<MessageSuffix> = None;
        let mut description: Option<String> = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::message_body => body = Some(inner),
                Rule::message_suffix => suffix = Some(Self::parse_message_suffix(inner)?),
                Rule::sequence_description => {
                    description = inner
                        .into_inner()
                        .next()
                        .map(|p| p.as_str().trim().to_string());
                }
                _ => {}
            }
        }

        let (left, arrow, right) =
            Self::parse_message_body(body.ok_or_else(|| {
                SequenceError::InvalidStatement("missing message body".to_string())
            })?)?;

        Ok(Message {
            left,
            arrow,
            right,
            suffix,
            description,
            source_location,
        })
    }

    fn parse_message_body(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<(MessageEndpoint, Arrow, MessageEndpoint), SequenceError> {
        let body = pair
            .into_inner()
            .next()
            .ok_or_else(|| SequenceError::InvalidStatement("empty message body".to_string()))?;
        let body_rule = body.as_rule();
        let mut endpoints = Vec::new();
        let mut arrow = None;

        for inner in body.into_inner() {
            match inner.as_rule() {
                Rule::message_endpoint => endpoints.push(Self::parse_message_endpoint(inner)?),
                Rule::sequence_arrow => arrow = Some(Self::parse_arrow(inner)?),
                _ => {}
            }
        }

        let arrow = arrow.ok_or_else(|| {
            SequenceError::InvalidStatement("message body must contain arrow".to_string())
        })?;
        let mut endpoints = endpoints.into_iter();

        match body_rule {
            Rule::message_full => Ok((
                endpoints.next().ok_or_else(|| {
                    SequenceError::InvalidStatement(
                        "message_full must contain left endpoint".to_string(),
                    )
                })?,
                arrow,
                endpoints.next().ok_or_else(|| {
                    SequenceError::InvalidStatement(
                        "message_full must contain right endpoint".to_string(),
                    )
                })?,
            )),
            Rule::message_missing_left => Ok((
                Self::missing_message_endpoint(),
                arrow,
                endpoints.next().ok_or_else(|| {
                    SequenceError::InvalidStatement(
                        "message_missing_left must contain right endpoint".to_string(),
                    )
                })?,
            )),
            Rule::message_missing_right => Ok((
                endpoints.next().ok_or_else(|| {
                    SequenceError::InvalidStatement(
                        "message_missing_right must contain left endpoint".to_string(),
                    )
                })?,
                arrow,
                Self::missing_message_endpoint(),
            )),
            _ => Err(SequenceError::InvalidStatement(format!(
                "unsupported message body: {:?}",
                body_rule
            ))),
        }
    }

    fn missing_message_endpoint() -> MessageEndpoint {
        MessageEndpoint::LostFound("?".to_string())
    }

    fn parse_message_endpoint(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MessageEndpoint, SequenceError> {
        let endpoint = pair.into_inner().next().ok_or_else(|| {
            SequenceError::InvalidStatement("message_endpoint must contain an endpoint".to_string())
        })?;

        Ok(match endpoint.as_rule() {
            Rule::inline_participant => {
                MessageEndpoint::Participant(Self::parse_participant_identifier(endpoint)?)
            }
            Rule::lost_found_marker => {
                MessageEndpoint::LostFound(Self::parse_lost_found_marker(endpoint)?)
            }
            _ => {
                return Err(SequenceError::InvalidStatement(format!(
                    "message_endpoint grammar produced unsupported value: {:?}",
                    endpoint.as_rule()
                )));
            }
        })
    }

    fn parse_lost_found_marker(pair: pest::iterators::Pair<Rule>) -> Result<String, SequenceError> {
        let marker = pair.into_inner().next().ok_or_else(|| {
            SequenceError::InvalidStatement(
                "lost_found_marker must contain a lost-found endpoint".to_string(),
            )
        })?;

        match marker.as_rule() {
            Rule::left_lost_found | Rule::right_lost_found | Rule::short_lost_found => {
                Ok(marker.as_str().trim().to_string())
            }
            _ => Err(SequenceError::InvalidStatement(format!(
                "lost_found_marker grammar produced unsupported value: {:?}",
                marker.as_rule()
            ))),
        }
    }

    fn parse_message_suffix(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MessageSuffix, SequenceError> {
        let suffixes: Vec<_> = pair
            .into_inner()
            .map(Self::parse_message_suffix_part)
            .collect::<Result<_, _>>()?;

        match suffixes.as_slice() {
            [suffix] => Ok(suffix.clone()),
            _ => Ok(MessageSuffix::Combined(suffixes)),
        }
    }

    fn parse_message_suffix_part(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MessageSuffix, SequenceError> {
        Ok(match pair.as_rule() {
            Rule::activate_suffix => MessageSuffix::Activate,
            Rule::deactivate_suffix => MessageSuffix::Deactivate,
            Rule::create_suffix => MessageSuffix::Create,
            Rule::destroy_suffix => MessageSuffix::Destroy,
            _ => {
                return Err(SequenceError::InvalidStatement(format!(
                    "message_suffix grammar produced unsupported value: {:?}",
                    pair.as_rule()
                )));
            }
        })
    }

    fn parse_arrow(pair: pest::iterators::Pair<Rule>) -> Result<Arrow, SequenceError> {
        common_parse_arrow(pair)
            .map_err(|e| SequenceError::InvalidStatement(format!("invalid arrow: {}", e)))
    }

    fn parse_group_cmd(
        pair: pest::iterators::Pair<Rule>,
        source_location: SourceLocation,
    ) -> Result<GroupCmd, SequenceError> {
        let inner = pair
            .into_inner()
            .next()
            .ok_or_else(|| SequenceError::InvalidStatement("empty group command".to_string()))?;

        match inner.as_rule() {
            Rule::group_start => Self::parse_group_start(inner, source_location),
            Rule::group_branch => Self::parse_group_branch(inner, source_location),
            Rule::group_end => Self::parse_group_end(inner, source_location),
            _ => Err(SequenceError::InvalidStatement(format!(
                "unsupported group command: {:?}",
                inner.as_rule()
            ))),
        }
    }

    fn parse_group_start(
        pair: pest::iterators::Pair<Rule>,
        source_location: SourceLocation,
    ) -> Result<GroupCmd, SequenceError> {
        let mut kind: Option<GroupKind> = None;
        let mut label: Option<String> = None;
        let mut is_parallel = false;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::parallel_marker => {
                    is_parallel = true;
                }
                Rule::group_start_type => {
                    kind = Some(Self::parse_group_kind(inner)?);
                }
                Rule::group_label => {
                    label = Some(inner.as_str().trim().to_string());
                }
                _ => {}
            }
        }

        Ok(GroupCmd::Start(GroupStart {
            kind: kind.ok_or_else(|| {
                SequenceError::InvalidStatement("missing group start kind".to_string())
            })?,
            label,
            is_parallel,
            source_location,
        }))
    }

    fn parse_group_branch(
        pair: pest::iterators::Pair<Rule>,
        source_location: SourceLocation,
    ) -> Result<GroupCmd, SequenceError> {
        let mut has_else = false;
        let mut label: Option<String> = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::group_branch_type => {
                    has_else = true;
                }
                Rule::group_label => {
                    label = Some(inner.as_str().trim().to_string());
                }
                _ => {}
            }
        }

        if !has_else {
            return Err(SequenceError::InvalidStatement(
                "missing group branch kind".to_string(),
            ));
        }

        Ok(GroupCmd::Else(GroupElse {
            label,
            source_location,
        }))
    }

    fn parse_group_end(
        pair: pest::iterators::Pair<Rule>,
        source_location: SourceLocation,
    ) -> Result<GroupCmd, SequenceError> {
        let mut kind: Option<GroupKind> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::group_start_type {
                kind = Some(Self::parse_group_kind(inner)?);
            }
        }

        Ok(GroupCmd::End(GroupEnd {
            kind,
            source_location,
        }))
    }

    fn parse_group_kind(pair: pest::iterators::Pair<Rule>) -> Result<GroupKind, SequenceError> {
        let text = pair.as_str().to_lowercase();
        Ok(match text.as_str() {
            "alt" => GroupKind::Alt,
            "opt" => GroupKind::Opt,
            "loop" => GroupKind::Loop,
            "par" => GroupKind::Par,
            "break" => GroupKind::Break,
            "critical" => GroupKind::Critical,
            "group" => GroupKind::Group,
            _ => {
                return Err(SequenceError::InvalidStatement(format!(
                    "unsupported group kind: {text}"
                )));
            }
        })
    }

    fn parse_ref_cmd(pair: pest::iterators::Pair<Rule>, source_location: SourceLocation) -> RefCmd {
        let mut participants = Vec::new();
        let mut text = None;

        Self::extract_ref_parts(pair, &mut participants, &mut text);

        RefCmd {
            participants,
            text,
            source_location,
        }
    }

    fn parse_return_cmd(
        pair: pest::iterators::Pair<Rule>,
        source_location: SourceLocation,
    ) -> ReturnCmd {
        let label = pair
            .into_inner()
            .find(|inner| inner.as_rule() == Rule::sequence_text_content)
            .map(|inner| inner.as_str().trim().to_string())
            .filter(|text| !text.is_empty());

        ReturnCmd {
            label,
            source_location,
        }
    }

    fn extract_ref_parts(
        pair: pest::iterators::Pair<Rule>,
        participants: &mut Vec<ParticipantRef>,
        text: &mut Option<String>,
    ) {
        match pair.as_rule() {
            Rule::participant_list => {
                participants.extend(pair.into_inner().filter_map(|inner| {
                    if inner.as_rule() == Rule::participant_ref {
                        Some(Self::parse_participant_ref(inner))
                    } else {
                        None
                    }
                }));
            }
            Rule::sequence_text_content | Rule::ref_body => {
                let value = pair.as_str().trim();
                if !value.is_empty() {
                    *text = Some(value.to_string());
                }
            }
            _ => {
                for inner in pair.into_inner() {
                    Self::extract_ref_parts(inner, participants, text);
                }
            }
        }
    }

    fn parse_create_cmd(
        pair: pest::iterators::Pair<Rule>,
        source_location: SourceLocation,
    ) -> Result<CreateCmd, SequenceError> {
        let mut participant_type: Option<ParticipantType> = None;
        let mut identifier: Option<ParticipantIdentifier> = None;
        let mut stereotype: Option<String> = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::participant_type => {
                    participant_type = Some(Self::parse_participant_type(inner)?);
                }
                Rule::participant_identifier => {
                    identifier = Some(Self::parse_participant_identifier(inner)?);
                }
                Rule::stereotype => {
                    stereotype = Some(Self::extract_stereotype(inner.as_str()));
                }
                Rule::order_clause => {}
                _ => {}
            }
        }

        Ok(CreateCmd {
            participant_type: participant_type.unwrap_or(ParticipantType::Participant),
            identifier: identifier.ok_or_else(|| {
                SequenceError::InvalidStatement("missing participant identifier".to_string())
            })?,
            stereotype,
            source_location,
        })
    }

    fn parse_destroy_cmd(pair: pest::iterators::Pair<Rule>) -> Result<DestroyCmd, SequenceError> {
        let mut participant: Option<ParticipantRef> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::parse_participant_ref(inner));
            }
        }

        Ok(DestroyCmd {
            participant: participant.ok_or_else(|| {
                SequenceError::InvalidStatement("missing participant in destroy".to_string())
            })?,
        })
    }

    fn parse_activate_cmd(pair: pest::iterators::Pair<Rule>) -> Result<ActivateCmd, SequenceError> {
        let mut participant: Option<ParticipantRef> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::parse_participant_ref(inner));
            }
        }

        Ok(ActivateCmd {
            participant: participant.ok_or_else(|| {
                SequenceError::InvalidStatement("missing participant in activate".to_string())
            })?,
        })
    }

    fn parse_deactivate_cmd(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<DeactivateCmd, SequenceError> {
        let mut participant: Option<ParticipantRef> = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::participant_ref {
                participant = Some(Self::parse_participant_ref(inner));
            }
        }

        Ok(DeactivateCmd {
            participant: participant.ok_or_else(|| {
                SequenceError::InvalidStatement("missing participant in deactivate".to_string())
            })?,
        })
    }

    fn parse_activation_short(
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<Statement>, SequenceError> {
        let mut parts = pair.into_inner();
        let participant = parts
            .next()
            .filter(|part| part.as_rule() == Rule::participant_ref)
            .map(Self::parse_participant_ref)
            .ok_or_else(|| {
                SequenceError::InvalidStatement(
                    "missing participant in short activation".to_string(),
                )
            })?;

        match parts.next().map(|part| part.as_rule()).ok_or_else(|| {
            SequenceError::InvalidStatement("missing short activation suffix".to_string())
        })? {
            Rule::activate_suffix => Ok(vec![Statement::ActivateCmd(ActivateCmd { participant })]),
            Rule::deactivate_suffix => Ok(vec![Statement::DeactivateCmd(DeactivateCmd {
                participant,
            })]),
            other => Err(SequenceError::InvalidStatement(format!(
                "unsupported short activation suffix: {:?}",
                other
            ))),
        }
    }

    fn parse_participant_ref(pair: pest::iterators::Pair<Rule>) -> ParticipantRef {
        ParticipantRef {
            identifier: Self::extract_participant_ref(pair),
        }
    }

    // Helper functions
    fn extract_quoted_string(s: &str) -> String {
        s.trim()
            .trim_start_matches('"')
            .trim_end_matches('"')
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
            Rule::participant_ref => {
                let fallback = pair.as_str().trim().to_string();
                pair.into_inner()
                    .next()
                    .map(Self::extract_participant_ref)
                    .unwrap_or(fallback)
            }
            Rule::CNAME => pair.as_str().trim().to_string(),
            _ => pair.as_str().trim().to_string(),
        }
    }

    #[cfg(not(coverage))]
    fn log_parse_tree_if_enabled(
        pairs: &pest::iterators::Pairs<Rule>,
        path: &Rc<PathBuf>,
        log_level: LogLevel,
    ) {
        if matches!(log_level, LogLevel::Debug | LogLevel::Trace) {
            let mut tree_output = String::new();
            format_parse_tree(pairs.clone(), 0, &mut tree_output);
            debug!(
                "\n=== Parse Tree for {} ===\n{}=== End Parse Tree ===",
                path.display(),
                tree_output
            );
        }
    }

    fn parse_document(
        pairs: pest::iterators::Pairs<Rule>,
        source_path: &str,
    ) -> Result<SeqPumlDocument, SequenceError> {
        let mut document = SeqPumlDocument {
            name: None,
            statements: Vec::new(),
        };

        for inner_pair in pairs
            .filter(|pair| pair.as_rule() == Rule::sequence_start)
            .flat_map(|pair| pair.into_inner())
        {
            match inner_pair.as_rule() {
                Rule::startuml => {
                    document.name = Self::parse_startuml(inner_pair);
                }
                Rule::sequence_statement => {
                    document
                        .statements
                        .extend(Self::parse_statement(inner_pair, source_path)?);
                }
                _ => {}
            }
        }

        Ok(document)
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

        #[cfg(not(coverage))]
        Self::log_parse_tree_if_enabled(&pairs, path, log_level);

        let source_path = path.as_ref().clone().to_string_lossy().to_string();
        Self::parse_document(pairs, &source_path)
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

    #[test]
    fn test_source_locations_are_preserved() {
        let input = "@startuml\nparticipant A\nparticipant B\nA -> B : call\n@enduml";
        let path = Rc::new(PathBuf::from("t.puml"));
        let mut parser = PumlSequenceParser;
        let doc = parser
            .parse_file(&path, input, LogLevel::Info)
            .expect("valid input must parse");

        let expected_file = path.as_ref().clone().to_string_lossy().to_string();

        let first_participant = match &doc.statements[0] {
            Statement::ParticipantDef(participant) => participant,
            actual => panic!(
                "expected first statement to be a participant, got {:?}",
                actual
            ),
        };

        assert_eq!(first_participant.source_location.line, 2);
        assert_eq!(
            first_participant.source_location.file.as_ref(),
            expected_file.as_str()
        );

        let message = match &doc.statements[2] {
            Statement::Message(message) => message,
            actual => panic!("expected third statement to be a message, got {:?}", actual),
        };

        assert_eq!(message.source_location.line, 4);
        assert_eq!(
            message.source_location.file.as_ref(),
            expected_file.as_str()
        );
    }

    #[test]
    fn test_statement_after_multiline_ref_is_preserved() {
        let input = "@startuml\nref over Alice, Bob\n  initialize service\nend ref\nAlice -> Bob : done\n@enduml";
        let mut parser = PumlSequenceParser;
        let doc = parser
            .parse_file(&Rc::new(PathBuf::from("t.puml")), input, LogLevel::Info)
            .expect("statement after multiline ref must parse");

        match &doc.statements[0] {
            Statement::RefCmd(ref_cmd) => {
                assert_eq!(ref_cmd.text.as_deref(), Some("initialize service"));
            }
            actual => panic!("expected ref statement, got {:?}", actual),
        }

        match &doc.statements[1] {
            Statement::Message(message) => {
                assert_eq!(message.description.as_deref(), Some("done"));
                assert_eq!(message.source_location.line, 5);
            }
            actual => panic!("expected message after ref statement, got {:?}", actual),
        }
    }
}
