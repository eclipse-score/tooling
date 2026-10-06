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
    ParticipantId, ParticipantType as LogicParticipantType, SequenceParticipant, SourceLocation,
};
use sequence_parser::sequence_ast::{
    CreateCmd, MessageEndpoint, ParticipantDef, ParticipantIdentifier, ParticipantRef,
    ParticipantType as SyntaxParticipantType, Statement,
};
use uid_normalization::RootAnchor;

use crate::error::SequenceResolverError;
use crate::participant_uid::participant_uid;

const DISPLAY_NAME_REASON: &str =
    "is the display name of an aliased participant, refer to it by its alias";

/// Participants in declaration order plus the uid each reference name stands for.
#[derive(Debug, Default)]
pub(crate) struct ParticipantTable {
    pub(crate) participants: Vec<SequenceParticipant>,
    uid_by_reference: HashMap<String, ParticipantId>,
    uids: HashSet<String>,
}

impl ParticipantTable {
    pub(crate) fn uid_of_identifier(
        &self,
        identifier: &ParticipantIdentifier,
        source_location: &SourceLocation,
    ) -> Result<ParticipantId, SequenceResolverError> {
        self.uid_of_reference(
            reference_name(&identifier.display_name, identifier.alias.as_deref()),
            source_location,
        )
    }

    pub(crate) fn uid_of_ref(
        &self,
        participant: &ParticipantRef,
        source_location: &SourceLocation,
    ) -> Result<ParticipantId, SequenceResolverError> {
        self.uid_of_reference(&participant.identifier, source_location)
    }

    fn uid_of_reference(
        &self,
        reference_name: &str,
        source_location: &SourceLocation,
    ) -> Result<ParticipantId, SequenceResolverError> {
        if let Some(uid) = self.uid_by_reference.get(reference_name) {
            return Ok(uid.clone());
        }

        if self.is_display_name_of_aliased(reference_name) {
            return Err(display_name_error(reference_name, source_location));
        }

        Err(SequenceResolverError::UnknownParticipant {
            reference: reference_name.to_string(),
            source_location: source_location.clone(),
        })
    }

    fn has_reference(&self, reference_name: &str) -> bool {
        self.uid_by_reference.contains_key(reference_name)
    }

    fn is_display_name_of_aliased(&self, name: &str) -> bool {
        self.participants
            .iter()
            .any(|participant| participant.alias.is_some() && participant.display_name == name)
    }
}

pub(crate) fn build_participant_table(
    statements: &[Statement],
    root_anchor: &RootAnchor,
) -> Result<ParticipantTable, SequenceResolverError> {
    let mut table = ParticipantTable::default();

    add_explicit_participants(statements, root_anchor, &mut table)?;

    // Message endpoints and create commands must share the ordered pass so the
    // first participant reference keeps its source location.
    add_implicit_participants(statements, root_anchor, &mut table)?;

    Ok(table)
}

fn add_explicit_participants(
    statements: &[Statement],
    root_anchor: &RootAnchor,
    table: &mut ParticipantTable,
) -> Result<(), SequenceResolverError> {
    for stmt in statements {
        if let Statement::ParticipantDef(participant_def) = stmt {
            add_participant(table, root_anchor, explicit_participant(participant_def))?;
        }
    }

    Ok(())
}

fn add_implicit_participants(
    statements: &[Statement],
    root_anchor: &RootAnchor,
    table: &mut ParticipantTable,
) -> Result<(), SequenceResolverError> {
    for stmt in statements {
        match stmt {
            Statement::Message(msg) => {
                add_endpoint_participant(table, root_anchor, &msg.left, &msg.source_location)?;
                add_endpoint_participant(table, root_anchor, &msg.right, &msg.source_location)?;
            }
            Statement::CreateCmd(create_cmd) => {
                add_referenced_participant(table, root_anchor, created_participant(create_cmd))?;
            }
            // Lifecycle and `ref over` statements do not declare participants.
            _ => {}
        }
    }

    Ok(())
}

/// Adds `participant` unless its reference name is already in the table.
/// The display name of an aliased participant is not a reference name.
fn add_referenced_participant(
    table: &mut ParticipantTable,
    root_anchor: &RootAnchor,
    participant: SequenceParticipant,
) -> Result<(), SequenceResolverError> {
    let reference = reference_name(&participant.display_name, participant.alias.as_deref());

    if table.has_reference(reference) {
        return Ok(());
    }

    if table.is_display_name_of_aliased(reference) {
        return Err(display_name_error(reference, &participant.source_location));
    }

    add_participant(table, root_anchor, participant)
}

