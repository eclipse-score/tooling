///////////////////////////////////////////////////////////////////////////////////
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
////////////////////////////////////////////////////////////////////////////////////
use log::{debug, error};
use serde::Serialize;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;

pub fn write_entity_tree(path: &Path, entity_tree: &str) {
    match write_entity_tree_inner(path, entity_tree) {
        Ok(()) => {
            debug!(
                "Wrote entity tree to {:?}\nEntity tree:\n{}",
                path,
                entity_tree.trim_end()
            );
        }
        Err(e) => {
            error!("Failed to write entity tree to {:?}: {}", path, e);
            debug!("Entity tree:\n{}", entity_tree.trim_end());
        }
    }
}

fn write_entity_tree_inner(path: &Path, entity_tree: &str) -> std::io::Result<()> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut file_out = BufWriter::new(file);

    file_out.write_all(entity_tree.as_bytes())?;
    file_out.flush()
}

pub fn write_debug_json<T, U>(
    output_path: &Path,
    types: &T,
    functions: &U,
) -> Result<(), Box<dyn std::error::Error>>
where
    T: Serialize,
    U: Serialize,
{
    let mut debug_json = serde_json::Map::new();
    debug_json.insert("types".to_owned(), serde_json::to_value(types)?);
    debug_json.insert("functions".to_owned(), serde_json::to_value(functions)?);

    let output_json = serde_json::to_string_pretty(&debug_json)?;
    fs::write(output_path, output_json)?;
    debug!("Wrote AST debug JSON to {:?}", output_path);

    Ok(())
}

pub fn write_fbs_output(output_path: &Path, buffer: &[u8]) -> Result<(), std::io::Error> {
    fs::write(output_path, buffer)?;
    debug!("Wrote FlatBuffer to {:?}", output_path);
    Ok(())
}
