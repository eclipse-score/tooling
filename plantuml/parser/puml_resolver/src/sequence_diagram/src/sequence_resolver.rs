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

use crate::logic_parser::build_tree;
use resolver_traits::DiagramResolver;
use sequence_logic::{
    ParticipantType as LogicParticipantType, SequenceParticipant, SequenceTree, SourceLocation,
};
use sequence_parser::sequence_ast::{
    CreateCmd, MessageEndpoint, ParticipantDef, ParticipantIdentifier,
    ParticipantType as SyntaxParticipantType, Statement,
};
use sequence_parser::SeqPumlDocument;
use std::collections::HashSet;
use std::fmt;

/// Resolver for sequence diagrams.
///
/// Uses the single-pass pattern: `resolve` delegates entirely to `build_tree`,
/// which converts the flat statement list into a `SequenceTree`.  The resolver
/// carries no mutable state, so calling `resolve` multiple times is safe.
pub struct SequenceResolver;

/// Error type for `SequenceResolver`.
#[derive(Debug)]
pub enum SequenceResolverError {}

impl fmt::Display for SequenceResolverError {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl std::error::Error for SequenceResolverError {}

impl DiagramResolver for SequenceResolver {
    type Document = SeqPumlDocument;
    type Output = SequenceTree;
    type Error = SequenceResolverError;

