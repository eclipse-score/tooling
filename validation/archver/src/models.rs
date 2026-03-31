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

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Shared key type
// ---------------------------------------------------------------------------

/// Composite key: `(canonical_alias, parent_alias)`.  `parent_alias` is `None`
/// for top-level entities.  Using the parent as part of the key means two
/// identically-named entities under different parents are treated as distinct.
pub type EntityKey = (String, Option<String>);

/// Extract the target name from a Bazel label like `@//path/to/package:target`
/// → `"target"`.  Returns the full label unchanged if it contains no colon.
/// Returns `Err` if the extracted name is empty.
pub(crate) fn label_short_name(label: &str) -> Result<&str, String> {
    let name = label.rsplit_once(':').map(|(_, n)| n).unwrap_or(label);
    if name.is_empty() {
        return Err(format!("Empty target name extracted from label: {label:?}"));
    }
    Ok(name)
}

// ---------------------------------------------------------------------------
// Bazel architecture JSON model
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BazelInput {
    pub components: BTreeMap<String, BazelInputEntry>,
}

impl BazelInput {
    /// Build a [`BazelArchitecture`] index from this architecture JSON.
    ///
    /// A pre-pass collects all **full** labels of components that appear as
    /// children of another component so that only their exact label is used
    /// for child-suppression — preventing a top-level component from being
    /// silently treated as nested just because another target in a different
    /// package shares the same short name.
    pub fn to_bazel_architecture(&self, errors: &mut Errors) -> BazelArchitecture {
        let mut seooc_set = BTreeMap::new();
        let mut comp_set = BTreeMap::new();
        let mut unit_set = BTreeMap::new();

        let child_labels: BTreeSet<String> = self
            .components
            .values()
            .flat_map(|e| e.components.iter())
            .map(|l| l.to_lowercase())
            .collect();

        for (comp_label, entry) in &self.components {
            let comp_key = match label_short_name(comp_label) {
                Ok(name) => name.to_lowercase(),
                Err(msg) => {
                    errors.push(msg);
                    continue;
                }
            };

            if !child_labels.contains(&comp_label.to_lowercase()) {
                // Top-level entries are dependable elements (SEooC)
                let key = (comp_key.clone(), None);
                if let Some(prev) = seooc_set.insert(key.clone(), comp_label.clone()) {
                    errors.push(format!(
                        "Duplicate dependable element key in Bazel build graph:\n\
                           Key   : {:?}\n\
                           Labels: {} and {}",
                        key, prev, comp_label
                    ));
                }
            }

            for u_label in &entry.units {
                let u_key = match label_short_name(u_label) {
                    Ok(name) => name.to_lowercase(),
                    Err(msg) => {
                        errors.push(msg);
                        continue;
                    }
                };
                let key = (u_key, Some(comp_key.clone()));
                if let Some(prev) = unit_set.insert(key.clone(), u_label.clone()) {
                    errors.push(format!(
                        "Duplicate unit key in Bazel build graph:\n\
                           Key   : {:?}\n\
                           Labels: {} and {}",
                        key, prev, u_label
                    ));
                }
            }

            for c_label in &entry.components {
                let c_key = match label_short_name(c_label) {
                    Ok(name) => name.to_lowercase(),
                    Err(msg) => {
                        errors.push(msg);
                        continue;
                    }
                };
                let key = (c_key, Some(comp_key.clone()));
                if let Some(prev) = comp_set.insert(key.clone(), c_label.clone()) {
                    errors.push(format!(
                        "Duplicate component key in Bazel build graph:\n\
                           Key   : {:?}\n\
                           Labels: {} and {}",
                        key, prev, c_label
                    ));
                }
            }
        }

        BazelArchitecture {
            seooc_set,
            comp_set,
            unit_set,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BazelInputEntry {
    #[serde(default)]
    pub units: Vec<String>,
    #[serde(default)]
    pub components: Vec<String>,
}

/// Indexed entity key-maps derived from the Bazel build graph.
///
/// Map values are the original Bazel label strings.
/// Built via [`BazelInput::to_bazel_architecture`].
pub struct BazelArchitecture {
    /// Top-level dependable elements (`<<SEooC>>`), keyed with `parent = None`.
    pub seooc_set: BTreeMap<EntityKey, String>,
    /// Nested components (`<<component>>`), keyed with `parent = Some(..)`.
    pub comp_set: BTreeMap<EntityKey, String>,
    pub unit_set: BTreeMap<EntityKey, String>,
}

// ---------------------------------------------------------------------------
// PlantUML diagram model
// ---------------------------------------------------------------------------

/// A single component-level entity parsed from a PlantUML `.fbs.bin` file.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagramInput {
    pub id: String,
    pub alias: Option<String>,
    pub parent_id: Option<String>,
    pub stereotype: Option<String>,
}

impl DiagramInput {
    /// Canonical match key: alias (lowercased) when present, otherwise raw id.
    pub fn match_key(&self) -> String {
        self.alias.as_deref().unwrap_or(&self.id).to_lowercase()
    }

    pub fn is_component(&self) -> bool {
        self.stereotype.as_deref() == Some("component")
    }

    pub fn is_unit(&self) -> bool {
        self.stereotype.as_deref() == Some("unit")
    }

    /// Returns `true` for `<<SEooC>>` package entities (dependable elements).
    pub fn is_seooc_package(&self) -> bool {
        self.stereotype.as_deref() == Some("SEooC")
    }
}

/// Collection of raw PlantUML entities read from FlatBuffers files.
///
/// Symmetric peer of [`BazelInput`]: produced by [`DiagramReader`] and
/// consumed by [`to_diagram_architecture`](DiagramInputs::to_diagram_architecture).
pub struct DiagramInputs {
    pub entities: Vec<DiagramInput>,
}

impl DiagramInputs {
    /// Build a [`DiagramArchitecture`] index from these diagram inputs.
    pub fn to_diagram_architecture(&self, errors: &mut Errors) -> DiagramArchitecture {
        DiagramArchitecture::from_entities(&self.entities, errors)
    }
}

/// Indexed entity key-maps derived from the parsed PlantUML diagram entities.
///
/// Built via [`DiagramInputs::to_diagram_architecture`].
pub struct DiagramArchitecture {
    /// `<<SEooC>>` package entities, keyed with `parent = None`.
    pub seooc_set: BTreeMap<EntityKey, DiagramInput>,
    /// `<<component>>` entities, keyed with `parent = Some(..)`.
    pub comp_set: BTreeMap<EntityKey, DiagramInput>,
    pub unit_set: BTreeMap<EntityKey, DiagramInput>,
    /// Full raw entity list, kept for debug output.
    pub entities: Vec<DiagramInput>,
    pub filtered_seooc_count: usize,
    pub filtered_component_count: usize,
    pub filtered_unit_count: usize,
}

impl DiagramArchitecture {
    /// Index `entities` by stereotype and parent alias.
    ///
    /// `<<SEooC>>` go into `seooc_set`;
    /// `<<component>>` go into `comp_set`;
    /// `<<unit>>` go into `unit_set`.
    /// Duplicates (same [`EntityKey`]) are reported via `errors`.
    fn from_entities(entities: &[DiagramInput], errors: &mut Errors) -> Self {
        // Index by raw id for parent resolution (PlantUML nesting uses id, not alias).
        let mut id_index: BTreeMap<String, &DiagramInput> = BTreeMap::new();
        for e in entities {
            let key = e.id.to_lowercase();
            if let Some(prev) = id_index.insert(key.clone(), e) {
                errors.push(format!(
                    "Duplicate entity ID in PlantUML diagram (case-insensitive):\n\
                       ID : {key:?}\n\
                       IDs: {} and {}",
                    prev.id, e.id
                ));
            }
        }

        let seoocs: Vec<&DiagramInput> = entities.iter().filter(|e| e.is_seooc_package()).collect();
        let components: Vec<&DiagramInput> = entities.iter().filter(|e| e.is_component()).collect();
        let units: Vec<&DiagramInput> = entities.iter().filter(|e| e.is_unit()).collect();

        let filtered_seooc_count = seoocs.len();
        let filtered_component_count = components.len();
        let filtered_unit_count = units.len();

        let seooc_set = Self::build_set(&seoocs, &id_index, errors);
        let comp_set = Self::build_set(&components, &id_index, errors);
        let unit_set = Self::build_set(&units, &id_index, errors);

        Self {
            seooc_set,
            comp_set,
            unit_set,
            entities: entities.to_vec(),
            filtered_seooc_count,
            filtered_component_count,
            filtered_unit_count,
        }
    }

    fn build_set(
        items: &[&DiagramInput],
        id_index: &BTreeMap<String, &DiagramInput>,
        errors: &mut Errors,
    ) -> BTreeMap<EntityKey, DiagramInput> {
        let mut set = BTreeMap::new();
        for e in items {
            let alias = e.match_key();
            let parent_alias = match &e.parent_id {
                Some(pid) => match id_index.get(&pid.to_lowercase()) {
                    Some(p) => Some(p.match_key()),
                    None => {
                        errors.push(format!(
                            "Unresolved parent_id in PlantUML diagram:\n\
                               Entity ID : {}\n\
                               Parent ID : {}\n\
                               Action    : Fix the parent reference or add the missing parent entity",
                            e.id, pid
                        ));
                        None
                    }
                },
                None => None,
            };
            let key = (alias, parent_alias);
            if let Some(prev) = set.insert(key.clone(), (*e).clone()) {
                errors.push(format!(
                    "Duplicate entity in PlantUML diagram:\n\
                       Key: {:?}\n\
                       IDs: {} and {}",
                    key, prev.id, e.id
                ));
            }
        }
        set
    }
}

// ---------------------------------------------------------------------------
// Error accumulator
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct Errors {
    pub messages: Vec<String>,
    pub debug_output: String,
}

impl Errors {
    pub fn push(&mut self, message: String) {
        self.messages.push(message);
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
