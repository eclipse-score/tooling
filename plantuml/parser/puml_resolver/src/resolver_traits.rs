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

/// One document of a [`DiagramResolver::resolve_group`] group; `name`
/// attributes errors to the file.
#[derive(Debug, Clone, Copy)]
pub struct DiagramSource<'a, D> {
    pub name: &'a str,
    pub document: &'a D,
}

/// An error from [`DiagramResolver::resolve_group`], attributed to a file.
#[derive(Debug)]
pub struct GroupResolveError<E> {
    pub source_name: String,
    pub source: E,
}

impl<E> GroupResolveError<E> {
    pub fn in_file(source_name: &str, source: E) -> Self {
        Self {
            source_name: source_name.to_string(),
            source,
        }
    }
}

impl<E: std::fmt::Display> std::fmt::Display for GroupResolveError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "in {}: {}", self.source_name, self.source)
    }
}

impl<E: std::error::Error + 'static> std::error::Error for GroupResolveError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Resolver trait for PlantUML diagrams.
///
/// Implementations convert a parsed document (`Self::Document`) into a logic model
/// (`Self::Output`).  Two patterns are used across the three diagram resolvers:
///
/// * **Per-statement visitor** (`ComponentResolver`): `resolve` iterates the
///   statement list and delegates each entry to a private helper, maintaining a
///   mutable scope stack between calls.  This suits diagrams that can be processed
///   one statement at a time with carrying state (e.g. nested component scopes).
///
/// * **Single-pass analysis** (`ClassResolver`, `SequenceResolver`): `resolve`
///   delegates to a private `analyze()` / `build_tree()` function that processes
///   the whole document at once.  Use this pattern when the resolver needs multiple
///   passes or sees the whole statement list before making decisions (e.g. the class
///   resolver registers all type names before resolving member types).
///
/// In both cases only `resolve` is part of the public trait contract.  Per-statement
/// helpers are ordinary private methods; callers always go through `resolve`.
pub trait DiagramResolver {
    type Document;
    type Output;
    type Error: std::error::Error + 'static;

    fn resolve(&mut self, document: &Self::Document) -> Result<Self::Output, Self::Error>;

    /// Resolves `documents` as one group: one `Output` per document, in order.
    ///
    /// The default resolves each document alone with a fresh `Self::default()`.
    fn resolve_group(
        documents: &[DiagramSource<'_, Self::Document>],
    ) -> Result<Vec<Self::Output>, GroupResolveError<Self::Error>>
    where
        Self: Default + Sized,
    {
        documents
            .iter()
            .map(|doc| {
                Self::default()
                    .resolve(doc.document)
                    .map_err(|source| GroupResolveError::in_file(doc.name, source))
            })
            .collect()
    }
}
