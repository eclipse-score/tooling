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

