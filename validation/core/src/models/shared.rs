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

//! Shared helper types used across the split validation models.

/// Composite key: `(canonical_alias, parent_alias)`. `parent_alias` is `None`
/// for top-level entities. Using the parent as part of the key means two
/// identically-named entities under different parents are treated as distinct.
pub type EntityKey = (String, Option<String>);

/// Extract the target name from a Bazel label like `@//path/to/package:target`
/// -> `"target"`. Returns the full label unchanged if it contains no colon.
/// Returns `Err` if the extracted name is empty.
pub(super) fn label_short_name(label: &str) -> Result<&str, String> {
    let name = label.rsplit_once(':').map(|(_, n)| n).unwrap_or(label);
    if name.is_empty() {
        return Err(format!("Empty target name extracted from label: {label:?}"));
    }
    Ok(name)
}
