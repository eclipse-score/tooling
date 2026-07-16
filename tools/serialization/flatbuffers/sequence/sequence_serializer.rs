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
use sequence_fbs::sequence_metamodel as fb;
use sequence_logic::{ConditionType, Event, SequenceNode, SequenceTree};

pub struct SequenceSerializer;

impl SequenceSerializer {
    pub fn serialize(diagram: &SequenceTree) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        let name_offset = diagram.name.as_deref().map(|n| builder.create_string(n));

        let node_offsets: Vec<_> = diagram
            .root_interactions
            .iter()
            .map(|node| Self::serialize_node(&mut builder, node))
            .collect();
        let nodes_offset = builder.create_vector(&node_offsets);

        let root = fb::SequenceDiagram::create(
            &mut builder,
            &fb::SequenceDiagramArgs {
                name: name_offset,
                root_interactions: Some(nodes_offset),
            },
        );

        builder.finish(root, Some("SEQD"));
        builder.finished_data().to_vec()
    }

    fn serialize_node<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        node: &SequenceNode,
    ) -> flatbuffers::WIPOffset<fb::SequenceNode<'a>> {
        // Recursively serialize child nodes first (depth-first).
        let branch_offsets: Vec<_> = node
            .branches_node
            .iter()
            .map(|child| Self::serialize_node(builder, child))
            .collect();
        let branches_offset = builder.create_vector(&branch_offsets);
        let location_file_offset = builder.create_string(node.source_location.file.as_ref());
        let source_location = fb::SourceLocation::create(
            builder,
            &fb::SourceLocationArgs {
                file: Some(location_file_offset),
                line: node.source_location.line,
            },
        );

        // Serialize the event union.
        let (event_type, event_offset) = Self::serialize_event(builder, &node.event);

        fb::SequenceNode::create(
            builder,
            &fb::SequenceNodeArgs {
                event_type,
                event: Some(event_offset),
                source_location: Some(source_location),
                branches_node: Some(branches_offset),
            },
        )
    }

    fn serialize_event(
        builder: &mut FlatBufferBuilder<'_>,
        event: &Event,
    ) -> (
        fb::Event,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) {
        match event {
            Event::Interaction(interaction) => {
                let caller = builder.create_string(&interaction.caller);
                let callee = builder.create_string(&interaction.callee);
                let method = builder.create_string(&interaction.method);
                let offset = fb::Interaction::create(
                    builder,
                    &fb::InteractionArgs {
                        caller: Some(caller),
                        callee: Some(callee),
                        method: Some(method),
                    },
                );
                (fb::Event::Interaction, offset.as_union_value())
            }
            Event::Return(ret) => {
                let caller = builder.create_string(&ret.caller);
                let callee = builder.create_string(&ret.callee);
                let return_content = builder.create_string(&ret.return_content);
                let offset = fb::Return::create(
                    builder,
                    &fb::ReturnArgs {
                        caller: Some(caller),
                        callee: Some(callee),
                        return_content: Some(return_content),
                    },
                );
                (fb::Event::Return, offset.as_union_value())
            }
            Event::Condition(cond) => {
                let condition_value = builder.create_string(&cond.condition_value);
                let offset = fb::Condition::create(
                    builder,
                    &fb::ConditionArgs {
                        condition_type: Self::map_condition_type(cond.condition_type.clone()),
                        condition_value: Some(condition_value),
                    },
                );
                (fb::Event::Condition, offset.as_union_value())
            }
        }
    }

    fn map_condition_type(ct: ConditionType) -> fb::ConditionType {
        match ct {
            ConditionType::Opt => fb::ConditionType::Opt,
            ConditionType::Alt => fb::ConditionType::Alt,
            ConditionType::Loop => fb::ConditionType::Loop,
            ConditionType::Par => fb::ConditionType::Par,
            ConditionType::Par2 => fb::ConditionType::Par2,
            ConditionType::Break => fb::ConditionType::Break,
            ConditionType::Critical => fb::ConditionType::Critical,
            ConditionType::Else => fb::ConditionType::Else,
            ConditionType::Also => fb::ConditionType::Also,
            ConditionType::End => fb::ConditionType::End,
            ConditionType::Group => fb::ConditionType::Group,
        }
    }
}
