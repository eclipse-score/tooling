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
use sequence_parser::SeqPumlDocument;

/// Resolver for sequence diagrams.
///
/// Uses the single-pass pattern: `resolve` delegates entirely to `build_tree`,
/// which converts the flat statement list into a `SequenceTree`.  The resolver
/// carries no mutable state, so calling `resolve` multiple times is safe.
pub struct SequenceResolver;

/// Error type for `SequenceResolver`.
///
/// `build_tree` is currently infallible, so this enum has no variants.
/// It satisfies the `std::error::Error` bound required by the CLI's generic
/// `puml_resolver<R>` helper.
#[derive(Debug)]
pub enum SequenceResolverError {}

impl std::fmt::Display for SequenceResolverError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl std::error::Error for SequenceResolverError {}

impl DiagramResolver for SequenceResolver {
    type Document = SeqPumlDocument;
    type Output = SequenceTree;
    type Error = SequenceResolverError;

    fn resolve(&mut self, document: &SeqPumlDocument) -> Result<SequenceTree, Self::Error> {
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
    use sequence_parser::syntax_ast::{Message, MessageContent, Statement};

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

    fn make_call(from: &str, to: &str, label: &str) -> Statement {
        Statement::Message(Message {
            content: MessageContent::WithTargets {
                left: from.to_string(),
                arrow: solid_arrow(),
                right: to.to_string(),
            },
            activation_marker: None,
            description: Some(label.to_string()),
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
}
