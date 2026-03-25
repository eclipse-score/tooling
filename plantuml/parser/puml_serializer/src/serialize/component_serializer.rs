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
use flatbuffers::FlatBufferBuilder;
/// Serializes the resolved component graph into a FlatBuffer binary format
use std::collections::HashMap;

use component_fbs::component as fb;
use component_resolver::{ComponentType, LogicComponent};

pub struct ComponentSerializer;

impl ComponentSerializer {
    pub fn serialize(components: &HashMap<String, LogicComponent>, source_file: &str) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        // --------------------------
        // 1) build components
        // --------------------------
        let mut comps_map = Vec::with_capacity(components.len());

        for comp in components.values() {
            let mut relation_offsets = Vec::new();

            for r in &comp.relations {
                let target_offset = builder.create_string(&r.target);
                let annotation_offset = r.annotation.as_ref().map(|s| builder.create_string(s));
                let relation_type_offset = builder.create_string(&r.relation_type);

                let rel = fb::LogicRelation::create(
                    &mut builder,
                    &fb::LogicRelationArgs {
                        target: Some(target_offset),
                        annotation: annotation_offset,
                        relation_type: Some(relation_type_offset),
                    },
                );
                relation_offsets.push(rel);
            }

            let relations_vector_offset = builder.create_vector(&relation_offsets);

            // component
            let comp_id_offset = builder.create_string(&comp.id);
            let comp_name_offset = comp.name.as_ref().map(|s| builder.create_string(s));
            let comp_alias_offset = comp.alias.as_ref().map(|s| builder.create_string(s));
            let comp_parent_id_offset = comp.parent_id.as_ref().map(|s| builder.create_string(s));
            let comp_stereotype_offset = comp.stereotype.as_ref().map(|s| builder.create_string(s));

            let comp_offset = fb::LogicComponent::create(
                &mut builder,
                &fb::LogicComponentArgs {
                    id: Some(comp_id_offset),
                    name: comp_name_offset,
                    alias: comp_alias_offset,
                    parent_id: comp_parent_id_offset,
                    comp_type: Self::convert_type(comp.comp_type),
                    stereotype: comp_stereotype_offset,
                    relations: Some(relations_vector_offset),
                },
            );

            let key_offset = builder.create_string(&comp.id);
            let comp_map = fb::ComponentMap::create(
                &mut builder,
                &fb::ComponentMapArgs {
                    key: Some(key_offset),
                    value: Some(comp_offset),
                },
            );

            comps_map.push(comp_map);
        }

        // --------------------------
        // 2️) vector
        // --------------------------
        let comps_vec = builder.create_vector(&comps_map);

        // --------------------------
        // 3) root object
        // --------------------------
        let source_file_offset = builder.create_string(source_file);
        let root = fb::ComponentGraph::create(
            &mut builder,
            &fb::ComponentGraphArgs {
                components: Some(comps_vec),
                source_file: Some(source_file_offset),
            },
        );

        // --------------------------
        // 4) finish
        // --------------------------
        builder.finish(root, None);

        builder.finished_data().to_vec()
    }

    fn convert_type(t: ComponentType) -> fb::ComponentType {
        match t {
            ComponentType::Artifact => fb::ComponentType::Artifact,
            ComponentType::Card => fb::ComponentType::Card,
            ComponentType::Cloud => fb::ComponentType::Cloud,
            ComponentType::Component => fb::ComponentType::Component,
            ComponentType::Database => fb::ComponentType::Database,
            ComponentType::File => fb::ComponentType::File,
            ComponentType::Folder => fb::ComponentType::Folder,
            ComponentType::Frame => fb::ComponentType::Frame,
            ComponentType::Hexagon => fb::ComponentType::Hexagon,
            ComponentType::Interface => fb::ComponentType::Interface,
            ComponentType::Node => fb::ComponentType::Node,
            ComponentType::Package => fb::ComponentType::Package,
            ComponentType::Queue => fb::ComponentType::Queue,
            ComponentType::Rectangle => fb::ComponentType::Rectangle,
            ComponentType::Stack => fb::ComponentType::Stack,
            ComponentType::Storage => fb::ComponentType::Storage,
        }
    }
}
