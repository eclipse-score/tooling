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
mod label_markup;
mod log;
mod write_files;

pub use label_markup::{
    decode_newline_escapes, normalize_identity_label, strip_style_markup, style_markup_tag_length,
};
pub use log::LogLevel;
pub use write_files::{write_fbs_to_file, write_json_to_file, write_placeholder_file};
