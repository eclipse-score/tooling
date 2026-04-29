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
    type Error;

    fn resolve(&mut self, document: &Self::Document) -> Result<Self::Output, Self::Error>;
}
