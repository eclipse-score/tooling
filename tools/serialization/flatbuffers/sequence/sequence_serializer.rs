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
use sequence_logic::{
    Block as LogicBlock, Branch as LogicBranch, BranchCase as LogicBranchCase,
    EarlyExit as LogicEarlyExit, Interaction as LogicInteraction, LifecycleAction,
    Loop as LogicLoop, Node as LogicNode, Parallel as LogicParallel,
    ParallelBranch as LogicParallelBranch, ParticipantLifecycle, ParticipantType, Reference,
    SequenceParticipant, SequenceTree, SourceLocation,
};

pub struct SequenceSerializer;

fn map_participant_type(value: &ParticipantType) -> fb::ParticipantType {
    match value {
        ParticipantType::Participant => fb::ParticipantType::Participant,
        ParticipantType::Actor => fb::ParticipantType::Actor,
        ParticipantType::Boundary => fb::ParticipantType::Boundary,
        ParticipantType::Control => fb::ParticipantType::Control,
        ParticipantType::Entity => fb::ParticipantType::Entity,
        ParticipantType::Queue => fb::ParticipantType::Queue,
        ParticipantType::Database => fb::ParticipantType::Database,
        ParticipantType::Collections => fb::ParticipantType::Collections,
    }
}

impl SequenceSerializer {
    pub fn serialize(diagram: &SequenceTree) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        let name_offset = diagram.name.as_deref().map(|n| builder.create_string(n));

        let participant_offsets: Vec<_> = diagram
            .participants
            .iter()
            .map(|participant| Self::serialize_participant(&mut builder, participant))
            .collect();
        let participants_offset = builder.create_vector(&participant_offsets);

        let root_offset = Self::serialize_block(&mut builder, &diagram.root);

        let root = fb::SequenceDiagram::create(
            &mut builder,
            &fb::SequenceDiagramArgs {
                name: name_offset,
                participants: Some(participants_offset),
                root: Some(root_offset),
            },
        );

