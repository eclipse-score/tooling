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

use class_diagram::{ClassDiagram, EntityType};
use puml_resolver::{ComponentType, LogicComponent};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub enum LobsterModel<'a> {
    Component(&'a HashMap<String, LogicComponent>),
    Class(&'a ClassDiagram),
    Empty,
}

/// Convert an in-memory resolved component model to a `lobster-imp-trace`
/// JSON [`Value`].
///
/// `source_path` is embedded in the `location.file` field of every emitted
/// item so that LOBSTER can trace items back to their source diagram.
fn comp_model_to_lobster(model: &HashMap<String, LogicComponent>, source_path: &str) -> Value {
    let items: Vec<Value> = model
        .values()
        .filter(|c| c.comp_type == ComponentType::Interface)
        .map(|c| build_lobster_item(&c.id, source_path, None, "Interface"))
        .collect();

    lobster_document_from_items(items)
}

/// Convert an in-memory resolved class model to a `lobster-imp-trace`
/// JSON [`Value`].
///
/// Every class entity becomes one lobster item. If an entity carries explicit
/// source location metadata that is used; otherwise `source_path` is used and
/// the line is emitted as `null` because LOBSTER does not accept `0`.
fn class_model_to_lobster(model: &ClassDiagram, source_path: &str) -> Value {
    let items: Vec<Value> = model
        .entities
        .iter()
        .map(|entity| {
            let source_file = entity.source_file.as_deref().unwrap_or(source_path);

            build_lobster_item(
                &entity.id,
                source_file,
                entity.source_line,
                map_entity_type_to_kind(entity.entity_type),
            )
        })
        .collect();

    lobster_document_from_items(items)
}

fn lobster_document_from_items(mut items: Vec<Value>) -> Value {
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

fn empty_lobster_document() -> Value {
    lobster_document_from_items(Vec::new())
}

fn build_lobster_item(
    name: &str,
    source_file: &str,
    source_line: Option<u32>,
    kind: &str,
) -> Value {
    json!({
        "tag": format!("req {}", name),
        "location": {
            "kind": "file",
            "file": source_file,
            "line": source_line,
            "column": null,
        },
        "name": name,
        "messages": [],
        "just_up": [],
        "just_down": [],
        "just_global": [],
        "refs": [],
        "language": "Architecture",
        "kind": kind,
    })
}

fn map_entity_type_to_kind(entity_type: EntityType) -> &'static str {
    match entity_type {
        EntityType::Class => "Class",
        EntityType::Struct => "Struct",
        EntityType::Interface => "Interface",
        EntityType::Enum => "Enum",
        EntityType::AbstractClass => "AbstractClass",
        EntityType::Annotation => "Annotation",
    }
}

/// Write a `lobster-imp-trace` JSON file derived from `model` into `output_dir`.
///
/// The output filename is `<stem>.lobster` where `<stem>` is the file stem of
/// `input_path` (the original `.puml` source file).
pub fn write_lobster_to_file(
    model: LobsterModel<'_>,
    input_path: &Path,
    output_dir: &Path,
) -> io::Result<PathBuf> {
    let lobster = match model {
        LobsterModel::Component(component_model) => {
            let source_str = input_path.to_string_lossy().into_owned();
            comp_model_to_lobster(component_model, &source_str)
        }
        LobsterModel::Class(class_model) => {
            let source_str = input_path.to_string_lossy().into_owned();
            class_model_to_lobster(class_model, &source_str)
        }
        LobsterModel::Empty => empty_lobster_document(),
    };

    write_lobster_value_to_file(&lobster, input_path, output_dir)
}

fn write_lobster_value_to_file(
    lobster: &Value,
    input_path: &Path,
    output_dir: &Path,
) -> io::Result<PathBuf> {
    let file_stem = input_path
        .file_stem()
        .unwrap_or_else(|| OsStr::new("output"));

    let output_path = output_dir.join(file_stem).with_extension("lobster");

    let content = serde_json::to_string_pretty(&lobster)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    fs::write(&output_path, content + "\n")?;
    Ok(output_path)
}
