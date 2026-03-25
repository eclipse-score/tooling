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

use log::debug;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn write_json_to_file<T: serde::Serialize>(
    data: &T,
    input_path: &Path,
    output_dir: &Path,
    suffix: &str,
) -> std::io::Result<PathBuf> {
    let file_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let json_path = output_dir.join(format!("{file_stem}.{suffix}.json"));

    let json_string = serde_json::to_string_pretty(data)?;

    let mut file = File::create(&json_path)?;
    file.write_all(json_string.as_bytes())?;

    debug!("JSON written to {}", json_path.display());
    Ok(json_path)
}

pub fn write_fbs_to_file(
    buffer: &[u8],
    path: &Path,
    output_dir: &Path,
) -> std::io::Result<PathBuf> {
    let file_stem = path
        .file_stem()
        .unwrap_or_else(|| std::ffi::OsStr::new("output"));
    let mut output_path = output_dir.join(file_stem);
    output_path.set_extension("fbs.bin");

    let mut file = File::create(&output_path)?;
    file.write_all(buffer)?;

    debug!("FlatBuffer written to {}", output_path.display());
    Ok(output_path)
}

pub fn write_placeholder_file(path: &Path, output_dir: &Path) -> std::io::Result<PathBuf> {
    let file_stem = path
        .file_stem()
        .unwrap_or_else(|| std::ffi::OsStr::new("output"));
    let mut output_path = output_dir.join(file_stem);
    output_path.set_extension("fbs.bin");

    // Create an empty file as a placeholder
    fs::File::create(&output_path)?;

    debug!("Placeholder file written to {}", output_path.display());
    Ok(output_path)
}