        builder.finish(root, Some("SEQD"));
        builder.finished_data().to_vec()
    }

    fn serialize_participant<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        participant: &SequenceParticipant,
    ) -> flatbuffers::WIPOffset<fb::SequenceParticipant<'a>> {
        let display_name = builder.create_string(&participant.display_name);
        let alias = participant
            .alias
            .as_deref()
            .map(|value| builder.create_string(value));
        let stereotype = participant
            .stereotype
            .as_deref()
            .map(|value| builder.create_string(value));
        let location_file = builder.create_string(participant.source_location.file.as_ref());
        let source_location = fb::SourceLocation::create(
            builder,
            &fb::SourceLocationArgs {
                file: Some(location_file),
                line: participant.source_location.line,
            },
        );

        fb::SequenceParticipant::create(
            builder,
            &fb::SequenceParticipantArgs {
                display_name: Some(display_name),
                alias,
                participant_type: map_participant_type(&participant.participant_type),
                source_location: Some(source_location),
                stereotype,
            },
        )
    }

    fn serialize_block<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        block: &LogicBlock,
    ) -> flatbuffers::WIPOffset<fb::Block<'a>> {
        let item_offsets: Vec<_> = block
            .items
            .iter()
            .map(|node| Self::serialize_node_item(builder, node))
            .collect();
        let items = builder.create_vector(&item_offsets);

        fb::Block::create(builder, &fb::BlockArgs { items: Some(items) })
    }

    fn serialize_node_item<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        node: &LogicNode,
    ) -> flatbuffers::WIPOffset<fb::NodeItem<'a>> {
        let (node_type, node) = match node {
            LogicNode::Interaction(interaction) => (
                fb::Node::Interaction,
                Self::serialize_interaction(builder, interaction).as_union_value(),
            ),
            LogicNode::Branch(branch) => (
                fb::Node::Branch,
                Self::serialize_branch(builder, branch).as_union_value(),
            ),
            LogicNode::Loop(loop_node) => (
                fb::Node::Loop,
                Self::serialize_loop(builder, loop_node).as_union_value(),
            ),
            LogicNode::Parallel(parallel) => (
                fb::Node::Parallel,
                Self::serialize_parallel(builder, parallel).as_union_value(),
            ),
            LogicNode::EarlyExit(early_exit) => (
                fb::Node::EarlyExit,
                Self::serialize_early_exit(builder, early_exit).as_union_value(),
            ),
            LogicNode::Lifecycle(lifecycle) => (
                fb::Node::ParticipantLifecycle,
                Self::serialize_lifecycle(builder, lifecycle).as_union_value(),
            ),
            LogicNode::Reference(reference) => (
                fb::Node::Reference,
                Self::serialize_reference(builder, reference).as_union_value(),
            ),
        };

        fb::NodeItem::create(
            builder,
            &fb::NodeItemArgs {
                node_type,
                node: Some(node),
            },
        )
    }

    fn serialize_interaction<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        interaction: &LogicInteraction,
    ) -> flatbuffers::WIPOffset<fb::Interaction<'a>> {
        let sender = interaction
            .sender
            .as_deref()
            .map(|sender| builder.create_string(sender));
        let receiver = interaction
            .receiver
            .as_deref()
            .map(|receiver| builder.create_string(receiver));
        let message = interaction
            .message
            .as_deref()
            .map(|message| builder.create_string(message));
        let source_location = serialize_source_location(builder, &interaction.source_location);

        fb::Interaction::create(
            builder,
            &fb::InteractionArgs {
                sender,
                receiver: receiver,
                message,
                source_location: Some(source_location),
            },
        )
    }

    fn serialize_branch<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        branch: &LogicBranch,
    ) -> flatbuffers::WIPOffset<fb::Branch<'a>> {
        let case_offsets: Vec<_> = branch
            .cases
            .iter()
            .map(|case| Self::serialize_branch_case(builder, case))
            .collect();
        let cases = builder.create_vector(&case_offsets);

        fb::Branch::create(builder, &fb::BranchArgs { cases: Some(cases) })
    }

    fn serialize_branch_case<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        case: &LogicBranchCase,
    ) -> flatbuffers::WIPOffset<fb::BranchCase<'a>> {
        let condition = case
            .condition
            .as_deref()
            .map(|condition| builder.create_string(condition));
        let block = Self::serialize_block(builder, &case.block);
        let source_location = serialize_source_location(builder, &case.source_location);

        fb::BranchCase::create(
            builder,
            &fb::BranchCaseArgs {
                condition,
                block: Some(block),
                source_location: Some(source_location),
            },
        )
    }

    fn serialize_loop<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        loop_node: &LogicLoop,
    ) -> flatbuffers::WIPOffset<fb::Loop<'a>> {
        let condition = loop_node
            .condition
            .as_deref()
            .map(|condition| builder.create_string(condition));
        let block = Self::serialize_block(builder, &loop_node.block);
        let source_location = serialize_source_location(builder, &loop_node.source_location);

        fb::Loop::create(
            builder,
            &fb::LoopArgs {
                condition,
                block: Some(block),
                source_location: Some(source_location),
            },
        )
    }

    fn serialize_parallel<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        parallel: &LogicParallel,
    ) -> flatbuffers::WIPOffset<fb::Parallel<'a>> {
        let branch_offsets: Vec<_> = parallel
            .branches
            .iter()
            .map(|branch| Self::serialize_parallel_branch(builder, branch))
            .collect();
        let branches = builder.create_vector(&branch_offsets);

        fb::Parallel::create(
            builder,
            &fb::ParallelArgs {
                branches: Some(branches),
            },
        )
    }

    fn serialize_parallel_branch<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        branch: &LogicParallelBranch,
    ) -> flatbuffers::WIPOffset<fb::ParallelBranch<'a>> {
        let label = branch
            .label
            .as_deref()
            .map(|label| builder.create_string(label));
        let block = Self::serialize_block(builder, &branch.block);
        let source_location = serialize_source_location(builder, &branch.source_location);

        fb::ParallelBranch::create(
            builder,
            &fb::ParallelBranchArgs {
                label,
                block: Some(block),
                source_location: Some(source_location),
            },
        )
    }

    fn serialize_early_exit<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        early_exit: &LogicEarlyExit,
    ) -> flatbuffers::WIPOffset<fb::EarlyExit<'a>> {
        let reason = early_exit
            .reason
            .as_deref()
            .map(|reason| builder.create_string(reason));
        let block = Self::serialize_block(builder, &early_exit.block);
        let source_location = serialize_source_location(builder, &early_exit.source_location);

        fb::EarlyExit::create(
            builder,
            &fb::EarlyExitArgs {
                reason,
                block: Some(block),
                source_location: Some(source_location),
            },
        )
    }

    fn serialize_lifecycle<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        lifecycle: &ParticipantLifecycle,
    ) -> flatbuffers::WIPOffset<fb::ParticipantLifecycle<'a>> {
        let participant = builder.create_string(&lifecycle.participant);
        let source_location = serialize_source_location(builder, &lifecycle.source_location);

        fb::ParticipantLifecycle::create(
            builder,
            &fb::ParticipantLifecycleArgs {
                participant: Some(participant),
                action: map_lifecycle_action(lifecycle.action),
                source_location: Some(source_location),
            },
        )
    }

    fn serialize_reference<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        reference: &Reference,
    ) -> flatbuffers::WIPOffset<fb::Reference<'a>> {
        let participant_offsets: Vec<_> = reference
            .participants
            .iter()
            .map(|participant| builder.create_string(participant))
            .collect();
        let participants = builder.create_vector(&participant_offsets);
        let text = reference
            .text
            .as_deref()
            .map(|text| builder.create_string(text));
        let source_location = serialize_source_location(builder, &reference.source_location);

        fb::Reference::create(
            builder,
            &fb::ReferenceArgs {
                participants: Some(participants),
                text,
                source_location: Some(source_location),
            },
        )
    }
}

fn serialize_source_location<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    source_location: &SourceLocation,
) -> flatbuffers::WIPOffset<fb::SourceLocation<'a>> {
    let file = builder.create_string(source_location.file.as_ref());
    fb::SourceLocation::create(
        builder,
        &fb::SourceLocationArgs {
            file: Some(file),
            line: source_location.line,
        },
    )
}

fn map_lifecycle_action(action: LifecycleAction) -> fb::LifecycleAction {
    match action {
        LifecycleAction::Create => fb::LifecycleAction::Create,
        LifecycleAction::Activate => fb::LifecycleAction::Activate,
        LifecycleAction::Deactivate => fb::LifecycleAction::Deactivate,
        LifecycleAction::Destroy => fb::LifecycleAction::Destroy,
    }
}
