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

use crate::{ErrorBuilder, ErrorCategory};

/// Composite key: `(design_name, full_parent_design_path)` (lowercased).
/// The parent is `None` for top-level entities.
pub type EntityKey = (String, Option<String>);

pub(super) fn entity_path(key: &EntityKey) -> String {
    let (design_name, parent_path) = key;
    match parent_path {
        Some(parent) => format!("{parent}.{design_name}"),
        None => design_name.clone(),
    }
}

/// Extract the target name from a Bazel label like `@//path/to/package:target`
/// -> `"target"`. Returns the full label unchanged if it contains no colon.
/// Returns `Err` if the extracted name is empty.
pub(super) fn label_short_name(label: &str) -> Result<&str, String> {
    let name = label.rsplit_once(':').map(|(_, n)| n).unwrap_or(label);
    if name.is_empty() {
        return Err(ErrorBuilder::new(ErrorCategory::Design)
            .title(format!("Bazel label \"{label}\" does not define a target."))
            .field("bazel label", format!("\"{label}\""))
            .fix(format!(
                "add a target name after ':' in Bazel label \"{label}\""
            ))
            .build());
    }
    Ok(name)
}
