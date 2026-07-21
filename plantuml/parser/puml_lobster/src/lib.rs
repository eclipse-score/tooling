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
//! For component diagrams only [`ComponentType::Interface`] elements are emitted.
//! For class diagrams, [`EntityType::Interface`] entities emit one item per method;
//! all other entity types emit one item per entity.

use class_diagram::{ClassDiagram, EntityType};
use component_diagram::{ComponentType, LogicComponent};
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
        .filter(|element| element.element_type == ComponentType::Interface)
        .map(|element| build_lobster_item(&element.id, source_path, None, "Interface"))
        .collect();

    lobster_document_from_items(items)
}

/// Convert an in-memory resolved class model to a `lobster-imp-trace`
/// JSON [`Value`].
///
/// For [`EntityType::Interface`] entities every method becomes its own lobster
/// item with id `{entity.id}.{method.name}` and kind `"Method"`.  All other
/// entity types are emitted as a single item (one per entity).
fn class_model_to_lobster(model: &ClassDiagram) -> Value {
    let items: Vec<Value> = model
        .entities
        .iter()
        .flat_map(|entity| {
            let source_file = entity.source_location.file.as_ref();
            let line = entity.source_location.line;
            let source_line = (line != 0).then_some(line);

            if entity.entity_type == EntityType::Interface {
                entity
                    .methods
                    .iter()
                    .map(|method| {
                        let method_id = format!("{}.{}", entity.id, method.name);
                        build_lobster_item(&method_id, source_file, source_line, "Method")
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![build_lobster_item(
                    &entity.id,
                    source_file,
                    source_line,
                    map_entity_type_to_kind(entity.entity_type),
                )]
            }
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
    }
}

/// Write a `lobster-imp-trace` JSON file derived from `model` into `output_dir`.
///
/// The output filename is `<stem>.lobster` where `<stem>` is the file stem of
/// `input_path` (the original `.puml` source file).
pub fn write_lobster_to_file(
    model: LobsterModel<'_>,
    input_path: &Path,
    source_path: &str,
    output_dir: &Path,
) -> io::Result<PathBuf> {
    let lobster = match model {
        LobsterModel::Component(component_model) => {
            comp_model_to_lobster(component_model, source_path)
        }
        LobsterModel::Class(class_model) => class_model_to_lobster(class_model),
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

#[cfg(test)]
mod tests {
    use super::*;
    use class_diagram::SimpleEntity;
    use component_diagram::SourceLocation;

    fn interface_component(id: &str) -> LogicComponent {
        LogicComponent {
            id: id.to_string(),
            name: None,
            alias: None,
            parent_id: None,
            element_type: ComponentType::Interface,
            stereotype: None,
            relations: Vec::new(),
            source_location: SourceLocation::new("test.puml", 0),
        }
    }

    fn unique_tmp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "puml_lobster_{}_{}_{}",
            tag,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Remove a test's temp dir, surfacing (rather than silently swallowing)
    /// any cleanup failure so a locked/undeletable dir doesn't go unnoticed.
    fn cleanup_tmp_dir(dir: &Path) {
        if let Err(e) = fs::remove_dir_all(dir) {
            eprintln!(
                "warning: failed to remove temp dir {}: {}",
                dir.display(),
                e
            );
        }
    }

    #[test]
    fn write_lobster_to_file_embeds_source_path_for_component_model() {
        let mut model = HashMap::new();
        model.insert("pkg.Iface".to_string(), interface_component("pkg.Iface"));
        let dir = unique_tmp_dir("component");
        let input = Path::new("some/dir/component.puml");

        let output = write_lobster_to_file(
            LobsterModel::Component(&model),
            input,
            "pkg/component.puml",
            &dir,
        )
        .expect("lobster file must be written");

        let doc: Value = serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        let items = doc["data"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["location"]["file"], "pkg/component.puml");

        cleanup_tmp_dir(&dir);
    }

    #[test]
    fn write_lobster_to_file_embeds_source_path_for_class_model() {
        let entity = SimpleEntity {
            id: "pkg.Foo".to_string(),
            name: "Foo".to_string(),
            source_location: SourceLocation::new("pkg/classes.puml", 0),
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![entity],
        };
        let dir = unique_tmp_dir("class");
        let input = Path::new("some/dir/classes.puml");

        let output =
            write_lobster_to_file(LobsterModel::Class(&model), input, "pkg/classes.puml", &dir)
                .expect("lobster file must be written");

        let doc: Value = serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        let items = doc["data"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["location"]["file"], "pkg/classes.puml");

        cleanup_tmp_dir(&dir);
    }

    /// Each entity's own `source_location.file` is embedded in its lobster
    /// item, independent of the `source_path` parameter passed to the writer
    /// (which only applies to component models).
    #[test]
    fn write_lobster_to_file_class_entity_uses_own_source_location() {
        let entity = SimpleEntity {
            id: "pkg.Foo".to_string(),
            name: "Foo".to_string(),
            source_location: SourceLocation::new("pkg/entity_specific.puml", 0),
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![entity],
        };
        let dir = unique_tmp_dir("class_override");
        let input = Path::new("some/dir/classes.puml");

        let output =
            write_lobster_to_file(LobsterModel::Class(&model), input, "pkg/classes.puml", &dir)
                .expect("lobster file must be written");

        let doc: Value = serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(
            doc["data"].as_array().unwrap()[0]["location"]["file"],
            "pkg/entity_specific.puml"
        );

        cleanup_tmp_dir(&dir);
    }

    #[test]
    fn write_lobster_to_file_output_filename_is_input_stem() {
        let dir = unique_tmp_dir("filename");
        let input = Path::new("some/dir/my_diagram.puml");

        let output = write_lobster_to_file(LobsterModel::Empty, input, "pkg/my_diagram.puml", &dir)
            .expect("lobster file must be written");

        assert_eq!(
            output.file_name().and_then(|n| n.to_str()),
            Some("my_diagram.lobster")
        );

        cleanup_tmp_dir(&dir);
    }
}
