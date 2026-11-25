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

use parser_core::common_ast::{Arrow, ArrowDecor};
use parser_core::format_arrow;
use sequence_logic::{
    Block, Interaction, LifecycleAction, MessageKind, Node, ParticipantId, ParticipantLifecycle,
    Reference, SourceLocation,
};
use sequence_parser::{
    GroupCmd, GroupKind, Message, MessageEndpoint, MessageSuffix, RefCmd, Statement,
};

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

/// A call that activated its callee.
#[derive(Clone, Debug, PartialEq)]
struct Activation {
    caller: Option<ParticipantId>,
    callee: ParticipantId,
    /// False for an `activate` that no call to the callee precedes: a
    /// `return` closes it without a reply.
    caller_known: bool,
}

/// Open activations of a group at its start and at the end of each case.
struct GroupFrame {
    kind: GroupKind,
    start: Vec<Activation>,
    ends: Vec<Vec<Activation>>,
}

impl GroupFrame {
    /// Activations after the group: a group that may be skipped changes
    /// nothing, an `alt` with `else` keeps what any case keeps open, a `par`
    /// keeps what every branch keeps open.
    fn merge(mut self) -> Vec<Activation> {
        let longest =
            |a: Vec<Activation>, b: Vec<Activation>| if b.len() > a.len() { b } else { a };
        let shortest =
            |a: Vec<Activation>, b: Vec<Activation>| if b.len() < a.len() { b } else { a };
        match self.kind {
            GroupKind::Opt | GroupKind::Loop | GroupKind::Break => self.start,
            GroupKind::Alt if self.ends.len() > 1 => {
                self.ends.into_iter().reduce(longest).unwrap_or(self.start)
            }
            GroupKind::Alt => self.start,
            GroupKind::Par => self.ends.into_iter().reduce(shortest).unwrap_or(self.start),
            GroupKind::Critical | GroupKind::Group => self.ends.pop().unwrap_or(self.start),
        }
    }
}

/// Open activations, innermost last. `return` replies to the caller of the
/// innermost one. Contents of `critical` are not in the model and are not
/// tracked.
#[derive(Default)]
struct Activations {
    open: Vec<Activation>,
    /// Callee and caller of the last call, until another node follows it.
    fresh_call: Option<(ParticipantId, Option<ParticipantId>)>,
    groups: Vec<GroupFrame>,
}

impl Activations {
    fn in_critical(&self) -> bool {
        self.groups
            .iter()
            .any(|frame| frame.kind == GroupKind::Critical)
    }

    fn observe(&mut self, node: &Node) {
        if self.in_critical() {
            return;
        }
        let fresh_call = self.fresh_call.take();

        match node {
            Node::Interaction(interaction)
                if matches!(
                    interaction.kind,
                    MessageKind::SynchCall | MessageKind::AsynchCall
                ) =>
            {
                if let Some(receiver) = &interaction.receiver {
                    self.fresh_call = Some((receiver.clone(), interaction.sender.clone()));
                }
            }
            Node::Lifecycle(lifecycle) => match lifecycle.action {
                LifecycleAction::Activate => {
                    let activation = match fresh_call {
                        Some((callee, caller)) if callee == lifecycle.participant => Activation {
                            caller,
                            callee,
                            caller_known: true,
                        },
                        _ => Activation {
                            caller: None,
                            callee: lifecycle.participant.clone(),
                            caller_known: false,
                        },
                    };
                    self.open.push(activation);
                }
                LifecycleAction::Deactivate => {
                    if let Some(index) = self
                        .open
                        .iter()
                        .rposition(|activation| activation.callee == lifecycle.participant)
                    {
                        self.open.remove(index);
                    }
                }
                LifecycleAction::Create | LifecycleAction::Destroy => {}
            },
            _ => {}
        }
    }

    // lobster-trace: Tools.ArchitectureModelingSequenceReturn
    /// Closes the innermost activation for a `return`.
    fn close_innermost(&mut self) -> Option<Activation> {
        if self.in_critical() {
            return None;
        }
        self.fresh_call = None;
        self.open.pop()
    }

    fn start_group(&mut self, kind: GroupKind) {
        self.fresh_call = None;
        self.groups.push(GroupFrame {
            kind,
            start: self.open.clone(),
            ends: Vec::new(),
        });
    }

