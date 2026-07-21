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
/// Serializes the resolved component graph into a FlatBuffer binary format
use flatbuffers::FlatBufferBuilder;
use std::collections::HashMap;

use component_diagram::{ComponentRelationType, ComponentType, EndpointRole, LogicComponent};
use component_fbs::component as fb;

pub struct ComponentSerializer;

impl ComponentSerializer {
    pub fn serialize(elements: &HashMap<String, LogicComponent>, _diagram_file: &str) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();
        let source_file = elements
            .values()
            .next()
            .map(|e| builder.create_string(e.source_location.file.as_ref()));

        // --------------------------
        // 1) build components
        // --------------------------
        let mut comps_map = Vec::with_capacity(elements.len());

        for element in elements.values() {
            let mut relation_offsets = Vec::new();

            for r in &element.relations {
                let target_offset = builder.create_string(&r.target);
                let annotation_offset = r.annotation.as_ref().map(|s| builder.create_string(s));
                let relation_source_location = fb::SourceLocation::create(
                    &mut builder,
                    &fb::SourceLocationArgs {
                        file: source_file,
                        line: r.source_location.line,
                    },
                );

                let rel = fb::LogicRelation::create(
                    &mut builder,
                    &fb::LogicRelationArgs {
                        target: Some(target_offset),
                        annotation: annotation_offset,
                        relation_type: Self::convert_relation_type(r.relation_type),
                        source_role: Self::convert_endpoint_role(r.source_role),
                        source_location: Some(relation_source_location),
                    },
                );
                relation_offsets.push(rel);
            }

            let relations_vector_offset = builder.create_vector(&relation_offsets);

            // component
            let comp_id_offset = builder.create_string(&element.id);
            let comp_name_offset = element.name.as_ref().map(|s| builder.create_string(s));
            let comp_alias_offset = element.alias.as_ref().map(|s| builder.create_string(s));
            let comp_parent_id_offset =
                element.parent_id.as_ref().map(|s| builder.create_string(s));
            let comp_stereotype_offset = element
                .stereotype
                .as_ref()
                .map(|s| builder.create_string(s));
            let comp_source_location = fb::SourceLocation::create(
                &mut builder,
                &fb::SourceLocationArgs {
                    file: source_file,
                    line: element.source_location.line,
                },
            );

            let comp_offset = fb::LogicComponent::create(
                &mut builder,
                &fb::LogicComponentArgs {
                    id: Some(comp_id_offset),
                    name: comp_name_offset,
                    alias: comp_alias_offset,
                    parent_id: comp_parent_id_offset,
                    comp_type: Self::convert_type(element.element_type),
                    stereotype: comp_stereotype_offset,
                    relations: Some(relations_vector_offset),
                    source_location: Some(comp_source_location),
                },
            );

            let key_offset = builder.create_string(&element.id);
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
        let root = fb::ComponentGraph::create(
            &mut builder,
            &fb::ComponentGraphArgs {
                components: Some(comps_vec),
            },
        );

        // --------------------------
        // 4) finish
        // --------------------------
        builder.finish(root, Some("COMD"));

        builder.finished_data().to_vec()
    }

    fn convert_type(t: ComponentType) -> fb::ComponentType {
        match t {
            ComponentType::Artifact => fb::ComponentType::Artifact,
            ComponentType::Actor => fb::ComponentType::Actor,
            ComponentType::Agent => fb::ComponentType::Agent,
            ComponentType::Boundary => fb::ComponentType::Boundary,
            ComponentType::Card => fb::ComponentType::Card,
            ComponentType::Cloud => fb::ComponentType::Cloud,
            ComponentType::Component => fb::ComponentType::Component,
            ComponentType::Control => fb::ComponentType::Control,
            ComponentType::Database => fb::ComponentType::Database,
            ComponentType::Entity => fb::ComponentType::Entity,
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
            ComponentType::Usecase => fb::ComponentType::Usecase,
        }
    }

    fn convert_relation_type(relation_type: ComponentRelationType) -> fb::ComponentRelationType {
        match relation_type {
            ComponentRelationType::Association => fb::ComponentRelationType::Association,
            ComponentRelationType::Dependency => fb::ComponentRelationType::Dependency,
            ComponentRelationType::InterfaceBinding => fb::ComponentRelationType::InterfaceBinding,
        }
    }

    fn convert_endpoint_role(source_role: EndpointRole) -> fb::EndpointRole {
        match source_role {
            EndpointRole::None => fb::EndpointRole::None,
            EndpointRole::Provided => fb::EndpointRole::Provided,
            EndpointRole::Required => fb::EndpointRole::Required,
        }
    }
}
