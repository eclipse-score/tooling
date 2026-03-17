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

//! Converts the resolved PlantUML logical model into a `lobster-imp-trace`
//! JSON file compatible with the LOBSTER traceability toolchain.
//!
//! Only [`ComponentType::Interface`] elements are emitted

use puml_resolver::{ComponentType, LogicComponent};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Convert an in-memory resolved component model to a `lobster-imp-trace`
/// JSON [`Value`].
///
/// `source_path` is embedded in the `location.file` field of every emitted
/// item so that LOBSTER can trace items back to their source diagram.
pub fn model_to_lobster(model: &HashMap<String, LogicComponent>, source_path: &str) -> Value {
    let mut items: Vec<Value> = model
        .values()
        .filter(|c| c.comp_type == ComponentType::Interface)
        .map(|c| {
            json!({
                "tag": format!("req {}", c.id),
                "location": {
                    "kind": "file",
                    "file": source_path,
                    "line": 1,
                    "column": null,
                },
                "name": c.id,
                "messages": [],
                "just_up": [],
                "just_down": [],
                "just_global": [],
                "refs": [],
                "language": "Architecture",
                "kind": "Interface",
            })
        })
        .collect();

    // Sort by tag for deterministic output
    items.sort_by(|a, b| {
        a["tag"]
            .as_str()
            .unwrap_or("")
            .cmp(b["tag"].as_str().unwrap_or(""))
    });

    json!({
        "schema": "lobster-imp-trace",
        "version": 3,
        "generator": "puml_lobster",
        "data": items,
    })
}

/// Write a `lobster-imp-trace` JSON file derived from `model` into `output_dir`.
///
/// The output filename is `<stem>.lobster` where `<stem>` is the file stem of
/// `input_path` (the original `.puml` source file).
pub fn write_lobster_to_file(
    model: &HashMap<String, LogicComponent>,
    input_path: &Path,
    output_dir: &Path,
) -> io::Result<PathBuf> {
    let file_stem = input_path
        .file_stem()
        .unwrap_or_else(|| OsStr::new("output"));

    let output_path = output_dir.join(file_stem).with_extension("lobster");

    let source_str = input_path.to_string_lossy().into_owned();
    let lobster = model_to_lobster(model, &source_str);

    let content = serde_json::to_string_pretty(&lobster)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    fs::write(&output_path, content + "\n")?;
    Ok(output_path)
}
