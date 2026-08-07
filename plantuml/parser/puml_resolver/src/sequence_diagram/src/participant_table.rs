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

use std::collections::HashSet;

use sequence_logic::{
    ParticipantType as LogicParticipantType, SequenceParticipant, SourceLocation,
};
use sequence_parser::sequence_ast::{
    CreateCmd, MessageEndpoint, ParticipantDef, ParticipantIdentifier,
    ParticipantType as SyntaxParticipantType, Statement,
};

pub(crate) fn build_participant_table(statements: &[Statement]) -> Vec<SequenceParticipant> {
    let mut resolved_names = HashSet::new();
    let mut participants = Vec::new();

    add_explicit_participants(statements, &mut participants, &mut resolved_names);

    // Message endpoints and create commands must share the ordered pass so the
    // first participant reference keeps its source location.
    add_implicit_participants(statements, &mut participants, &mut resolved_names);

    participants
}

fn add_explicit_participants(
    statements: &[Statement],
    participants: &mut Vec<SequenceParticipant>,
    resolved_names: &mut HashSet<String>,
) {
    for stmt in statements {
        if let Statement::ParticipantDef(participant_def) = stmt {
            add_participant(
                participants,
                resolved_names,
                explicit_participant(participant_def),
            );
        }
    }
}

fn add_implicit_participants(
    statements: &[Statement],
    participants: &mut Vec<SequenceParticipant>,
    resolved_names: &mut HashSet<String>,
) {
    for stmt in statements {
        match stmt {
            Statement::Message(msg) => {
                add_endpoint_participant(
                    participants,
                    resolved_names,
                    &msg.left,
                    &msg.source_location,
                );
                add_endpoint_participant(
                    participants,
                    resolved_names,
                    &msg.right,
                    &msg.source_location,
                );
            }
            Statement::CreateCmd(create_cmd) => {
                add_participant(
                    participants,
                    resolved_names,
                    created_participant(create_cmd),
                );
            }
            _ => {}
        }
    }
}

fn add_participant(
    participants: &mut Vec<SequenceParticipant>,
    resolved_names: &mut HashSet<String>,
    participant: SequenceParticipant,
) {
    let identifier = ParticipantIdentifier {
        display_name: participant.display_name.clone(),
        alias: participant.alias.clone(),
    };
    let reference_name = participant_reference_name(&identifier);
    if reference_name.is_empty() || !resolved_names.insert(reference_name.to_string()) {
        return;
    }
    participants.push(participant);
}

fn add_endpoint_participant(
    participants: &mut Vec<SequenceParticipant>,
    resolved_names: &mut HashSet<String>,
    endpoint: &MessageEndpoint,
    source_location: &SourceLocation,
) {
    if let MessageEndpoint::Participant(identifier) = endpoint {
        add_participant(
            participants,
            resolved_names,
            implicit_participant(identifier, source_location),
        );
    }
}

fn explicit_participant(participant_def: &ParticipantDef) -> SequenceParticipant {
    SequenceParticipant {
        display_name: participant_def.identifier.display_name.clone(),
        alias: participant_def.identifier.alias.clone(),
        participant_type: map_parser_participant_type(&participant_def.participant_type),
        source_location: participant_def.source_location.clone(),
        stereotype: participant_def.stereotype.clone(),
    }
}

fn created_participant(create_cmd: &CreateCmd) -> SequenceParticipant {
    SequenceParticipant {
        display_name: create_cmd.identifier.display_name.clone(),
        alias: create_cmd.identifier.alias.clone(),
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

#[cfg(test)]
mod participant_table_tests {
    use super::*;
    use parser_core::common_ast::{Arrow, ArrowDecor, ArrowLine};
    use sequence_parser::sequence_ast::{DestroyCmd, Message, ParticipantRef};

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

        let participants = build_participant_table(&statements);

        assert_eq!(participants.len(), 2);
        assert_eq!(participants[0].display_name, "A");
        assert_eq!(participants[1].display_name, "B");
    }

    #[test]
    fn aliased_participant_reference_does_not_create_duplicate() {
        let statements = vec![
            participant("A"),
            participant_with_alias("Display B", "B"),
            message("A", "B", source(1)),
        ];

        let participants = build_participant_table(&statements);

        assert_eq!(participants.len(), 2);
        assert_eq!(participants[1].display_name, "Display B");
        assert_eq!(participants[1].alias.as_deref(), Some("B"));
    }

    #[test]
    fn aliased_participant_display_name_reference_creates_implicit_participant() {
        let statements = vec![
            participant("A"),
            participant_with_alias("Display B", "B"),
            message("A", "Display B", source(1)),
        ];

        let participants = build_participant_table(&statements);

        assert_eq!(participants.len(), 3);
        assert_eq!(participants[2].display_name, "Display B");
        assert_eq!(participants[2].alias, None);
    }

    #[test]
    fn no_participants_declared_creates_implicit_participants() {
        let statements = vec![message("X", "Y", source(1))];

        let participants = build_participant_table(&statements);

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

        let participants = build_participant_table(&statements);

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

        let participants = build_participant_table(&statements);

        assert!(participants.is_empty());
    }
}
