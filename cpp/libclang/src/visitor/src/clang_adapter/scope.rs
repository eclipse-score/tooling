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

//! Shared semantic-scope extraction helpers for libclang entities.

use clang::{Entity, EntityKind};
use cpp_semantics::Scope;

// ── Namespace scopes ───────────────────────────────────────────────────────

/// Returns the enclosing namespace names from outermost to innermost.
pub(crate) fn namespace_path(entity: &Entity) -> Vec<String> {
    let mut namespaces = Vec::new();
    let mut current = entity.get_semantic_parent();

    while let Some(parent) = current {
        if parent.get_kind() == EntityKind::Namespace {
            // Anonymous namespaces have no stable name in libclang and are intentionally
            // treated as transparent scopes. The current model does not distinguish
            // same-named declarations from separate anonymous namespaces.
            if let Some(name) = parent.get_name() {
                namespaces.push(name);
            }
        }
        current = parent.get_semantic_parent();
    }

    namespaces.reverse();
    namespaces
}

/// Returns the enclosing namespace as a C++ qualified identifier.
pub(crate) fn namespace_id(entity: &Entity) -> Option<String> {
    let path = namespace_path(entity);
    (!path.is_empty()).then(|| path.join("::"))
}

// ── Type scopes ────────────────────────────────────────────────────────────

/// Returns the enclosing type names from outermost to innermost.
pub(crate) fn type_scope_path(entity: &Entity) -> Option<Vec<String>> {
    let mut types = Vec::new();
    let mut current = Some(*entity);

    while let Some(parent) = current {
        if is_type_scope(parent.get_kind()) {
            types.push(type_scope_name(&parent)?);
        }
        current = parent.get_semantic_parent();
    }

    types.reverse();
    (!types.is_empty()).then_some(types)
}

/// Returns whether an entity kind can own C++ member callables.
pub(crate) fn is_type_scope(kind: EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::ClassDecl
            | EntityKind::StructDecl
            | EntityKind::UnionDecl
            | EntityKind::ClassTemplate
            | EntityKind::ClassTemplatePartialSpecialization
    )
}

fn type_scope_name(entity: &Entity) -> Option<String> {
    match entity.get_kind() {
        EntityKind::ClassTemplatePartialSpecialization => {
            entity.get_display_name().or_else(|| entity.get_name())
        }
        kind if is_type_scope(kind) => entity.get_name(),
        _ => None,
    }
}

// ── Named declaration parents ──────────────────────────────────────────────

/// Returns named semantic parents that can own a nested C++ declaration.
///
/// This intentionally excludes aliases, template parameters, and enums: they
/// appear in libclang's semantic-parent chain but cannot own named nested types.
pub(crate) fn semantic_parent_id(entity: &Entity) -> Option<String> {
    let mut parents = Vec::new();
    let mut current = entity.get_semantic_parent();

    while let Some(parent) = current {
        let name = match parent.get_kind() {
            EntityKind::Namespace => parent.get_name(),
            kind if is_type_scope(kind) => type_scope_name(&parent),
            _ => None,
        };

        if let Some(name) = name {
            parents.push(name);
        }
        current = parent.get_semantic_parent();
    }

    parents.reverse();
    (!parents.is_empty()).then(|| parents.join("::"))
}

// ── Callable scopes ────────────────────────────────────────────────────────

pub(crate) fn callable_scope(entity: &Entity) -> Option<Scope> {
    match entity.get_semantic_parent() {
        Some(parent) if is_type_scope(parent.get_kind()) => Some(Scope::Type {
            namespace: namespace_path(&parent),
            type_path: type_scope_path(&parent)?,
        }),
        _ => {
            let namespace = namespace_path(entity);
            Some(if namespace.is_empty() {
                Scope::Global
            } else {
                Scope::Namespace(namespace)
            })
        }
    }
}
