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

//! Models for sequence-diagram FlatBuffer inputs used by design verification.

use std::collections::BTreeSet;

use sequence_logic::{Event, SequenceNode, SequenceTree};

use super::Errors;

/// One parsed sequence diagram from a FlatBuffer file.
pub struct SequenceDiagramInput {
    pub tree: SequenceTree,
    pub source_files: Vec<String>,
    pub version: Option<String>,
}

/// Collection of sequence diagrams loaded from one or more FlatBuffer files.
pub struct SequenceDiagramInputs {
    pub diagrams: Vec<SequenceDiagramInput>,
}

impl SequenceDiagramInputs {
    /// Build a [`SequenceDiagramIndex`] from sequence diagram inputs.
    pub fn to_sequence_diagram_index(&self, _errors: &mut Errors) -> SequenceDiagramIndex {
        SequenceDiagramIndex::from_diagrams(&self.diagrams)
    }
}

/// Indexed sequence-diagram data prepared for validators.
pub struct SequenceDiagramIndex {
    used_participants: BTreeSet<String>,
}

impl SequenceDiagramIndex {
    fn from_diagrams(diagrams: &[SequenceDiagramInput]) -> Self {
        let mut used_participants = BTreeSet::new();

        for diagram in diagrams {
            for node in &diagram.tree.root_interactions {
                collect_used_participants(node, &mut used_participants);
            }
        }

        Self { used_participants }
    }

    pub fn used_participants(&self) -> &BTreeSet<String> {
        &self.used_participants
    }
}

fn collect_used_participants(node: &SequenceNode, out: &mut BTreeSet<String>) {
    match &node.event {
        Event::Interaction(interaction) => {
            if !interaction.caller.is_empty() {
                out.insert(interaction.caller.clone());
            }
            if !interaction.callee.is_empty() {
                out.insert(interaction.callee.clone());
            }
        }
        Event::Return(ret) => {
            if !ret.caller.is_empty() {
                out.insert(ret.caller.clone());
            }
            if !ret.callee.is_empty() {
                out.insert(ret.callee.clone());
            }
        }
        Event::Condition(_) => {}
    }

    for child in &node.branches_node {
        collect_used_participants(child, out);
    }
}
