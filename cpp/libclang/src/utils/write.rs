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
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

pub fn write_entity_tree(path: &PathBuf, entity_tree: &str) {
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

fn write_entity_tree_inner(path: &PathBuf, entity_tree: &str) -> std::io::Result<()> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut file_out = BufWriter::new(file);

    file_out.write_all(entity_tree.as_bytes())?;
    file_out.flush()
}
