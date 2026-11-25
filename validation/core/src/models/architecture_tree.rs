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

//! Parent tree of the architecture elements (SEooC, components, units).

use std::collections::BTreeMap;

use uid_utils::{resolve_uid, UidMatch};

use super::ComponentDiagramArchitecture;

/// Element id to parent id, filled from one or more edge sources.
pub(crate) struct ArchitectureTree {
    parents: BTreeMap<String, Option<String>>,
}

#[derive(Default)]
pub(crate) struct ArchitectureTreeBuilder {
    parents: BTreeMap<String, Option<String>>,
}

impl ArchitectureTreeBuilder {
    /// SEooC, component and unit elements of the component diagrams.
    pub(crate) fn component_diagrams(
        mut self,
        architecture: &ComponentDiagramArchitecture,
    ) -> Self {
        let elements = [
            &architecture.seooc_set,
            &architecture.comp_set,
            &architecture.unit_set,
        ]
        .into_iter()
        .flat_map(|set| set.values());
        for element in elements {
            self.parents
                .insert(element.id.clone(), element.parent_id.clone());
        }
        self
    }

    pub(crate) fn build(self) -> ArchitectureTree {
        ArchitectureTree {
            parents: self.parents,
        }
    }
}

impl ArchitectureTree {
    pub(crate) fn builder() -> ArchitectureTreeBuilder {
        ArchitectureTreeBuilder::default()
    }

    /// Id of the one element `uid` names; `None` if unknown or ambiguous.
    pub(crate) fn resolve(&self, uid: &str) -> Option<&str> {
        match resolve_uid(uid, self.parents.keys().map(String::as_str)) {
            UidMatch::Resolved(id) => Some(id),
            UidMatch::Ambiguous(_) | UidMatch::Unresolved => None,
        }
    }

    pub(crate) fn parent(&self, id: &str) -> Option<&str> {
        self.parents.get(id)?.as_deref()
    }

    /// `id` is a descendant of `container`.
    pub(crate) fn is_inside(&self, id: &str, container: &str) -> bool {
        let mut current = self.parent(id);
        while let Some(parent) = current {
            if parent == container {
                return true;
            }
            current = self.parent(parent);
        }
        false
    }
}
