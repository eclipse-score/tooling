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

use activity_diagram::{
    ActivityDiagram, ActivityStmt, BackwardNode, ControlKind, IfDisplay, IfNode, LoopDisplay,
    RepeatWhileNode, WhileNode,
};
use activity_fbs::activity as fb;
use flatbuffers::FlatBufferBuilder;

pub struct ActivitySerializer;

impl ActivitySerializer {
    pub fn serialize(diagram: &ActivityDiagram, source_file: &str) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        let name_offset = diagram.name.as_deref().map(|n| builder.create_string(n));

        let statement_offsets: Vec<_> = diagram
            .statements
            .iter()
            .map(|statement| Self::serialize_statement(&mut builder, statement))
            .collect();
        let statements_offset = builder.create_vector(&statement_offsets);

        let source_file_offset = builder.create_string(source_file);
        let root = fb::ActivityDiagram::create(
            &mut builder,
            &fb::ActivityDiagramArgs {
                name: name_offset,
                statements: Some(statements_offset),
                source_file: Some(source_file_offset),
            },
        );

        builder.finish(root, None);
        builder.finished_data().to_vec()
    }

    fn serialize_statement<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        statement: &ActivityStmt,
    ) -> flatbuffers::WIPOffset<fb::ActivityStmt<'a>> {
        let (value_type, value) = Self::serialize_statement_value(builder, statement);

        fb::ActivityStmt::create(
            builder,
            &fb::ActivityStmtArgs {
                value_type,
                value: Some(value),
            },
        )
    }

    fn serialize_statement_value(
        builder: &mut FlatBufferBuilder<'_>,
        statement: &ActivityStmt,
    ) -> (
        fb::ActivityStmtValue,
        flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
    ) {
        match statement {
            ActivityStmt::Title(title) => {
                let text_offset = builder.create_string(&title.text);
                let offset = fb::TitleNode::create(
                    builder,
                    &fb::TitleNodeArgs {
                        text: Some(text_offset),
                    },
                );
                (fb::ActivityStmtValue::TitleNode, offset.as_union_value())
            }
            ActivityStmt::Action(action) => {
                let label_offset = builder.create_string(&action.label);
                let offset = fb::ActionNode::create(
                    builder,
                    &fb::ActionNodeArgs {
                        label: Some(label_offset),
                    },
                );
                (fb::ActivityStmtValue::ActionNode, offset.as_union_value())
            }
            ActivityStmt::If(if_node) => {
                let offset = Self::serialize_if(builder, if_node);
                (fb::ActivityStmtValue::IfNode, offset.as_union_value())
            }
            ActivityStmt::While(while_node) => {
                let offset = Self::serialize_while(builder, while_node);
                (fb::ActivityStmtValue::WhileNode, offset.as_union_value())
            }
            ActivityStmt::RepeatWhile(repeat_while) => {
                let offset = Self::serialize_repeat_while(builder, repeat_while);
                (
                    fb::ActivityStmtValue::RepeatWhileNode,
                    offset.as_union_value(),
                )
            }
            ActivityStmt::Control(control) => {
                let offset = fb::ControlNode::create(
                    builder,
                    &fb::ControlNodeArgs {
                        kind: Self::map_control_kind(control.kind.clone()),
                    },
                );
                (fb::ActivityStmtValue::ControlNode, offset.as_union_value())
            }
        }
    }

    fn serialize_if<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        if_node: &IfNode,
    ) -> flatbuffers::WIPOffset<fb::IfNode<'a>> {
        let condition_offset = builder.create_string(&if_node.condition);
        let body_offsets: Vec<_> = if_node
            .body
            .iter()
            .map(|statement| Self::serialize_statement(builder, statement))
            .collect();
        let body_offset = builder.create_vector(&body_offsets);
        let else_branch_offsets: Vec<_> = if_node
            .else_branch
            .iter()
            .map(|statement| Self::serialize_statement(builder, statement))
            .collect();
        let else_branch_offset = builder.create_vector(&else_branch_offsets);
        let display_offset = if_node
            .display
            .as_ref()
            .map(|display| Self::serialize_if_display(builder, display));

        fb::IfNode::create(
            builder,
            &fb::IfNodeArgs {
                condition: Some(condition_offset),
                body: Some(body_offset),
                else_branch: Some(else_branch_offset),
                display: display_offset,
            },
        )
    }

    fn serialize_while<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        while_node: &WhileNode,
    ) -> flatbuffers::WIPOffset<fb::WhileNode<'a>> {
        let condition_offset = builder.create_string(&while_node.condition);
        let body_offsets: Vec<_> = while_node
            .body
            .iter()
            .map(|statement| Self::serialize_statement(builder, statement))
            .collect();
        let body_offset = builder.create_vector(&body_offsets);
        let backward_offset = while_node
            .backward
            .as_ref()
            .map(|backward| Self::serialize_backward(builder, backward));
        let display_offset = while_node
            .display
            .as_ref()
            .map(|display| Self::serialize_loop_display(builder, display));

        fb::WhileNode::create(
            builder,
            &fb::WhileNodeArgs {
                condition: Some(condition_offset),
                body: Some(body_offset),
                backward: backward_offset,
                display: display_offset,
            },
        )
    }

    fn serialize_repeat_while<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        repeat_while: &RepeatWhileNode,
    ) -> flatbuffers::WIPOffset<fb::RepeatWhileNode<'a>> {
        let body_offsets: Vec<_> = repeat_while
            .body
            .iter()
            .map(|statement| Self::serialize_statement(builder, statement))
            .collect();
        let body_offset = builder.create_vector(&body_offsets);
        let condition_offset = builder.create_string(&repeat_while.condition);
        let backward_offset = repeat_while
            .backward
            .as_ref()
            .map(|backward| Self::serialize_backward(builder, backward));
        let display_offset = repeat_while
            .display
            .as_ref()
            .map(|display| Self::serialize_loop_display(builder, display));

        fb::RepeatWhileNode::create(
            builder,
            &fb::RepeatWhileNodeArgs {
                body: Some(body_offset),
                condition: Some(condition_offset),
                backward: backward_offset,
                display: display_offset,
            },
        )
    }

    fn serialize_backward<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        backward: &BackwardNode,
    ) -> flatbuffers::WIPOffset<fb::BackwardNode<'a>> {
        let label_offset = builder.create_string(&backward.label);
        fb::BackwardNode::create(
            builder,
            &fb::BackwardNodeArgs {
                label: Some(label_offset),
            },
        )
    }

    fn serialize_if_display<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        display: &IfDisplay,
    ) -> flatbuffers::WIPOffset<fb::IfDisplay<'a>> {
        let then_label_offset = display
            .then_label
            .as_ref()
            .map(|label| builder.create_string(label));
        let else_label_offset = display
            .else_label
            .as_ref()
            .map(|label| builder.create_string(label));

        fb::IfDisplay::create(
            builder,
            &fb::IfDisplayArgs {
                then_label: then_label_offset,
                else_label: else_label_offset,
            },
        )
    }

    fn serialize_loop_display<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        display: &LoopDisplay,
    ) -> flatbuffers::WIPOffset<fb::LoopDisplay<'a>> {
        let continue_label_offset = display
            .continue_label
            .as_ref()
            .map(|label| builder.create_string(label));
        let exit_label_offset = display
            .exit_label
            .as_ref()
            .map(|label| builder.create_string(label));

        fb::LoopDisplay::create(
            builder,
            &fb::LoopDisplayArgs {
                continue_label: continue_label_offset,
                exit_label: exit_label_offset,
            },
        )
    }

    fn map_control_kind(kind: ControlKind) -> fb::ControlKind {
        match kind {
            ControlKind::Stop => fb::ControlKind::Stop,
            ControlKind::Kill => fb::ControlKind::Kill,
            ControlKind::Detach => fb::ControlKind::Detach,
            ControlKind::Break => fb::ControlKind::Break,
            ControlKind::Continue => fb::ControlKind::Continue,
        }
    }
}
