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

use log::warn;
use std::collections::BTreeMap;

use class_diagram::SimpleEntity;

pub trait EntityMapExt {
    fn insert_or_merge_type(&mut self, type_name: String, entity: SimpleEntity);
}

impl EntityMapExt for BTreeMap<String, SimpleEntity> {
    fn insert_or_merge_type(&mut self, type_name: String, entity: SimpleEntity) {
        match self.get_mut(&type_name) {
            Some(existing) => merge_simple_entity(existing, entity),
            None => {
                self.insert(type_name, entity);
            }
        }
    }
}

fn merge_simple_entity(existing: &mut SimpleEntity, incoming: SimpleEntity) {
    if existing.name != incoming.name {
        warn!(
            "conflicting entity names while merging '{}': keeping {:?}, dropping {:?}",
            existing.id, existing.name, incoming.name
        );
    }
    if existing.enclosing_namespace_id != incoming.enclosing_namespace_id {
        warn!(
            "conflicting enclosing namespaces while merging '{}': keeping {:?}, dropping {:?}",
            existing.id, existing.enclosing_namespace_id, incoming.enclosing_namespace_id
        );
    }
    if existing.template_parameters.is_none() {
        existing.template_parameters = incoming.template_parameters.clone();
    }
    if existing.source_location == Default::default() {
        existing.source_location = incoming.source_location.clone();
    }

    if existing.entity_type != incoming.entity_type {
        warn!(
            "conflicting entity types while merging '{}': keeping {:?}, dropping {:?}",
            existing.id, existing.entity_type, incoming.entity_type
        );
    }

    extend_unique(&mut existing.stereotypes, incoming.stereotypes);
    extend_unique(&mut existing.type_aliases, incoming.type_aliases);
    extend_unique(&mut existing.variables, incoming.variables);
    extend_unique(&mut existing.methods, incoming.methods);
    extend_unique(&mut existing.enum_literals, incoming.enum_literals);
    extend_unique(&mut existing.relationships, incoming.relationships);
}

fn extend_unique<T: PartialEq>(existing: &mut Vec<T>, incoming: Vec<T>) {
    for item in incoming {
        if !existing.contains(&item) {
            existing.push(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EntityMapExt;
    use class_diagram::{EntityType, Method, SimpleEntity, SourceLocation, Visibility};
    use std::collections::BTreeMap;

    #[test]
    fn insert_or_merge_type_preserves_members_when_later_entity_is_sparser() {
        let mut types = BTreeMap::new();
        types.insert_or_merge_type(
            "util::Widget".to_string(),
            SimpleEntity {
                id: "util::Widget".to_string(),
                name: "Widget".to_string(),
                enclosing_namespace_id: Some("util".to_string()),
                entity_type: EntityType::Class,
                methods: vec![Method {
                    name: "compute".to_string(),
                    return_type: Some("int".to_string()),
                    visibility: Visibility::Public,
                    source_location: SourceLocation::new("first.h", 10),
                    ..Default::default()
                }],
                source_location: SourceLocation::new("first.h", 1),
                ..Default::default()
            },
        );

        types.insert_or_merge_type(
            "util::Widget".to_string(),
            SimpleEntity {
                id: "util::Widget".to_string(),
                name: "Widget".to_string(),
                enclosing_namespace_id: Some("util".to_string()),
                entity_type: EntityType::Class,
                stereotypes: vec!["header-only".to_string()],
                source_location: SourceLocation::new("second.h", 1),
                ..Default::default()
            },
        );

        let widget = types.get("util::Widget").expect("merged type should exist");

        assert_eq!(widget.methods.len(), 1);
        assert_eq!(widget.methods[0].name, "compute");
        assert_eq!(
            widget.methods[0].source_location,
            SourceLocation::new("first.h", 10)
        );
        assert_eq!(widget.stereotypes, vec!["header-only"]);
        assert_eq!(widget.source_location, SourceLocation::new("first.h", 1));
        assert_eq!(widget.enclosing_namespace_id.as_deref(), Some("util"));
    }

    #[test]
    fn insert_or_merge_type_adds_missing_details_when_later_entity_is_richer() {
        let mut types = BTreeMap::new();
        types.insert_or_merge_type(
            "util::Widget".to_string(),
            SimpleEntity {
                id: "util::Widget".to_string(),
                name: "Widget".to_string(),
                enclosing_namespace_id: Some("util".to_string()),
                entity_type: EntityType::Class,
                ..Default::default()
            },
        );

        types.insert_or_merge_type(
            "util::Widget".to_string(),
            SimpleEntity {
                id: "util::Widget".to_string(),
                name: "Widget".to_string(),
                enclosing_namespace_id: Some("util".to_string()),
                entity_type: EntityType::Class,
                methods: vec![Method {
                    name: "compute".to_string(),
                    return_type: Some("int".to_string()),
                    visibility: Visibility::Public,
                    source_location: SourceLocation::new("second.h", 10),
                    ..Default::default()
                }],
                stereotypes: vec!["header-only".to_string()],
                source_location: SourceLocation::new("second.h", 1),
                ..Default::default()
            },
        );

        let widget = types.get("util::Widget").expect("merged type should exist");

        assert_eq!(widget.methods.len(), 1);
        assert_eq!(widget.methods[0].name, "compute");
        assert_eq!(
            widget.methods[0].source_location,
            SourceLocation::new("second.h", 10)
        );
        assert_eq!(widget.stereotypes, vec!["header-only"]);
        assert_eq!(widget.source_location, SourceLocation::new("second.h", 1));
        assert_eq!(widget.enclosing_namespace_id.as_deref(), Some("util"));
    }
}
