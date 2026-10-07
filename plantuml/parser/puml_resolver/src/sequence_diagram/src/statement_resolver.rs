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

use parser_core::format_arrow;
use sequence_logic::{
    Block, Interaction, LifecycleAction, Node, ParticipantId, ParticipantLifecycle, Reference,
    SourceLocation,
};
use sequence_parser::{GroupCmd, Message, MessageEndpoint, MessageSuffix, RefCmd, Statement};

use crate::error::SequenceResolverError;
use crate::participant_table::ParticipantTable;
use crate::sequence_tree_builder::SequenceTreeBuilder;

#[derive(Debug, Clone, Copy)]
struct LifecycleOp {
    action: LifecycleAction,
    target: LifecycleTarget,
}

#[derive(Debug, Clone, Copy)]
enum LifecycleTarget {
    Sender,
    Receiver,
}

pub(crate) fn build_sequence_tree(
    statements: &[Statement],
    table: &ParticipantTable,
) -> Result<Block, SequenceResolverError> {
    let mut builder = SequenceTreeBuilder::new();

    for statement in statements {
        consume_statement(&mut builder, statement, table)?;
    }

    builder.finish()
}

fn consume_statement(
    builder: &mut SequenceTreeBuilder,
    statement: &Statement,
    table: &ParticipantTable,
) -> Result<(), SequenceResolverError> {
    match statement {
        Statement::Message(message) => {
            for node in message_nodes(message, table)? {
                builder.push(node);
            }
        }
        Statement::GroupCmd(GroupCmd::Start(start)) => builder.start_group(start),
        Statement::GroupCmd(GroupCmd::Else(cmd)) => builder.else_group(cmd)?,
        Statement::GroupCmd(GroupCmd::End(end)) => builder.end_group(end)?,
        Statement::CreateCmd(create_cmd) => {
            builder.push(lifecycle_node(
                table.uid_of_identifier(&create_cmd.identifier, &create_cmd.source_location)?,
                LifecycleAction::Create,
                create_cmd.source_location.clone(),
            ));
        }
        Statement::DestroyCmd(destroy_cmd) => {
            builder.push(lifecycle_node(
                table.uid_of_ref(&destroy_cmd.participant, &destroy_cmd.source_location)?,
                LifecycleAction::Destroy,
                destroy_cmd.source_location.clone(),
            ));
        }
        Statement::ActivateCmd(activate_cmd) => {
            builder.push(lifecycle_node(
                table.uid_of_ref(&activate_cmd.participant, &activate_cmd.source_location)?,
                LifecycleAction::Activate,
                activate_cmd.source_location.clone(),
            ));
        }
        Statement::DeactivateCmd(deactivate_cmd) => {
            builder.push(lifecycle_node(
                table.uid_of_ref(&deactivate_cmd.participant, &deactivate_cmd.source_location)?,
                LifecycleAction::Deactivate,
                deactivate_cmd.source_location.clone(),
            ));
        }
        Statement::RefCmd(ref_cmd) => builder.push(reference_node(ref_cmd, table)?),
        Statement::ParticipantDef(_) => {}
        // `return` has no sender or receiver. Modeling it requires a call stack
        // to resolve the matching invocation, which the resolver does not yet maintain.
        Statement::ReturnCmd(_) => {}
    }

    Ok(())
}

fn message_nodes(
    message: &Message,
    table: &ParticipantTable,
) -> Result<Vec<Node>, SequenceResolverError> {
    let (sender, receiver) = directed_endpoints(message)?;

    let sender_name = endpoint_uid(sender, table, &message.source_location)?;
    let receiver_name = endpoint_uid(receiver, table, &message.source_location)?;

    let lifecycle_ops = collect_message_lifecycle_ops(message.suffix.as_ref());

    let mut nodes = Vec::new();

    // Create lifecycle happens before message delivery.
    for op in &lifecycle_ops {
        if op.action != LifecycleAction::Create {
            continue;
        }

        if let Some(participant) = receiver_name.clone() {
            nodes.push(lifecycle_node(
                participant,
                op.action,
                message.source_location.clone(),
            ));
        }
    }

    nodes.push(Node::Interaction(Interaction {
        sender: sender_name.clone(),
        receiver: receiver_name.clone(),
        message: message.description.clone(),
        source_location: message.source_location.clone(),
    }));

    // Other lifecycle actions happen after the interaction.
    for op in lifecycle_ops {
        if op.action == LifecycleAction::Create {
            continue;
        }

        let participant = match op.target {
            LifecycleTarget::Sender => &sender_name,
            LifecycleTarget::Receiver => &receiver_name,
        };

        if let Some(participant) = participant.clone() {
            nodes.push(lifecycle_node(
                participant,
                op.action,
                message.source_location.clone(),
            ));
        }
    }

    Ok(nodes)
}

