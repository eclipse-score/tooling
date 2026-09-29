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

use std::collections::{HashMap, HashSet};

use sequence_logic::{
    ParticipantType as LogicParticipantType, SequenceParticipant, SourceLocation,
};
use sequence_parser::sequence_ast::{
    CreateCmd, MessageEndpoint, ParticipantDef, ParticipantIdentifier, ParticipantRef,
    ParticipantType as SyntaxParticipantType, Statement,
};
use uid_normalization::{InternalScope, RootAnchor};
use uid_utils::{normalize, normalized_segments};

use crate::error::SequenceResolverError;

const EXTERNAL_ENDPOINT: &str = "ExternalEndpoint";

#[derive(Debug, Clone, Default)]
pub(crate) struct ParticipantTable {
    participants: Vec<SequenceParticipant>,
    resolved_names: HashMap<String, String>,
}

impl ParticipantTable {
    #[cfg(test)]
    pub(crate) fn participants(&self) -> &[SequenceParticipant] {
        &self.participants
    }

    pub(crate) fn into_participants(self) -> Vec<SequenceParticipant> {
        self.participants
    }

    pub(crate) fn resolve_identifier(
        &self,
        identifier: &ParticipantIdentifier,
        source_location: &SourceLocation,
        root_anchor: &RootAnchor,
    ) -> Result<String, SequenceResolverError> {
        self.resolve_reference_name(
            participant_reference_name(identifier),
            source_location,
            root_anchor,
        )
    }

    pub(crate) fn resolve_participant_ref(
        &self,
        participant: &ParticipantRef,
        source_location: &SourceLocation,
        root_anchor: &RootAnchor,
    ) -> Result<String, SequenceResolverError> {
        self.resolve_reference_name(
            participant.identifier.as_str(),
            source_location,
            root_anchor,
        )
    }

    fn resolve_reference_name(
        &self,
        reference_name: &str,
        source_location: &SourceLocation,
        root_anchor: &RootAnchor,
    ) -> Result<String, SequenceResolverError> {
        let normalized_reference = normalize(reference_name);

        if let Some(resolved) = self.resolved_names.get(&normalized_reference) {
            return Ok(resolved.clone());
        }

        resolve_selected_participant_uid(reference_name, source_location, root_anchor)
    }
}

pub(crate) fn build_participant_table(
    statements: &[Statement],
    root_anchor: &RootAnchor,
) -> Result<ParticipantTable, SequenceResolverError> {
    let mut resolved_names = HashSet::new();
    let mut resolved_uids = HashSet::new();
    let mut table = ParticipantTable::default();

    add_explicit_participants(
        statements,
        &mut table,
        &mut resolved_names,
        &mut resolved_uids,
        root_anchor,
    )?;

    // Message endpoints and create commands must share the ordered pass so the
    // first participant reference keeps its source location.
    add_implicit_participants(
        statements,
        &mut table,
        &mut resolved_names,
        &mut resolved_uids,
        root_anchor,
    )?;

    Ok(table)
}

fn add_explicit_participants(
    statements: &[Statement],
    table: &mut ParticipantTable,
    resolved_names: &mut HashSet<String>,
    resolved_uids: &mut HashSet<String>,
    root_anchor: &RootAnchor,
) -> Result<(), SequenceResolverError> {
    for stmt in statements {
        if let Statement::ParticipantDef(participant_def) = stmt {
            add_participant(
                table,
                resolved_names,
                resolved_uids,
                explicit_participant(participant_def),
                root_anchor,
            )?;
        }
    }

    Ok(())
}

