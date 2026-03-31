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

use crate::models::{DiagramInput, DiagramInputs};
use component_fbs::component as fb_component;
use std::fs;

pub struct DiagramReader;

impl DiagramReader {
    /// Read all `Component` and `Package` entities from the given FlatBuffers
    /// binary files.
    pub fn read(paths: &[String]) -> Result<DiagramInputs, String> {
        let mut out = Vec::new();

        for path in paths {
            let data = fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;

            let graph = flatbuffers::root::<fb_component::ComponentGraph>(&data)
                .map_err(|e| format!("Failed to parse FlatBuffer {path}: {e}"))?;

            if let Some(entries) = graph.components() {
                for entry in entries.iter() {
                    if let Some(comp) = entry.value() {
                        match comp.comp_type() {
                            fb_component::ComponentType::Component
                            | fb_component::ComponentType::Package => {
                                out.push(DiagramInput {
                                    id: comp.id().unwrap_or_default().to_string(),
                                    alias: comp.alias().map(|s| s.to_string()),
                                    parent_id: comp.parent_id().map(|s| s.to_string()),
                                    stereotype: comp.stereotype().map(|s| s.to_string()),
                                });
                            }
                            // Other diagram entity types (Artifact, Database, etc.)
                            // are not relevant for architecture verification.
                            _ => {}
                        }
                    } else {
                        return Err(format!(
                            "FlatBuffer entry with key {:?} has null value in {path} (corrupted or truncated file)",
                            entry.key()
                        ));
                    }
                }
            }
        }

        Ok(DiagramInputs { entities: out })
    }
}
