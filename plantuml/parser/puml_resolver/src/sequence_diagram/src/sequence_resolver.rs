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
use sequence_logic::SequenceTree;
use sequence_parser::syntax_ast::{MessageContent, ParticipantIdentifier, Statement};
use sequence_parser::SeqPumlDocument;
use std::collections::HashSet;

/// Resolver for sequence diagrams.
///
/// Uses the single-pass pattern: `resolve` delegates entirely to `build_tree`,
/// which converts the flat statement list into a `SequenceTree`.  The resolver
/// carries no mutable state, so calling `resolve` multiple times is safe.
pub struct SequenceResolver;

/// Error type for `SequenceResolver`.
#[derive(Debug)]
pub enum SequenceResolverError {
    /// A message references a participant that was not declared in a
    /// `participant` (or actor/boundary/…) statement.
    UndeclaredParticipant { name: String, role: &'static str },
}

impl std::fmt::Display for SequenceResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SequenceResolverError::UndeclaredParticipant { name, role } => {
                write!(f, "{role} '{name}' is not declared as a participant")
            }
        }
    }
}

impl std::error::Error for SequenceResolverError {}

/// Collect all identifiers that a `ParticipantIdentifier` makes available.
fn collect_participant_names(id: &ParticipantIdentifier, out: &mut HashSet<String>) {
    match id {
        ParticipantIdentifier::QuotedAsId { quoted, id } => {
            out.insert(quoted.clone());
            out.insert(id.clone());
        }
        ParticipantIdentifier::IdAsQuoted { id, quoted } => {
            out.insert(id.clone());
            out.insert(quoted.clone());
        }
        ParticipantIdentifier::IdAsId { id1, id2 } => {
            out.insert(id1.clone());
            out.insert(id2.clone());
        }
        ParticipantIdentifier::Quoted(s) | ParticipantIdentifier::Id(s) => {
            out.insert(s.clone());
        }
    }
}

impl DiagramResolver for SequenceResolver {
    type Document = SeqPumlDocument;
    type Output = SequenceTree;
    type Error = SequenceResolverError;

    fn resolve(&mut self, document: &SeqPumlDocument) -> Result<SequenceTree, Self::Error> {
        // 1. Collect declared participants.
        let mut declared = HashSet::new();
        for stmt in &document.statements {
            if let Statement::ParticipantDef(p) = stmt {
                collect_participant_names(&p.identifier, &mut declared);
            }
        }

        // 2. Validate message targets only when participants are declared.
        if !declared.is_empty() {
            for stmt in &document.statements {
                if let Statement::Message(msg) = stmt {
                    let MessageContent::WithTargets { left, right, .. } = &msg.content;
                    if !left.is_empty() && !declared.contains(left) {
                        return Err(SequenceResolverError::UndeclaredParticipant {
                            name: left.clone(),
                            role: "caller",
                        });
                    }
                    if !right.is_empty() && !declared.contains(right) {
                        return Err(SequenceResolverError::UndeclaredParticipant {
                            name: right.clone(),
                            role: "callee",
                        });
                    }
                }
            }
        }

        // 3. Build the tree.
        let root_interactions = build_tree(&document.statements);
        Ok(SequenceTree {
            name: document.name.clone(),
            root_interactions,
        })
    }
}

#[cfg(test)]
mod sequence_resolver_tests {
    use super::*;
    use parser_core::common_ast::{Arrow, ArrowDecor, ArrowLine};
    use resolver_traits::DiagramResolver;
    use sequence_logic::SourceLocation;
    use sequence_parser::syntax_ast::{
        Message, MessageContent, ParticipantDef, ParticipantIdentifier, ParticipantType, Statement,
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

    fn make_call(from: &str, to: &str, label: &str) -> Statement {
        Statement::Message(Message {
            content: MessageContent::WithTargets {
                left: from.to_string(),
                arrow: solid_arrow(),
                right: to.to_string(),
            },
            activation_marker: None,
            description: Some(label.to_string()),
            source_location: dummy_source_location(),
        })
    }

    fn make_return(from: &str, to: &str, label: &str) -> Statement {
        Statement::Message(Message {
            content: MessageContent::WithTargets {
                left: from.to_string(),
                arrow: dashed_arrow(),
                right: to.to_string(),
            },
            activation_marker: None,
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
            participant_type: ParticipantType::Participant,
            identifier: ParticipantIdentifier::Id(name.to_string()),
            stereotype: None,
            source_location: dummy_source_location(),
        })
    }

    /// When participants are declared, all message targets must be among them.
    #[test]
    fn test_declared_participants_pass_validation() {
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
        assert!(resolver.resolve(&doc).is_ok());
    }

    /// An undeclared callee should cause an error.
    #[test]
    fn test_undeclared_callee_raises_error() {
        let stmts = vec![make_participant("A"), make_call("A", "B", "doWork")];
        let mut resolver = SequenceResolver;
        let doc = SeqPumlDocument {
            name: Some("bad_callee".to_string()),
            statements: stmts,
        };
        let err = resolver.resolve(&doc).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("B"),
            "error should name the undeclared participant"
        );
        assert!(msg.contains("callee"), "error should indicate the role");
    }

    /// An undeclared caller should cause an error.
    #[test]
    fn test_undeclared_caller_raises_error() {
        let stmts = vec![make_participant("B"), make_call("A", "B", "doWork")];
        let mut resolver = SequenceResolver;
        let doc = SeqPumlDocument {
            name: Some("bad_caller".to_string()),
            statements: stmts,
        };
        let err = resolver.resolve(&doc).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("A"),
            "error should name the undeclared participant"
        );
        assert!(msg.contains("caller"), "error should indicate the role");
    }

    /// When no participants are declared, messages are allowed freely (no validation).
    #[test]
    fn test_no_participants_declared_skips_validation() {
        let stmts = vec![make_call("X", "Y", "hello")];
        let mut resolver = SequenceResolver;
        let doc = SeqPumlDocument {
            name: Some("implicit".to_string()),
            statements: stmts,
        };
        assert!(resolver.resolve(&doc).is_ok());
    }

    /// Resolver output nodes must preserve source_location provenance.
    #[test]
    fn test_source_locations_are_preserved() {
        let call_location = SourceLocation::new("sequence/provenance_case.puml", 42);
        let return_location = SourceLocation::new("sequence/provenance_case.puml", 43);

        let stmts = vec![
            Statement::Message(Message {
                content: MessageContent::WithTargets {
                    left: "A".to_string(),
                    arrow: solid_arrow(),
                    right: "B".to_string(),
                },
                activation_marker: None,
                description: Some("doWork".to_string()),
                source_location: call_location.clone(),
            }),
            Statement::Message(Message {
                content: MessageContent::WithTargets {
                    left: "B".to_string(),
                    arrow: dashed_arrow(),
                    right: "A".to_string(),
                },
                activation_marker: None,
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