/// Adds `participant`; its reference name and uid must both be unused.
fn add_participant(
    table: &mut ParticipantTable,
    root_anchor: &RootAnchor,
    mut participant: SequenceParticipant,
) -> Result<(), SequenceResolverError> {
    let reference =
        reference_name(&participant.display_name, participant.alias.as_deref()).to_string();

    if table.has_reference(&reference) {
        return Err(SequenceResolverError::DuplicateParticipantReference {
            reference,
            source_location: participant.source_location,
        });
    }

    participant.uid = participant_uid(
        &participant.display_name,
        participant.alias.as_deref(),
        root_anchor,
    )
    .map_err(
        |reason| SequenceResolverError::InvalidParticipantIdentifier {
            participant: participant.display_name.clone(),
            reason: reason.to_string(),
            source_location: participant.source_location.clone(),
        },
    )?;

    if !table.uids.insert(participant.uid.clone()) {
        return Err(SequenceResolverError::DuplicateParticipantId {
            participant_id: participant.uid,
            source_location: participant.source_location,
        });
    }

    table
        .uid_by_reference
        .insert(reference, ParticipantId::from(participant.uid.as_str()));
    table.participants.push(participant);

    Ok(())
}

fn add_endpoint_participant(
    table: &mut ParticipantTable,
    root_anchor: &RootAnchor,
    endpoint: &MessageEndpoint,
    source_location: &SourceLocation,
) -> Result<(), SequenceResolverError> {
    if let MessageEndpoint::Participant(identifier) = endpoint {
        add_referenced_participant(
            table,
            root_anchor,
            implicit_participant(identifier, source_location),
        )?;
    }

    Ok(())
}

