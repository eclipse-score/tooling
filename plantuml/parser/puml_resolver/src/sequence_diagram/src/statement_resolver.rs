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

use std::sync::Arc;

use parser_core::format_arrow;
use sequence_logic::{
    Block, Interaction, LifecycleAction, Node, ParticipantId, ParticipantLifecycle, Reference,
    SourceLocation,
};
use sequence_parser::{
    GroupCmd, Message, MessageEndpoint, MessageSuffix, ParticipantRef, RefCmd, Statement,
};

use crate::error::SequenceResolverError;
use crate::participant_table::participant_reference_name;
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
) -> Result<Block, SequenceResolverError> {
    let mut builder = SequenceTreeBuilder::new();

    for statement in statements {
        consume_statement(&mut builder, statement)?;
    }

    builder.finish()
}

fn consume_statement(
    builder: &mut SequenceTreeBuilder,
    statement: &Statement,
) -> Result<(), SequenceResolverError> {
    match statement {
        Statement::Message(message) => {
            for node in message_nodes(message)? {
                builder.push(node);
            }
        }
        Statement::GroupCmd(GroupCmd::Start(start)) => builder.start_group(start),
        Statement::GroupCmd(GroupCmd::Else(cmd)) => builder.else_group(cmd)?,
        Statement::GroupCmd(GroupCmd::End(end)) => builder.end_group(end)?,
        Statement::CreateCmd(create_cmd) => {
            builder.push(lifecycle_node(
                Arc::<str>::from(participant_reference_name(&create_cmd.identifier)),
                LifecycleAction::Create,
                create_cmd.source_location.clone(),
            ));
        }
        Statement::DestroyCmd(destroy_cmd) => {
            builder.push(lifecycle_node(
                participant_ref_name(&destroy_cmd.participant),
                LifecycleAction::Destroy,
                destroy_cmd.source_location.clone(),
            ));
        }
        Statement::ActivateCmd(activate_cmd) => {
            builder.push(lifecycle_node(
                participant_ref_name(&activate_cmd.participant),
                LifecycleAction::Activate,
                activate_cmd.source_location.clone(),
            ));
        }
        Statement::DeactivateCmd(deactivate_cmd) => {
            builder.push(lifecycle_node(
                participant_ref_name(&deactivate_cmd.participant),
                LifecycleAction::Deactivate,
                deactivate_cmd.source_location.clone(),
            ));
        }
        Statement::RefCmd(ref_cmd) => builder.push(reference_node(ref_cmd)),
        Statement::ParticipantDef(_) => {}
        // `return` has no sender or receiver. Modeling it requires a call stack
        // to resolve the matching invocation, which the resolver does not yet maintain.
        Statement::ReturnCmd(_) => {}
    }

    Ok(())
}

fn message_nodes(message: &Message) -> Result<Vec<Node>, SequenceResolverError> {
    let (sender, receiver) = directed_endpoints(message)?;

    let sender_name = endpoint_name(sender);
    let receiver_name = endpoint_name(receiver);

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

fn reference_node(ref_cmd: &RefCmd) -> Node {
    Node::Reference(Reference {
        participants: ref_cmd
            .participants
            .iter()
            .map(participant_ref_name)
            .collect(),
        text: ref_cmd.text.clone(),
        source_location: ref_cmd.source_location.clone(),
    })
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

fn endpoint_name(endpoint: &MessageEndpoint) -> Option<ParticipantId> {
    match endpoint {
        MessageEndpoint::Participant(identifier) => {
            Some(Arc::<str>::from(participant_reference_name(identifier)))
        }
        MessageEndpoint::LostFound(_) => None,
    }
}

fn participant_ref_name(participant: &ParticipantRef) -> ParticipantId {
    Arc::<str>::from(participant.identifier.as_str())
}

fn directed_endpoints(
    message: &Message,
) -> Result<(&MessageEndpoint, &MessageEndpoint), SequenceResolverError> {
    let arrow = &message.arrow;

    let left_arrow = arrow.left.as_ref().is_some_and(|d| d.raw.contains('<'));

    let right_arrow = arrow.right.as_ref().is_some_and(|d| d.raw.contains('>'));

    match (left_arrow, right_arrow) {
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
    use parser_core::common_ast::{Arrow, ArrowDecor, ArrowLine};
    use sequence_parser::ParticipantIdentifier;

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
            message_nodes(&message(arrow("-", Some(">"))))
                .expect("must resolve a directed arrow")
                .len(),
            1
        );
    }

    #[test]
    fn test_dashed_directed_arrow_produces_interaction() {
        assert_eq!(
            message_nodes(&message(arrow("--", Some(">"))))
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

            let nodes = message_nodes(&suffixed_message).expect("suffix must resolve");
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

        let root =
            build_sequence_tree(&[Statement::Message(call), Statement::Message(return_message)])
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

        let nodes = message_nodes(&combined_message).expect("combined suffix must resolve");
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

        let nodes = message_nodes(&combined_message).expect("combined suffix must resolve");
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
}