fn collect_message_lifecycle_ops(suffix: Option<&MessageSuffix>) -> Vec<LifecycleOp> {
    let mut ops = Vec::new();

    if let Some(suffix) = suffix {
        collect_lifecycle_ops(suffix, &mut ops);
    }

    ops
}

fn collect_lifecycle_ops(suffix: &MessageSuffix, output: &mut Vec<LifecycleOp>) {
    match suffix {
        MessageSuffix::Combined(items) => {
            for item in items {
                collect_lifecycle_ops(item, output);
            }
        }
        suffix => {
            output.push(suffix_to_lifecycle(suffix));
        }
    }
}

fn suffix_to_lifecycle(suffix: &MessageSuffix) -> LifecycleOp {
    match suffix {
        MessageSuffix::Activate => LifecycleOp {
            action: LifecycleAction::Activate,
            target: LifecycleTarget::Receiver,
        },

        MessageSuffix::Deactivate => LifecycleOp {
            action: LifecycleAction::Deactivate,
            target: LifecycleTarget::Sender,
        },

        MessageSuffix::Create => LifecycleOp {
            action: LifecycleAction::Create,
            target: LifecycleTarget::Receiver,
        },

        MessageSuffix::Destroy => LifecycleOp {
            action: LifecycleAction::Destroy,
            target: LifecycleTarget::Receiver,
        },

        MessageSuffix::Combined(_) => {
            unreachable!("combined suffix must be flattened first")
        }
    }
}

fn reference_node(
    ref_cmd: &RefCmd,
    table: &ParticipantTable,
) -> Result<Node, SequenceResolverError> {
    let participants = ref_cmd
        .participants
        .iter()
        .map(|participant| table.uid_of_ref(participant, &ref_cmd.source_location))
        .collect::<Result<_, _>>()?;

    Ok(Node::Reference(Reference {
        participants,
        text: ref_cmd.text.clone(),
        source_location: ref_cmd.source_location.clone(),
    }))
}

fn lifecycle_node(
    participant: ParticipantId,
    action: LifecycleAction,
    source_location: SourceLocation,
) -> Node {
    Node::Lifecycle(ParticipantLifecycle {
        participant,
        action,
        source_location,
    })
}

fn endpoint_uid(
    endpoint: &MessageEndpoint,
    table: &ParticipantTable,
    source_location: &SourceLocation,
) -> Result<Option<ParticipantId>, SequenceResolverError> {
    match endpoint {
        MessageEndpoint::Participant(identifier) => table
            .uid_of_identifier(identifier, source_location)
            .map(Some),
        MessageEndpoint::LostFound(_) => Ok(None),
    }
}

fn directed_endpoints(
    message: &Message,
) -> Result<(&MessageEndpoint, &MessageEndpoint), SequenceResolverError> {
    let arrow = &message.arrow;

    let left_points_left = arrow.left.as_ref().is_some_and(|d| d.raw.contains('<'));

    let right_points_right = arrow.right.as_ref().is_some_and(|d| d.raw.contains('>'));

    let left_terminates = arrow.left.as_ref().is_some_and(|d| d.raw == "x");
    let right_terminates = arrow.right.as_ref().is_some_and(|d| d.raw == "x");

    match (
        left_points_left || (left_terminates && arrow.right.is_none()),
        right_points_right || (right_terminates && arrow.left.is_none()),
    ) {
        (true, false) => Ok((&message.right, &message.left)),
        (false, true) => Ok((&message.left, &message.right)),
        _ => Err(SequenceResolverError::InvalidMessageDirection {
            arrow: format_arrow(arrow),
            source_location: message.source_location.clone(),
        }),
    }
}

#[cfg(test)]
mod message_arrow_tests {
    use super::*;
    use crate::participant_table::build_participant_table;
    use parser_core::common_ast::{Arrow, ArrowDecor, ArrowLine};
    use sequence_parser::ParticipantIdentifier;

