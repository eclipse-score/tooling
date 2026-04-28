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

//! Models for class-diagram FlatBuffer inputs used by design verification.

use std::collections::BTreeSet;

use class_diagram::ClassDiagram as ClassDiagramInput;

use super::Errors;

/// Collection of class diagrams loaded from one or more FlatBuffer files.
pub type ClassDiagramInputs = Vec<ClassDiagramInput>;

/// Indexed class-diagram data prepared for validators.
pub struct ClassDiagramIndex {
    observed_enclosing_namespace_ids: BTreeSet<String>,
}

impl ClassDiagramIndex {
    /// Build a [`ClassDiagramIndex`] from class diagram inputs.
    pub fn build_index(diagrams: &[ClassDiagramInput], _errors: &mut Errors) -> Self {
        let observed_enclosing_namespace_ids = diagrams
            .iter()
            .flat_map(|diagram| diagram.entities.iter())
            .filter_map(|entity| entity.enclosing_namespace_id.clone())
            .filter(|namespace_id| !namespace_id.is_empty())
            .collect();

        Self {
            observed_enclosing_namespace_ids,
        }
    }

    pub fn enclosing_namespace_ids(&self) -> &BTreeSet<String> {
        &self.observed_enclosing_namespace_ids
    }
}