fn display_name_error(name: &str, source_location: &SourceLocation) -> SequenceResolverError {
    SequenceResolverError::InvalidParticipantIdentifier {
        participant: name.to_string(),
        reason: DISPLAY_NAME_REASON.to_string(),
        source_location: source_location.clone(),
    }
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

fn reference_name<'a>(display_name: &'a str, alias: Option<&'a str>) -> &'a str {
    alias.unwrap_or(display_name)
}

#[cfg(test)]
mod participant_table_tests {
    use super::*;
    use parser_core::common_ast::{Arrow, ArrowDecor, ArrowLine};
    use sequence_parser::sequence_ast::{DestroyCmd, Message, ParticipantRef};

    fn build(statements: &[Statement]) -> ParticipantTable {
        build_participant_table(statements, &RootAnchor::default()).unwrap()
    }

    fn build_err(statements: &[Statement]) -> SequenceResolverError {
        build_participant_table(statements, &RootAnchor::default()).unwrap_err()
    }

    fn source(line: u32) -> SourceLocation {
        SourceLocation::new("test.puml", line)
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

        let participants = build(&statements).participants;

        assert_eq!(participants.len(), 2);
        assert_eq!(participants[0].display_name, "A");
        assert_eq!(participants[0].uid, "A");
        assert_eq!(participants[1].display_name, "B");
        assert_eq!(participants[1].uid, "B");
    }

    #[test]
    fn aliased_participant_reference_does_not_create_duplicate() {
        let statements = vec![
            participant("A"),
            participant_with_alias("Display B", "B"),
            message("A", "B", source(1)),
        ];

        let participants = build(&statements).participants;

        assert_eq!(participants.len(), 2);
        assert_eq!(participants[1].display_name, "Display B");
        assert_eq!(participants[1].alias.as_deref(), Some("B"));
        assert_eq!(participants[1].uid, "B");
    }

    #[test]
    fn aliased_participant_display_name_reference_is_rejected() {
        let statements = vec![
            participant("A"),
            participant_with_alias("Display B", "B"),
            message("A", "Display B", source(1)),
        ];

        match build_err(&statements) {
            SequenceResolverError::InvalidParticipantIdentifier {
                participant,
                reason,
                source_location,
            } => {
                assert_eq!(participant, "Display B");
                assert_eq!(reason, DISPLAY_NAME_REASON);
                assert_eq!(source_location, source(1));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn plain_display_name_of_aliased_participant_is_rejected_as_endpoint() {
        let statements = vec![
            participant_with_alias("Display", "d"),
            message("d", "Display", source(2)),
        ];

        assert!(matches!(
            build_err(&statements),
            SequenceResolverError::InvalidParticipantIdentifier { participant, reason, .. }
                if participant == "Display" && reason == DISPLAY_NAME_REASON
        ));
    }

    #[test]
    fn declared_name_equal_to_an_aliased_display_name_is_not_rejected() {
        let statements = vec![
            participant_with_alias("Display", "d"),
            participant("Display"),
            message("d", "Display", source(2)),
        ];

        assert_eq!(build(&statements).participants.len(), 2);
    }

    #[test]
    fn no_participants_declared_creates_implicit_participants() {
        let statements = vec![message("X", "Y", source(1))];

        let participants = build(&statements).participants;

        assert_eq!(participants.len(), 2);
        assert_eq!(participants[0].display_name, "X");
        assert_eq!(participants[1].display_name, "Y");
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

        let participants = build(&statements).participants;

        assert_eq!(participants.len(), 2);
        assert_eq!(participants[0].display_name, "A");
        assert_eq!(participants[0].source_location, message_location);
        assert_eq!(participants[1].display_name, "B");
        assert_eq!(participants[1].source_location, message_location);
    }

    #[test]
    fn destroy_statement_does_not_create_implicit_participant() {
        let statements = vec![Statement::DestroyCmd(DestroyCmd {
            participant: ParticipantRef {
                identifier: "Implicit".to_string(),
            },
            source_location: source(1),
        })];

        let table = build(&statements);

        assert!(table.participants.is_empty());
    }

    #[test]
    fn uid_follows_rule_b() {
        let statements = vec![
            participant_with_alias("comp::unit", "u"),
            participant_with_alias("Prose label", "p"),
            participant("Bare"),
        ];

        let table = build(&statements);
        let uids: Vec<_> = table.participants.iter().map(|p| p.uid.as_str()).collect();

        assert_eq!(uids, ["comp.unit", "p", "Bare"]);
    }

    #[test]
    fn references_resolve_to_the_uid_by_alias_or_name() {
        let statements = vec![
            participant_with_alias("comp::unit", "u"),
            participant("Bare"),
        ];
        let table = build(&statements);

        assert_eq!(
            table.uid_of_reference("u", &source(1)).unwrap().as_ref(),
            "comp.unit"
        );
        assert_eq!(
            table.uid_of_reference("Bare", &source(1)).unwrap().as_ref(),
            "Bare"
        );
    }

    #[test]
    fn display_name_of_aliased_participant_is_not_a_reference() {
        let table = build(&[participant_with_alias("Display", "d")]);

        assert!(matches!(
            table.uid_of_reference("Display", &source(4)),
            Err(SequenceResolverError::InvalidParticipantIdentifier { participant, reason, source_location })
                if participant == "Display"
                    && reason == DISPLAY_NAME_REASON
                    && source_location == source(4)
        ));
    }

    #[test]
    fn undeclared_reference_is_unknown() {
        let table = build(&[participant("A")]);

        assert!(matches!(
            table.uid_of_reference("Ghost", &source(4)),
            Err(SequenceResolverError::UnknownParticipant { reference, .. }) if reference == "Ghost"
        ));
    }

    #[test]
    fn create_with_display_name_of_aliased_participant_is_rejected() {
        let statements = vec![
            participant_with_alias("Display", "d"),
            Statement::CreateCmd(CreateCmd {
                participant_type: SyntaxParticipantType::Participant,
                identifier: ParticipantIdentifier {
                    display_name: "Display".to_string(),
                    alias: None,
                },
                stereotype: None,
                source_location: source(2),
            }),
        ];

        assert!(matches!(
            build_err(&statements),
            SequenceResolverError::InvalidParticipantIdentifier { participant, reason, source_location }
                if participant == "Display"
                    && reason == DISPLAY_NAME_REASON
                    && source_location == source(2)
        ));
    }

    #[test]
    fn explicit_declarations_with_the_same_uid_are_duplicates() {
        let statements = vec![
            participant_with_alias("comp::X", "a"),
            participant_with_alias("comp::X", "b"),
        ];

        assert!(matches!(
            build_err(&statements),
            SequenceResolverError::DuplicateParticipantId { participant_id, .. }
                if participant_id == "comp.X"
        ));
    }

    #[test]
    fn explicit_declarations_with_the_same_reference_name_are_duplicates() {
        let statements = vec![participant("A"), participant_with_alias("comp::B", "A")];

        assert!(matches!(
            build_err(&statements),
            SequenceResolverError::DuplicateParticipantReference { reference, .. }
                if reference == "A"
        ));
    }

    #[test]
    fn implicit_participant_with_taken_uid_is_a_duplicate() {
        let statements = vec![
            participant_with_alias("comp::X", "a"),
            message("a", "comp.X", source(3)),
        ];

        match build_err(&statements) {
            SequenceResolverError::DuplicateParticipantId {
                participant_id,
                source_location,
            } => {
                assert_eq!(participant_id, "comp.X");
                assert_eq!(source_location, source(3));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn free_text_without_alias_is_invalid() {
        let statements = vec![participant("Order Service")];

        assert!(matches!(
            build_err(&statements),
            SequenceResolverError::InvalidParticipantIdentifier { participant, .. }
                if participant == "Order Service"
        ));
    }

    #[test]
    fn create_reuses_an_already_known_participant() {
        let statements = vec![
            participant("A"),
            Statement::CreateCmd(CreateCmd {
                participant_type: SyntaxParticipantType::Participant,
                identifier: ParticipantIdentifier {
                    display_name: "A".to_string(),
                    alias: None,
                },
                stereotype: None,
                source_location: source(2),
            }),
        ];

        assert_eq!(build(&statements).participants.len(), 1);
    }

    #[test]
    fn root_anchor_prefixes_participant_uids() {
        let table = build_participant_table(
            &[participant_with_alias("comp::unit", "u")],
            &RootAnchor::new(Some("score::mw")),
        )
        .unwrap();

        assert_eq!(table.participants[0].uid, "score.mw.comp.unit");
    }
}