    fn table_for(statements: &[Statement]) -> ParticipantTable {
        build_participant_table(statements).expect("table must build")
    }

    fn resolve_message(message: &Message) -> Result<Vec<Node>, SequenceResolverError> {
        let table = table_for(&[Statement::Message(message.clone())]);
        message_nodes(message, &table)
    }

    fn arrow(line: &str, right: Option<&str>) -> Arrow {
        Arrow {
            left: None,
            line: ArrowLine {
                raw: line.to_string(),
            },
            middle: None,
            right: right.map(|r| ArrowDecor { raw: r.to_string() }),
        }
    }

    fn bidirectional_arrow() -> Arrow {
        Arrow {
            left: Some(ArrowDecor {
                raw: "<".to_string(),
            }),
            line: ArrowLine {
                raw: "--".to_string(),
            },
            middle: None,
            right: Some(ArrowDecor {
                raw: ">".to_string(),
            }),
        }
    }

    fn message(arrow: Arrow) -> Message {
        Message {
            left: MessageEndpoint::Participant(ParticipantIdentifier {
                display_name: "A".to_string(),
                alias: None,
            }),
            arrow,
            right: MessageEndpoint::Participant(ParticipantIdentifier {
                display_name: "B".to_string(),
                alias: None,
            }),
            suffix: None,
            description: None,
            source_location: SourceLocation::new("", 0),
        }
    }

    #[test]
    fn test_solid_directed_arrow_produces_interaction() {
        assert_eq!(
            resolve_message(&message(arrow("-", Some(">"))))
                .expect("must resolve a directed arrow")
                .len(),
            1
        );
    }

    #[test]
    fn test_dashed_directed_arrow_produces_interaction() {
        assert_eq!(
            resolve_message(&message(arrow("--", Some(">"))))
                .expect("must resolve a directed arrow")
                .len(),
            1
        );
    }

    #[test]
    fn test_bidirectional_arrow_is_rejected() {
        let err = directed_endpoints(&message(bidirectional_arrow()))
            .expect_err("must reject bidirectional arrows");

        assert!(matches!(
            err,
            SequenceResolverError::InvalidMessageDirection { arrow, .. }
                if arrow == "<-->"
        ));
    }

    #[test]
    fn test_undirected_arrow_is_rejected() {
        let err = directed_endpoints(&message(arrow("--", None)))
            .expect_err("must reject undirected arrows");

        assert!(matches!(
            err,
            SequenceResolverError::InvalidMessageDirection { arrow, .. }
                if arrow == "--"
        ));
    }

    #[test]
    fn test_lifecycle_suffixes_target_the_correct_message_endpoint() {
        let cases = [
            (MessageSuffix::Activate, "B", LifecycleAction::Activate),
            (MessageSuffix::Deactivate, "A", LifecycleAction::Deactivate),
            (MessageSuffix::Create, "B", LifecycleAction::Create),
            (MessageSuffix::Destroy, "B", LifecycleAction::Destroy),
        ];

        for (suffix, participant, action) in cases {
            let is_create = matches!(suffix, MessageSuffix::Create);
            let mut suffixed_message = message(arrow("-", Some(">")));
            suffixed_message.suffix = Some(suffix);

            let nodes = resolve_message(&suffixed_message).expect("suffix must resolve");
            let lifecycle_matches = |lifecycle: &ParticipantLifecycle| {
                lifecycle.participant.as_ref() == participant && lifecycle.action == action
            };

            if is_create {
                assert!(matches!(
                    nodes.as_slice(),
                    [
                        Node::Lifecycle(lifecycle),
                        Node::Interaction(_),
                    ] if lifecycle_matches(lifecycle)
                ));
            } else {
                assert!(matches!(
                    nodes.as_slice(),
                    [
                        Node::Interaction(_),
                        Node::Lifecycle(lifecycle),
                    ] if lifecycle_matches(lifecycle)
                ));
            }
        }
    }