    fn resolve(&mut self, document: &SeqPumlDocument) -> Result<SequenceTree, Self::Error> {
        let participants = build_participant_table(&document.statements);
        let root_interactions = build_tree(&document.statements);

        Ok(SequenceTree {
            name: document.name.clone(),
            participants,
            root_interactions,
        })
    }
}

fn build_participant_table(statements: &[Statement]) -> Vec<SequenceParticipant> {
    let mut resolved_names = HashSet::new();
    let mut participants = Vec::new();

    add_explicit_participants(statements, &mut participants, &mut resolved_names);
    add_implicit_participants(statements, &mut participants, &mut resolved_names);

    participants
}

fn add_explicit_participants(
    statements: &[Statement],
    participants: &mut Vec<SequenceParticipant>,
    resolved_names: &mut HashSet<String>,
) {
    for stmt in statements {
        match stmt {
            Statement::ParticipantDef(participant_def) => {
                add_participant(
                    participants,
                    resolved_names,
                    explicit_participant(participant_def),
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

fn add_implicit_participants(
    statements: &[Statement],
    participants: &mut Vec<SequenceParticipant>,
    resolved_names: &mut HashSet<String>,
) {
    for stmt in statements {
        if let Statement::Message(msg) = stmt {
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

fn participant_reference_name(identifier: &ParticipantIdentifier) -> &str {
    identifier
        .alias
        .as_deref()
        .unwrap_or(&identifier.display_name)
}

#[cfg(test)]
mod sequence_resolver_tests {
    use super::*;
    use parser_core::common_ast::{Arrow, ArrowDecor, ArrowLine};
    use resolver_traits::DiagramResolver;
    use sequence_logic::SourceLocation;
    use sequence_parser::sequence_ast::{
        Message, MessageEndpoint, ParticipantDef, ParticipantIdentifier,
        ParticipantType as SyntaxParticipantType, Statement,
    };

    fn solid_arrow() -> Arrow {
        Arrow {
            left: None,
            line: ArrowLine {
                raw: "-".to_string(),
            },
            middle: None,
            right: Some(ArrowDecor {
                raw: ">".to_string(),
            }),
        }
    }

    fn dashed_arrow() -> Arrow {
        Arrow {
            left: None,
            line: ArrowLine {
                raw: "--".to_string(),
            },
            middle: None,
            right: Some(ArrowDecor {
                raw: ">".to_string(),
            }),
        }
    }

    fn dummy_source_location() -> SourceLocation {
        SourceLocation::new("test.puml", 0)
    }

    fn message_endpoint(name: &str) -> MessageEndpoint {
        MessageEndpoint::Participant(ParticipantIdentifier {
            display_name: name.to_string(),
            alias: None,
        })
    }

    fn make_call(from: &str, to: &str, label: &str) -> Statement {
        Statement::Message(Message {
            left: message_endpoint(from),
            arrow: solid_arrow(),
            right: message_endpoint(to),
            suffix: None,
            description: Some(label.to_string()),
            source_location: dummy_source_location(),
        })
    }

    fn make_return(from: &str, to: &str, label: &str) -> Statement {
        Statement::Message(Message {
            left: message_endpoint(from),
            arrow: dashed_arrow(),
            right: message_endpoint(to),
            suffix: None,
            description: Some(label.to_string()),
            source_location: dummy_source_location(),
        })
    }

    /// SequenceResolver must implement DiagramResolver — compile-time check.
    #[test]
    fn test_implements_diagram_resolver_trait() {
        fn assert_is_diagram_resolver<R: DiagramResolver>() {}
        assert_is_diagram_resolver::<SequenceResolver>();
    }

    /// An empty diagram produces an empty SequenceTree.
    #[test]
    fn test_empty_document_yields_empty_tree() {
        let mut resolver = SequenceResolver;
        let doc = SeqPumlDocument {
            name: Some("empty".to_string()),
            statements: vec![],
        };
        let tree = resolver.resolve(&doc).expect("must not fail");
        assert!(tree.root_interactions.is_empty());
        assert_eq!(tree.name.as_deref(), Some("empty"));
    }

    /// A single call with its matching return produces one Interaction node.
    #[test]
    fn test_call_and_return_produce_one_interaction_node() {
        let stmts = vec![
            make_call("A", "B", "doWork"),
            make_return("B", "A", "result"),
        ];
        let mut resolver = SequenceResolver;
        let doc = SeqPumlDocument {
            name: Some("test".to_string()),
            statements: stmts,
        };
        let tree = resolver.resolve(&doc).expect("must not fail");
        assert_eq!(
            tree.root_interactions.len(),
            1,
            "one call + matching return = one Interaction node at root level"
        );
    }

    /// resolve must be callable multiple times without carrying state from a previous call.
    #[test]
    fn test_resolver_is_stateless_across_calls() {
        let stmts = vec![make_call("A", "B", "ping")];
        let doc1 = SeqPumlDocument {
            name: Some("first".to_string()),
            statements: stmts.clone(),
        };
        let doc2 = SeqPumlDocument {
            name: Some("second".to_string()),
            statements: stmts,
        };

        let mut resolver = SequenceResolver;
        let tree1 = resolver.resolve(&doc1).unwrap();
        let tree2 = resolver.resolve(&doc2).unwrap();

        assert_eq!(tree1.root_interactions.len(), tree2.root_interactions.len());
    }

    fn make_participant(name: &str) -> Statement {
        Statement::ParticipantDef(ParticipantDef {
            participant_type: SyntaxParticipantType::Participant,
            identifier: ParticipantIdentifier {
                display_name: name.to_string(),
                alias: None,
            },
            stereotype: None,
            source_location: dummy_source_location(),
        })
    }

    fn make_participant_with_alias(display_name: &str, alias: &str) -> Statement {
        Statement::ParticipantDef(ParticipantDef {
            participant_type: SyntaxParticipantType::Participant,
            identifier: ParticipantIdentifier {
                display_name: display_name.to_string(),
                alias: Some(alias.to_string()),
            },
            stereotype: None,
            source_location: dummy_source_location(),
        })
    }

    /// Explicit participants remain in the symbol table and message references
    /// to them do not create duplicates.
    #[test]
    fn test_declared_participants_are_preserved_without_duplicates() {
        let stmts = vec![
            make_participant("A"),
            make_participant("B"),
            make_call("A", "B", "doWork"),
            make_return("B", "A", "result"),
        ];
        let mut resolver = SequenceResolver;
        let doc = SeqPumlDocument {
            name: Some("valid".to_string()),
            statements: stmts,
        };
        let tree = resolver.resolve(&doc).expect("must not fail");
        assert_eq!(tree.participants.len(), 2);
        assert_eq!(tree.participants[0].display_name, "A");
        assert_eq!(tree.participants[1].display_name, "B");
    }

    #[test]
    fn test_aliased_participant_reference_does_not_create_duplicate() {
        let stmts = vec![
            make_participant("A"),
            make_participant_with_alias("Display B", "B"),
            make_call("A", "B", "doWork"),
        ];
        let mut resolver = SequenceResolver;
        let doc = SeqPumlDocument {
            name: Some("valid_alias".to_string()),
            statements: stmts,
        };
        let tree = resolver.resolve(&doc).expect("must not fail");
        assert_eq!(tree.participants.len(), 2);
        assert_eq!(tree.participants[1].display_name, "Display B");
        assert_eq!(tree.participants[1].alias.as_deref(), Some("B"));
    }

    #[test]
    fn test_aliased_participant_display_name_reference_creates_implicit_participant() {
        let stmts = vec![
            make_participant("A"),
            make_participant_with_alias("Display B", "B"),
            make_call("A", "Display B", "doWork"),
        ];
        let mut resolver = SequenceResolver;
        let doc = SeqPumlDocument {
            name: Some("invalid_display_reference".to_string()),
            statements: stmts,
        };
        let tree = resolver.resolve(&doc).expect("must not fail");
        assert_eq!(tree.participants.len(), 3);
        assert_eq!(tree.participants[2].display_name, "Display B");
        assert_eq!(tree.participants[2].alias, None);
    }

    /// When no participants are declared, message endpoints form the participant table.
    #[test]
    fn test_no_participants_declared_creates_implicit_participants() {
        let stmts = vec![make_call("X", "Y", "hello")];
        let mut resolver = SequenceResolver;
        let doc = SeqPumlDocument {
            name: Some("implicit".to_string()),
            statements: stmts,
        };
        let tree = resolver.resolve(&doc).expect("must not fail");
        assert_eq!(tree.participants.len(), 2);
        assert_eq!(tree.participants[0].display_name, "X");
        assert_eq!(tree.participants[1].display_name, "Y");
    }

    /// Resolver output nodes must preserve source_location provenance.
    #[test]
    fn test_source_locations_are_preserved() {
        let call_location = SourceLocation::new("sequence/provenance_case.puml", 42);
        let return_location = SourceLocation::new("sequence/provenance_case.puml", 43);

        let stmts = vec![
            Statement::Message(Message {
                left: message_endpoint("A"),
                arrow: solid_arrow(),
                right: message_endpoint("B"),
                suffix: None,
                description: Some("doWork".to_string()),
                source_location: call_location.clone(),
            }),
            Statement::Message(Message {
                left: message_endpoint("B"),
                arrow: dashed_arrow(),
                right: message_endpoint("A"),
                suffix: None,
                description: Some("result".to_string()),
                source_location: return_location.clone(),
            }),
        ];

        let mut resolver = SequenceResolver;
        let doc = SeqPumlDocument {
            name: Some("provenance".to_string()),
            statements: stmts,
        };

        let tree = resolver.resolve(&doc).expect("must not fail");
        assert_eq!(tree.root_interactions.len(), 1);

        let interaction = &tree.root_interactions[0];
        assert_eq!(interaction.source_location, call_location);

        assert_eq!(interaction.branches_node.len(), 1);
        let ret = &interaction.branches_node[0];
        assert_eq!(ret.source_location, return_location);
    }
}
