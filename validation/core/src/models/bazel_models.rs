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

use super::shared::label_short_name;
use super::{EntityKey, Errors};

// ---------------------------------------------------------------------------
/// Bazel architecture JSON model produced by the dependable element rule.
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
    /// for child-suppression - preventing a top-level component from being
    /// silently treated as nested just because another target in a different
    /// package shares the same short name.
    pub fn to_bazel_architecture(&self, errors: &mut Errors) -> BazelArchitecture {
        let mut seooc_set = BTreeMap::new();
        let mut comp_set = BTreeMap::new();
        let mut unit_set = BTreeMap::new();

        let child_labels: BTreeSet<String> = self
            .components
            .values()
            .flat_map(|entry| entry.components.iter())
            .map(|label| label.to_lowercase())
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
                // Top-level entries are dependable elements (SEooC).
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

            for unit_label in &entry.units {
                let unit_key = match label_short_name(unit_label) {
                    Ok(name) => name.to_lowercase(),
                    Err(msg) => {
                        errors.push(msg);
                        continue;
                    }
                };
                let key = (unit_key, Some(comp_key.clone()));
                if let Some(prev) = unit_set.insert(key.clone(), unit_label.clone()) {
                    errors.push(format!(
                        "Duplicate unit key in Bazel build graph:\n\
                           Key   : {:?}\n\
                           Labels: {} and {}",
                        key, prev, unit_label
                    ));
                }
            }

            for component_label in &entry.components {
                let component_key = match label_short_name(component_label) {
                    Ok(name) => name.to_lowercase(),
                    Err(msg) => {
                        errors.push(msg);
                        continue;
                    }
                };
                let key = (component_key, Some(comp_key.clone()));
                if let Some(prev) = comp_set.insert(key.clone(), component_label.clone()) {
                    errors.push(format!(
                        "Duplicate component key in Bazel build graph:\n\
                           Key   : {:?}\n\
                           Labels: {} and {}",
                        key, prev, component_label
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

/// JSON payload for a single architecture entry, including nested components
/// and implementation units.
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
#[derive(Clone)]
pub struct BazelArchitecture {
    /// Top-level dependable elements (`<<SEooC>>`), keyed with `parent = None`.
    pub seooc_set: BTreeMap<EntityKey, String>,
    /// Nested components (`<<component>>`), keyed with `parent = Some(..)`.
    pub comp_set: BTreeMap<EntityKey, String>,
    /// Nested units (`<<unit>>`), keyed with the enclosing component alias.
    pub unit_set: BTreeMap<EntityKey, String>,
}
