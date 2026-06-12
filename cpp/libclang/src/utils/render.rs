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
use clang::{Entity, EntityKind, EntityVisitResult};

pub fn render_entity_tree(entity: &Entity, level: usize) -> String {
    let mut output = String::new();
    append_entity_tree(entity, level, &mut output);
    output
}

fn append_entity_tree(entity: &Entity, level: usize, output: &mut String) {
    output.push_str(&format_entity_line(entity, level));
    output.push('\n');

    entity.visit_children(|child, _parent| {
        append_entity_tree(&child, level + 1, output);
        EntityVisitResult::Continue
    });
}

fn format_entity_line(entity: &Entity, level: usize) -> String {
    let indent = "  ".repeat(level);
    let kind = entity.get_kind();
    let spelling = match kind {
        EntityKind::AccessSpecifier => entity
            .get_accessibility()
            .map(|a| format!("{:?}", a).to_lowercase())
            .unwrap_or_default(),
        _ => entity.get_name().unwrap_or_default(),
    };

    format!("{}{:?} | {}", indent, kind, spelling)
}
