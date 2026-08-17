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

//! Conversion helpers between libclang locations and the shared metamodel.

use clang::Entity;
use class_diagram::SourceLocation;

pub(crate) fn parse_source_location(entity: &Entity) -> SourceLocation {
    let Some(location) = entity.get_location() else {
        return SourceLocation::default();
    };

    let file_location = location.get_file_location();
    let source_file = file_location
        .file
        .map(|file| file.get_path().to_string_lossy().to_string());
    SourceLocation::new(source_file.unwrap_or_default(), file_location.line)
}