    /// Every case of a group starts with the activations of the group start.
    fn else_group(&mut self) {
        if let Some(frame) = self.groups.last_mut() {
            let end = std::mem::replace(&mut self.open, frame.start.clone());
            frame.ends.push(end);
        }
    }

    fn end_group(&mut self) {
        self.fresh_call = None;
        if let Some(mut frame) = self.groups.pop() {
            frame.ends.push(std::mem::take(&mut self.open));
            self.open = frame.merge();
        }
    }
}

pub(crate) fn build_sequence_tree(
    statements: &[Statement],
    table: &ParticipantTable,
) -> Result<Block, SequenceResolverError> {
    let mut builder = SequenceTreeBuilder::new();
    let mut activations = Activations::default();

    for statement in statements {
        consume_statement(&mut builder, &mut activations, statement, table)?;
    }

    builder.finish()
}

fn push_observed(builder: &mut SequenceTreeBuilder, activations: &mut Activations, node: Node) {
    activations.observe(&node);
    builder.push(node);
}

fn consume_statement(
    builder: &mut SequenceTreeBuilder,
    activations: &mut Activations,
    statement: &Statement,
    table: &ParticipantTable,
) -> Result<(), SequenceResolverError> {
    match statement {
        Statement::Message(message) => {
            for node in message_nodes(message, table)? {
                push_observed(builder, activations, node);
            }
        }
        Statement::GroupCmd(GroupCmd::Start(start)) => {
            activations.start_group(start.kind);
            builder.start_group(start);
        }
        Statement::GroupCmd(GroupCmd::Else(cmd)) => {
            activations.else_group();
            builder.else_group(cmd)?;
        }
        Statement::GroupCmd(GroupCmd::End(end)) => {
            activations.end_group();
            builder.end_group(end)?;
        }
        Statement::CreateCmd(create_cmd) => {
            push_observed(
                builder,
                activations,
                lifecycle_node(
                    table.uid_of_identifier(&create_cmd.identifier, &create_cmd.source_location)?,
                    LifecycleAction::Create,
                    create_cmd.source_location.clone(),
                ),
            );
        }
        Statement::DestroyCmd(destroy_cmd) => {
            push_observed(
                builder,
                activations,
                lifecycle_node(
                    table.uid_of_ref(&destroy_cmd.participant, &destroy_cmd.source_location)?,
                    LifecycleAction::Destroy,
                    destroy_cmd.source_location.clone(),
                ),
            );
        }
        Statement::ActivateCmd(activate_cmd) => {
            push_observed(
                builder,
                activations,
                lifecycle_node(
                    table.uid_of_ref(&activate_cmd.participant, &activate_cmd.source_location)?,
                    LifecycleAction::Activate,
                    activate_cmd.source_location.clone(),
                ),
            );
        }
        Statement::DeactivateCmd(deactivate_cmd) => {
            push_observed(
                builder,
                activations,
                lifecycle_node(
                    table
                        .uid_of_ref(&deactivate_cmd.participant, &deactivate_cmd.source_location)?,
                    LifecycleAction::Deactivate,
                    deactivate_cmd.source_location.clone(),
                ),
            );
        }
        Statement::RefCmd(ref_cmd) => builder.push(reference_node(ref_cmd, table)?),
        Statement::ParticipantDef(_) => {}
        // A `return` without a caller to reply to produces no message.
        Statement::ReturnCmd(return_cmd) => {
            if let Some(activation) = activations.close_innermost() {
                if activation.caller_known {
                    builder.push(Node::Interaction(Interaction {
                        sender: Some(activation.callee),
                        receiver: activation.caller,
                        message: return_cmd.label.clone(),
                        kind: MessageKind::Reply,
                        source_location: return_cmd.source_location.clone(),
                    }));
                }
            }
        }
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
        kind: message_kind(&message.arrow, &lifecycle_ops),
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

// lobster-trace: Tools.ArchitectureModelingSequenceCreate
// lobster-trace: Tools.ArchitectureModelingSequenceMessageKinds
/// Create (`**`) and delete (`!!`) suffixes take precedence, then a dashed line
/// is a reply, then an open arrowhead (`>>`) is asynchronous.
fn message_kind(arrow: &Arrow, lifecycle_ops: &[LifecycleOp]) -> MessageKind {
    let has_action = |action| lifecycle_ops.iter().any(|op| op.action == action);

    if has_action(LifecycleAction::Create) {
        return MessageKind::Create;
    }
    if has_action(LifecycleAction::Destroy) {
        return MessageKind::Delete;
    }
    if arrow.line.raw.contains("--") || arrow.line.raw.contains("..") {
        return MessageKind::Reply;
    }

    let open_head = |decor: &Option<ArrowDecor>| {
        decor
            .as_ref()
            .is_some_and(|d| d.raw.contains(">>") || d.raw.contains("<<"))
    };
    if open_head(&arrow.left) || open_head(&arrow.right) {
        MessageKind::AsynchCall
    } else {
        MessageKind::SynchCall
    }
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
        build_participant_table(statements, &[]).expect("table must build")
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
    fn test_message_kind_follows_arrow_and_suffix() {
        let kind_of = |arrow: Arrow, suffix: Option<MessageSuffix>| {
            let mut msg = message(arrow);
            msg.suffix = suffix;
            let nodes = resolve_message(&msg).expect("message must resolve");
            nodes
                .iter()
                .find_map(|node| match node {
                    Node::Interaction(interaction) => Some(interaction.kind),
                    _ => None,
                })
                .expect("interaction expected")
        };

        assert_eq!(kind_of(arrow("-", Some(">")), None), MessageKind::SynchCall);
        assert_eq!(
            kind_of(arrow("-", Some(">\\")), None),
            MessageKind::SynchCall
        );
        assert_eq!(
            kind_of(arrow("-", Some(">>")), None),
            MessageKind::AsynchCall
        );
        assert_eq!(kind_of(arrow("--", Some(">")), None), MessageKind::Reply);
        assert_eq!(kind_of(arrow("--", Some(">>")), None), MessageKind::Reply);
        assert_eq!(
            kind_of(arrow("-", Some(">")), Some(MessageSuffix::Create)),
            MessageKind::Create
        );
        assert_eq!(
            kind_of(arrow("-", Some(">")), Some(MessageSuffix::Destroy)),
            MessageKind::Delete
        );
        assert_eq!(
            kind_of(arrow("-", Some(">")), Some(MessageSuffix::Activate)),
            MessageKind::SynchCall
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

    #[test]
    fn test_create_statement_keeps_following_message_a_call() {
        let create_b = Statement::CreateCmd(sequence_parser::CreateCmd {
            participant_type: sequence_parser::ParticipantType::Participant,
            identifier: ParticipantIdentifier {
                display_name: "B".to_string(),
                alias: None,
            },
            stereotype: None,
            source_location: SourceLocation::new("", 1),
        });
        let statements = [create_b, Statement::Message(message(arrow("-", Some(">"))))];

        let root = build_sequence_tree(&statements, &table_for(&statements))
            .expect("statements must resolve");

        assert!(matches!(
            root.items.as_slice(),
            [
                Node::Lifecycle(create),
                Node::Interaction(interaction),
            ] if create.action == LifecycleAction::Create
                && interaction.kind == MessageKind::SynchCall
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

    fn activating_call() -> Statement {
        let mut call = message(arrow("-", Some(">")));
        call.suffix = Some(MessageSuffix::Activate);
        Statement::Message(call)
    }

    fn return_cmd(label: &str) -> Statement {
        Statement::ReturnCmd(sequence_parser::ReturnCmd {
            label: Some(label.to_string()),
            source_location: SourceLocation::new("", 9),
        })
    }

    fn replies(root: &Block) -> Vec<(String, String, String)> {
        let mut found = Vec::new();
        for node in &root.items {
            if let Node::Interaction(i) = node {
                if i.kind == MessageKind::Reply {
                    found.push((
                        i.sender.as_deref().unwrap_or("").to_string(),
                        i.receiver.as_deref().unwrap_or("").to_string(),
                        i.message.clone().unwrap_or_default(),
                    ));
                }
            }
        }
        found
    }

    #[test]
    fn test_return_replies_to_the_caller_of_the_innermost_activation() {
        let statements = [activating_call(), return_cmd("ok")];

        let root = build_sequence_tree(&statements, &table_for(&statements))
            .expect("statements must resolve");

        assert_eq!(
            replies(&root),
            [("B".to_string(), "A".to_string(), "ok".to_string())]
        );
    }

    #[test]
    fn test_return_without_activation_produces_no_message() {
        let statements = [
            Statement::Message(message(arrow("-", Some(">")))),
            return_cmd("ok"),
        ];

        let root = build_sequence_tree(&statements, &table_for(&statements))
            .expect("statements must resolve");

        assert!(replies(&root).is_empty());
    }

    #[test]
    fn test_return_closes_each_alt_branch_separately() {
        let location = SourceLocation::new("", 5);
        let statements = [
            activating_call(),
            Statement::GroupCmd(GroupCmd::Start(sequence_parser::GroupStart {
                kind: sequence_parser::GroupKind::Alt,
                label: None,
                is_parallel: false,
                source_location: location.clone(),
            })),
            return_cmd("first"),
            Statement::GroupCmd(GroupCmd::Else(sequence_parser::GroupElse {
                label: None,
                source_location: location.clone(),
            })),
            return_cmd("second"),
            Statement::GroupCmd(GroupCmd::End(sequence_parser::GroupEnd {
                kind: Some(sequence_parser::GroupKind::Alt),
                source_location: location,
            })),
        ];

        let root = build_sequence_tree(&statements, &table_for(&statements))
            .expect("statements must resolve");

        let Node::Branch(branch) = &root.items[2] else {
            panic!("expected branch node");
        };
        assert_eq!(branch.cases.len(), 2);
        for case in &branch.cases {
            assert!(matches!(
                case.block.items.as_slice(),
                [Node::Interaction(reply)] if reply.kind == MessageKind::Reply
            ));
        }
    }

    fn group_start(kind: sequence_parser::GroupKind) -> Statement {
        Statement::GroupCmd(GroupCmd::Start(sequence_parser::GroupStart {
            kind,
            label: None,
            is_parallel: false,
            source_location: SourceLocation::new("", 5),
        }))
    }

    fn group_else() -> Statement {
        Statement::GroupCmd(GroupCmd::Else(sequence_parser::GroupElse {
            label: None,
            source_location: SourceLocation::new("", 5),
        }))
    }

    fn group_end(kind: sequence_parser::GroupKind) -> Statement {
        Statement::GroupCmd(GroupCmd::End(sequence_parser::GroupEnd {
            kind: Some(kind),
            source_location: SourceLocation::new("", 5),
        }))
    }

    fn top_level_replies(statements: &[Statement]) -> usize {
        let root = build_sequence_tree(statements, &table_for(statements))
            .expect("statements must resolve");
        replies(&root).len()
    }

    #[test]
    fn test_return_after_alt_with_early_return_in_one_case_still_replies() {
        use sequence_parser::GroupKind::Alt;
        let statements = [
            activating_call(),
            group_start(Alt),
            return_cmd("early"),
            group_else(),
            group_end(Alt),
            return_cmd("late"),
        ];

        assert_eq!(top_level_replies(&statements), 1);
    }

    #[test]
    fn test_return_in_optional_group_leaves_the_activation_open() {
        use sequence_parser::GroupKind::Opt;
        let statements = [
            activating_call(),
            group_start(Opt),
            return_cmd("early"),
            group_end(Opt),
            return_cmd("late"),
        ];

        assert_eq!(top_level_replies(&statements), 1);
    }

    #[test]
    fn test_return_in_every_alt_case_closes_the_activation() {
        use sequence_parser::GroupKind::Alt;
        let statements = [
            activating_call(),
            group_start(Alt),
            return_cmd("first"),
            group_else(),
            return_cmd("second"),
            group_end(Alt),
            return_cmd("late"),
        ];

        assert_eq!(top_level_replies(&statements), 0);
    }

    #[test]
    fn test_return_in_one_parallel_branch_closes_the_activation() {
        use sequence_parser::GroupKind::Par;
        let statements = [
            activating_call(),
            group_start(Par),
            return_cmd("first"),
            group_else(),
            group_end(Par),
            return_cmd("late"),
        ];

        assert_eq!(top_level_replies(&statements), 0);
    }

    #[test]
    fn test_return_inside_critical_is_ignored() {
        use sequence_parser::GroupKind::Critical;
        let statements = [
            activating_call(),
            group_start(Critical),
            return_cmd("dropped"),
            group_end(Critical),
            return_cmd("kept"),
        ];

        assert_eq!(top_level_replies(&statements), 1);
    }

    #[test]
    fn test_activate_without_a_call_has_no_caller_to_reply_to() {
        let statements = [
            Statement::Message(message(arrow("-", Some(">")))),
            Statement::ActivateCmd(sequence_parser::ActivateCmd {
                participant: participant_ref("A"),
                source_location: SourceLocation::new("", 6),
            }),
            return_cmd("ok"),
        ];

        assert_eq!(top_level_replies(&statements), 0);
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
