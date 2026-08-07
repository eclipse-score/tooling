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

use resolver_traits::DiagramResolver;
use sequence_logic::SequenceTree;
use sequence_parser::SeqPumlDocument;

use crate::error::SequenceResolverError;
use crate::lifecycle_validator::validate_lifecycle_consistency;
use crate::participant_table::build_participant_table;
use crate::statement_resolver::build_sequence_tree;

/// Resolver for sequence diagrams.
///
/// `resolve` builds the participant table and sequence logic tree, then
/// validates lifecycle consistency before assembling the final `SequenceTree`.
/// The resolver stores no per-document state, so the same instance can resolve
/// multiple documents safely.
pub struct SequenceResolver;

impl DiagramResolver for SequenceResolver {
    type Document = SeqPumlDocument;
    type Output = SequenceTree;
    type Error = SequenceResolverError;

    fn resolve(&mut self, document: &SeqPumlDocument) -> Result<SequenceTree, Self::Error> {
        let participants = build_participant_table(&document.statements);
        let root = build_sequence_tree(&document.statements)?;
        validate_lifecycle_consistency(&root)?;

        Ok(SequenceTree {
            name: document.name.clone(),
            participants,
            root,
        })
    }
}

#[cfg(test)]
mod sequence_resolver_tests {
    use super::SequenceResolver;
    use parser_core::common_ast::{Arrow, ArrowDecor, ArrowLine};
    use resolver_traits::DiagramResolver;
    use sequence_logic::SourceLocation;
    use sequence_parser::sequence_ast::{
        Message, MessageEndpoint, ParticipantIdentifier, SeqPumlDocument, Statement,
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
        assert!(tree.root.items.is_empty());
        assert_eq!(tree.name.as_deref(), Some("empty"));
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

        assert_eq!(tree1.root.items.len(), tree2.root.items.len());
    }
}