    #[test]
    fn test_message_nodes_preserve_source_locations() {
        let call_location = SourceLocation::new("sequence/provenance_case.puml", 42);
        let return_location = SourceLocation::new("sequence/provenance_case.puml", 43);
        let mut call = message(arrow("-", Some(">")));
        call.source_location = call_location.clone();
        let mut return_message = message(arrow("--", Some(">")));
        return_message.source_location = return_location.clone();

        let statements = [Statement::Message(call), Statement::Message(return_message)];
        let root = build_sequence_tree(&statements, &table_for(&statements))
            .expect("messages must resolve");

        assert_eq!(root.items.len(), 2);
        let Node::Interaction(interaction) = &root.items[0] else {
            panic!("expected interaction node");
        };
        assert_eq!(interaction.source_location, call_location);

        let Node::Interaction(interaction) = &root.items[1] else {
            panic!("expected interaction node");
        };
        assert_eq!(interaction.source_location, return_location);
    }

    #[test]
    fn test_combined_lifecycle_suffix_resolves_in_source_order() {
        let mut combined_message = message(arrow("-", Some(">")));
        combined_message.suffix = Some(MessageSuffix::Combined(vec![
            MessageSuffix::Deactivate,
            MessageSuffix::Activate,
        ]));

        let nodes = resolve_message(&combined_message).expect("combined suffix must resolve");
        assert!(matches!(
            nodes.as_slice(),
            [
                Node::Interaction(interaction),
                Node::Lifecycle(deactivate),
                Node::Lifecycle(activate),
            ] if interaction.sender.as_deref() == Some("A")
                && interaction.receiver.as_deref() == Some("B")
                && deactivate.participant.as_ref() == "A"
                && deactivate.action == LifecycleAction::Deactivate
                && activate.participant.as_ref() == "B"
                && activate.action == LifecycleAction::Activate
        ));
    }

    #[test]
    fn test_combined_create_and_activate_suffix_creates_before_interaction() {
        let mut combined_message = message(arrow("-", Some(">")));
        combined_message.suffix = Some(MessageSuffix::Combined(vec![
            MessageSuffix::Create,
            MessageSuffix::Activate,
        ]));

        let nodes = resolve_message(&combined_message).expect("combined suffix must resolve");
        assert!(matches!(
            nodes.as_slice(),
            [
                Node::Lifecycle(create),
                Node::Interaction(_),
                Node::Lifecycle(activate),
            ] if create.participant.as_ref() == "B"
                && create.action == LifecycleAction::Create
                && activate.participant.as_ref() == "B"
                && activate.action == LifecycleAction::Activate
        ));
    }

    fn aliased_participant(display_name: &str, alias: &str) -> Statement {
        Statement::ParticipantDef(sequence_parser::sequence_ast::ParticipantDef {
            participant_type: sequence_parser::ParticipantType::Participant,
            identifier: ParticipantIdentifier {
                display_name: display_name.to_string(),
                alias: Some(alias.to_string()),
            },
            stereotype: None,
            source_location: SourceLocation::new("", 0),
        })
    }

    fn participant_ref(identifier: &str) -> sequence_parser::ParticipantRef {
        sequence_parser::ParticipantRef {
            identifier: identifier.to_string(),
        }
    }

    #[test]
    fn test_lifecycle_and_ref_statements_resolve_aliases_to_uids() {
        let statements = [
            aliased_participant("comp::unit", "u"),
            Statement::ActivateCmd(sequence_parser::ActivateCmd {
                participant: participant_ref("u"),
                source_location: SourceLocation::new("", 1),
            }),
            Statement::RefCmd(RefCmd {
                participants: vec![participant_ref("u")],
                text: None,
                source_location: SourceLocation::new("", 2),
            }),
        ];

        let root = build_sequence_tree(&statements, &table_for(&statements))
            .expect("statements must resolve");

        assert!(matches!(
            root.items.as_slice(),
            [Node::Lifecycle(activate), Node::Reference(reference)]
                if activate.participant.as_ref() == "comp.unit"
                    && reference.participants.iter().map(|p| p.as_ref()).eq(["comp.unit"])
        ));
    }

    #[test]
    fn test_unknown_lifecycle_participant_is_rejected() {
        let statements = [Statement::ActivateCmd(sequence_parser::ActivateCmd {
            participant: participant_ref("Ghost"),
            source_location: SourceLocation::new("", 7),
        })];

        let err = build_sequence_tree(&statements, &table_for(&statements))
            .expect_err("unknown participant must be rejected");

        assert!(matches!(
            err,
            SequenceResolverError::UnknownParticipant { reference, .. } if reference == "Ghost"
        ));
    }
}