fn add_implicit_participants(
    statements: &[Statement],
    table: &mut ParticipantTable,
    resolved_names: &mut HashSet<String>,
    resolved_uids: &mut HashSet<String>,
    root_anchor: &RootAnchor,
) -> Result<(), SequenceResolverError> {
    for stmt in statements {
        match stmt {
            Statement::Message(msg) => {
                add_endpoint_participant(
                    table,
                    resolved_names,
                    resolved_uids,
                    &msg.left,
                    &msg.source_location,
                    root_anchor,
                )?;
                add_endpoint_participant(
                    table,
                    resolved_names,
                    resolved_uids,
                    &msg.right,
                    &msg.source_location,
                    root_anchor,
                )?;
            }
            Statement::CreateCmd(create_cmd) => {
                add_participant(
                    table,
                    resolved_names,
                    resolved_uids,
                    created_participant(create_cmd),
                    root_anchor,
                )?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn add_participant(
    table: &mut ParticipantTable,
    resolved_names: &mut HashSet<String>,
    resolved_uids: &mut HashSet<String>,
    mut participant: SequenceParticipant,
    root_anchor: &RootAnchor,
) -> Result<(), SequenceResolverError> {
    let identifier = ParticipantIdentifier {
        display_name: participant.display_name.clone(),
        alias: participant.alias.clone(),
    };
    let reference_name = normalized_participant_reference_name(&identifier);

    if reference_name.is_empty() {
        return Ok(());
    }

    if !resolved_names.insert(reference_name.clone()) {
        return Ok(());
    }

    let resolved_uid =
        resolve_participant_identifier_uid(&identifier, &participant.source_location, root_anchor)?;

    if !resolved_uids.insert(resolved_uid.clone()) {
        return Err(SequenceResolverError::DuplicateParticipantId {
            participant_id: resolved_uid,
            source_location: participant.source_location.clone(),
        });
    }

    participant.uid = resolved_uid.clone();
    table.resolved_names.insert(reference_name, resolved_uid);
    table.participants.push(participant);

    Ok(())
}

fn add_endpoint_participant(
    table: &mut ParticipantTable,
    resolved_names: &mut HashSet<String>,
    resolved_uids: &mut HashSet<String>,
    endpoint: &MessageEndpoint,
    source_location: &SourceLocation,
    root_anchor: &RootAnchor,
) -> Result<(), SequenceResolverError> {
    if let MessageEndpoint::Participant(identifier) = endpoint {
        add_participant(
            table,
            resolved_names,
            resolved_uids,
            implicit_participant(identifier, source_location),
            root_anchor,
        )?;
    }

    Ok(())
}

fn explicit_participant(participant_def: &ParticipantDef) -> SequenceParticipant {
    SequenceParticipant {
        display_name: participant_def.identifier.display_name.clone(),
        alias: participant_def.identifier.alias.clone(),
        uid: String::new(),
        participant_type: map_parser_participant_type(&participant_def.participant_type),
        source_location: participant_def.source_location.clone(),
        stereotype: participant_def.stereotype.clone(),
    }
}

fn created_participant(create_cmd: &CreateCmd) -> SequenceParticipant {
    SequenceParticipant {
        display_name: create_cmd.identifier.display_name.clone(),
        alias: create_cmd.identifier.alias.clone(),
        uid: String::new(),
        participant_type: map_parser_participant_type(&create_cmd.participant_type),
        source_location: create_cmd.source_location.clone(),
        stereotype: create_cmd.stereotype.clone(),
    }
}

fn implicit_participant(
    identifier: &ParticipantIdentifier,
    source_location: &SourceLocation,
) -> SequenceParticipant {
    SequenceParticipant {
        display_name: identifier.display_name.clone(),
        alias: identifier.alias.clone(),
        uid: String::new(),
        participant_type: LogicParticipantType::Participant,
        source_location: source_location.clone(),
        stereotype: None,
    }
}

fn map_parser_participant_type(kind: &SyntaxParticipantType) -> LogicParticipantType {
    match kind {
        SyntaxParticipantType::Participant => LogicParticipantType::Participant,
        SyntaxParticipantType::Actor => LogicParticipantType::Actor,
        SyntaxParticipantType::Boundary => LogicParticipantType::Boundary,
        SyntaxParticipantType::Control => LogicParticipantType::Control,
        SyntaxParticipantType::Entity => LogicParticipantType::Entity,
        SyntaxParticipantType::Queue => LogicParticipantType::Queue,
        SyntaxParticipantType::Database => LogicParticipantType::Database,
        SyntaxParticipantType::Collections => LogicParticipantType::Collections,
    }
}

pub(crate) fn participant_reference_name(identifier: &ParticipantIdentifier) -> &str {
    identifier
        .alias
        .as_deref()
        .unwrap_or(&identifier.display_name)
}

pub(crate) fn normalized_participant_reference_name(identifier: &ParticipantIdentifier) -> String {
    normalize(participant_reference_name(identifier))
}

fn resolve_participant_identifier_uid(
    identifier: &ParticipantIdentifier,
    source_location: &SourceLocation,
    root_anchor: &RootAnchor,
) -> Result<String, SequenceResolverError> {
    let selected_text = select_participant_uid_text(identifier, source_location)?;
    resolve_selected_participant_uid(&selected_text, source_location, root_anchor)
}

fn resolve_selected_participant_uid(
    selected_text: &str,
    source_location: &SourceLocation,
    root_anchor: &RootAnchor,
) -> Result<String, SequenceResolverError> {
    let normalized = normalize(selected_text);

    if normalized == EXTERNAL_ENDPOINT {
        return Ok(EXTERNAL_ENDPOINT.to_string());
    }

    let parts = normalized_segments(&normalized);

    let Some((leaf, internal_scope)) = parts.split_last() else {
        return Err(SequenceResolverError::InvalidParticipantIdentifier {
            participant: selected_text.to_string(),
            reason: "resolved uid is empty".to_string(),
            source_location: source_location.clone(),
        });
    };

    Ok(
        InternalScope::new(internal_scope.iter().map(String::as_str))
            .resolve_with_leaf(root_anchor, leaf),
    )
}

fn select_participant_uid_text(
    identifier: &ParticipantIdentifier,
    source_location: &SourceLocation,
) -> Result<String, SequenceResolverError> {
    let first_line = identifier
        .display_name
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();

    let standalone_colons = standalone_colon_positions(first_line);

    if standalone_colons.len() > 1 {
        return Err(SequenceResolverError::InvalidParticipantIdentifier {
            participant: identifier.display_name.clone(),
            reason: "multiple standalone ':' separators are not allowed".to_string(),
            source_location: source_location.clone(),
        });
    }

    if let Some(colon_index) = standalone_colons.first().copied() {
        let rhs = first_line[colon_index + 1..].trim();
        if rhs.is_empty() {
            return Err(SequenceResolverError::InvalidParticipantIdentifier {
                participant: identifier.display_name.clone(),
                reason: "standalone ':' must have a non-empty right-hand side".to_string(),
                source_location: source_location.clone(),
            });
        }
        return Ok(rhs.to_string());
    }

    if is_bare_participant_identifier(first_line) {
        return Ok(first_line.to_string());
    }

    if let Some(alias) = identifier.alias.as_deref() {
        return Ok(alias.to_string());
    }

    Err(SequenceResolverError::InvalidParticipantIdentifier {
        participant: identifier.display_name.clone(),
        reason: "free-text participant display names require an alias for uid derivation"
            .to_string(),
        source_location: source_location.clone(),
    })
}

fn standalone_colon_positions(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut positions = Vec::new();

    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b':' {
            continue;
        }

        let prev_is_colon = index > 0 && bytes[index - 1] == b':';
        let next_is_colon = index + 1 < bytes.len() && bytes[index + 1] == b':';

        if !prev_is_colon && !next_is_colon {
            positions.push(index);
        }
    }

    positions
}

fn is_bare_participant_identifier(text: &str) -> bool {
    if text.is_empty() || text.chars().any(char::is_whitespace) {
        return false;
    }

    let segments = normalized_segments(text);
    !segments.is_empty()
        && segments
            .iter()
            .all(|segment| segment.chars().all(is_identifier_char))
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod participant_table_tests {
    use super::*;
    use parser_core::common_ast::{Arrow, ArrowDecor, ArrowLine};
    use sequence_parser::sequence_ast::{DestroyCmd, Message, ParticipantRef};

    fn source(line: u32) -> SourceLocation {
        SourceLocation::new("test.puml", line)
    }

    fn root_anchor() -> RootAnchor {
        RootAnchor::new(None)
    }

    fn message_endpoint(name: &str) -> MessageEndpoint {
        MessageEndpoint::Participant(ParticipantIdentifier {
            display_name: name.to_string(),
            alias: None,
        })
    }

    fn message(from: &str, to: &str, source_location: SourceLocation) -> Statement {
        Statement::Message(Message {
            left: message_endpoint(from),
            arrow: Arrow {
                left: None,
                line: ArrowLine {
                    raw: "-".to_string(),
                },
                middle: None,
                right: Some(ArrowDecor {
                    raw: ">".to_string(),
                }),
            },
            right: message_endpoint(to),
            suffix: None,
            description: Some("message".to_string()),
            source_location,
        })
    }

    fn participant(name: &str) -> Statement {
        Statement::ParticipantDef(ParticipantDef {
            participant_type: SyntaxParticipantType::Participant,
            identifier: ParticipantIdentifier {
                display_name: name.to_string(),
                alias: None,
            },
            stereotype: None,
            source_location: source(0),
        })
    }

    fn participant_with_alias(display_name: &str, alias: &str) -> Statement {
        Statement::ParticipantDef(ParticipantDef {
            participant_type: SyntaxParticipantType::Participant,
            identifier: ParticipantIdentifier {
                display_name: display_name.to_string(),
                alias: Some(alias.to_string()),
            },
            stereotype: None,
            source_location: source(0),
        })
    }

    #[test]
    fn declared_participants_are_preserved_without_duplicates() {
        let statements = vec![
            participant("A"),
            participant("B"),
            message("A", "B", source(1)),
            message("B", "A", source(2)),
        ];

        let participants = build_participant_table(&statements, &root_anchor())
            .expect("participant table must build");

        assert_eq!(participants.participants().len(), 2);
        assert_eq!(participants.participants()[0].display_name, "A");
        assert_eq!(participants.participants()[1].display_name, "B");
    }

    #[test]
    fn aliased_participant_reference_does_not_create_duplicate() {
        let statements = vec![
            participant("A"),
            participant_with_alias("Display B", "B"),
            message("A", "B", source(1)),
        ];

        let participants = build_participant_table(&statements, &root_anchor())
            .expect("participant table must build");

        assert_eq!(participants.participants().len(), 2);
        assert_eq!(participants.participants()[1].display_name, "Display B");
        assert_eq!(participants.participants()[1].alias.as_deref(), Some("B"));
    }

    #[test]
    fn aliased_participant_display_name_reference_is_rejected() {
        let statements = vec![
            participant("A"),
            participant_with_alias("Display B", "B"),
            message("A", "Display B", source(1)),
        ];

        let err = build_participant_table(&statements, &root_anchor())
            .expect_err("free-text display name references without aliases must fail");

        assert!(matches!(
            err,
            SequenceResolverError::InvalidParticipantIdentifier { .. }
        ));
    }

    #[test]
    fn qualified_aliases_are_deduplicated_after_normalization() {
        let statements = vec![
            participant_with_alias("Service", "score::logging::Service"),
            message("Caller", "score.logging.Service", source(1)),
        ];

        let participants = build_participant_table(&statements, &root_anchor())
            .expect("participant table must build");

        assert_eq!(participants.participants().len(), 2);
        assert_eq!(
            participants.participants()[0].alias.as_deref(),
            Some("score::logging::Service")
        );
        assert_eq!(participants.participants()[1].display_name, "Caller");
    }

    #[test]
    fn duplicate_resolved_participant_ids_are_rejected() {
        let statements = vec![
            participant_with_alias("svc:billing::InvoiceService", "inv1"),
            participant_with_alias("billing::InvoiceService", "inv2"),
        ];

        let err = build_participant_table(&statements, &root_anchor())
            .expect_err("duplicate resolved participant ids must fail");

        assert!(matches!(
            err,
            SequenceResolverError::DuplicateParticipantId { ref participant_id, .. }
                if participant_id == "billing.InvoiceService"
        ));
    }

    #[test]
    fn no_participants_declared_creates_implicit_participants() {
        let statements = vec![message("X", "Y", source(1))];

        let participants = build_participant_table(&statements, &root_anchor())
            .expect("participant table must build");

        assert_eq!(participants.participants().len(), 2);
        assert_eq!(participants.participants()[0].display_name, "X");
        assert_eq!(participants.participants()[1].display_name, "Y");
    }

    #[test]
    fn message_endpoint_before_create_sets_participant_source_location() {
        let message_location = source(1);
        let create_location = source(2);
        let statements = vec![
            message("A", "B", message_location.clone()),
            Statement::CreateCmd(CreateCmd {
                participant_type: SyntaxParticipantType::Participant,
                identifier: ParticipantIdentifier {
                    display_name: "B".to_string(),
                    alias: None,
                },
                stereotype: None,
                source_location: create_location,
            }),
        ];

        let participants = build_participant_table(&statements, &root_anchor())
            .expect("participant table must build");

        assert_eq!(participants.participants().len(), 2);
        assert_eq!(participants.participants()[0].display_name, "A");
        assert_eq!(
            participants.participants()[0].source_location,
            message_location
        );
        assert_eq!(participants.participants()[1].display_name, "B");
        assert_eq!(
            participants.participants()[1].source_location,
            message_location
        );
    }

    #[test]
    fn destroy_statement_does_not_create_implicit_participant() {
        let statements = vec![Statement::DestroyCmd(DestroyCmd {
            participant: ParticipantRef {
                identifier: "Implicit".to_string(),
            },
            source_location: source(1),
        })];

        let participants = build_participant_table(&statements, &root_anchor())
            .expect("participant table must build");

        assert!(participants.participants().is_empty());
    }

    #[test]
    fn qualified_display_name_resolves_to_type_text_uid() {
        let identifier = ParticipantIdentifier {
            display_name: "core::service::ResourceBuilder".to_string(),
            alias: Some("Builder".to_string()),
        };

        let uid = resolve_participant_identifier_uid(&identifier, &source(1), &root_anchor())
            .expect("uid must resolve");

        assert_eq!(uid, "core.service.ResourceBuilder");
    }

    #[test]
    fn single_standalone_colon_uses_rhs_as_type_text() {
        let identifier = ParticipantIdentifier {
            display_name: "svc : core::runtime::Service".to_string(),
            alias: Some("Service".to_string()),
        };

        let uid = resolve_participant_identifier_uid(&identifier, &source(1), &root_anchor())
            .expect("uid must resolve");

        assert_eq!(uid, "core.runtime.Service");
    }

    #[test]
    fn multiple_standalone_colons_are_rejected() {
        let identifier = ParticipantIdentifier {
            display_name: "svc : core::runtime : Service".to_string(),
            alias: Some("Service".to_string()),
        };

        let err = resolve_participant_identifier_uid(&identifier, &source(1), &root_anchor())
            .expect_err("multiple standalone colons must fail");

        assert!(matches!(
            err,
            SequenceResolverError::InvalidParticipantIdentifier { .. }
        ));
    }

    #[test]
    fn free_text_display_name_without_alias_is_rejected() {
        let identifier = ParticipantIdentifier {
            display_name: "Display Service".to_string(),
            alias: None,
        };

        let err = resolve_participant_identifier_uid(&identifier, &source(1), &root_anchor())
            .expect_err("free-text display names without alias must fail");

        assert!(matches!(
            err,
            SequenceResolverError::InvalidParticipantIdentifier { .. }
        ));
    }

    #[test]
    fn bare_identifiers_without_alias_are_accepted() {
        let simple = ParticipantIdentifier {
            display_name: "A".to_string(),
            alias: None,
        };
        let qualified = ParticipantIdentifier {
            display_name: "score::logging::Service".to_string(),
            alias: None,
        };

        let simple_uid = resolve_participant_identifier_uid(&simple, &source(1), &root_anchor())
            .expect("simple bare identifiers must resolve");
        let qualified_uid =
            resolve_participant_identifier_uid(&qualified, &source(2), &root_anchor())
                .expect("qualified bare identifiers must resolve");

        assert_eq!(simple_uid, "A");
        assert_eq!(qualified_uid, "score.logging.Service");
    }
}
