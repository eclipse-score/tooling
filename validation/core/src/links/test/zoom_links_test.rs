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

use super::*;
use crate::models::{ComponentDiagramInputs, ComponentType, LogicComponent, SequenceDiagramInputs};
use crate::validators::check_sequence_refinement;
use crate::ValidationResult;
use sequence_logic::{
    Block, Interaction, MessageKind, Node, ParticipantType, SequenceParticipant, SequenceTree,
    SourceLocation,
};

fn element(
    id: &str,
    parent: Option<&str>,
    element_type: ComponentType,
    stereotype: &str,
) -> LogicComponent {
    let alias = id.rsplit('.').next().unwrap();
    LogicComponent {
        id: id.to_string(),
        name: Some(alias.to_string()),
        alias: Some(alias.to_string()),
        parent_id: parent.map(str::to_string),
        element_type,
        stereotype: Some(stereotype.to_string()),
        relations: Vec::new(),
        source_location: SourceLocation::new("arch.puml", 1),
    }
}

/// `sl` (SEooC) > `comp` > `u1`.
fn index_of(diagrams: Vec<SequenceTree>) -> SequenceRefinementIndex {
    let mut result = ValidationResult::default();
    let architecture = ComponentDiagramInputs {
        entities: vec![
            element("sl", None, ComponentType::Package, "SEooC"),
            element("sl.comp", Some("sl"), ComponentType::Component, "component"),
            element(
                "sl.comp.u1",
                Some("sl.comp"),
                ComponentType::Component,
                "unit",
            ),
        ],
    }
    .to_diagram_architecture(&mut result);
    let inputs = SequenceDiagramInputs { diagrams };
    let sequence = inputs.to_sequence_diagram_index(&mut result);
    assert!(result.is_empty(), "{:?}", result.failures);
    SequenceRefinementIndex::build(&inputs.diagrams, &sequence, &architecture)
}

/// One call `app -> to : method` with its reply per entry, on line `n + 2`,
/// written in `file` (defaults to the diagram itself).
fn diagram(source: &str, calls: &[(&str, &str, Option<&str>)]) -> SequenceTree {
    diagram_with_replies(source, calls, true)
}

fn diagram_with_replies(
    source: &str,
    calls: &[(&str, &str, Option<&str>)],
    replies: bool,
) -> SequenceTree {
    let participant = |uid: &str| SequenceParticipant {
        display_name: uid.to_string(),
        alias: None,
        uid: uid.to_string(),
        participant_type: if uid == "app" {
            ParticipantType::Actor
        } else {
            ParticipantType::Participant
        },
        source_location: SourceLocation::new(source, 1),
        stereotype: None,
    };
    let mut uids: Vec<&str> = calls.iter().map(|(to, _, _)| *to).collect();
    uids.push("app");
    uids.sort();
    uids.dedup();

    SequenceTree {
        source: Some(source.to_string()),
        name: Some(source.to_string()),
        participants: uids.into_iter().map(participant).collect(),
        root: Block {
            items: calls
                .iter()
                .enumerate()
                .flat_map(|(n, (to, method, file))| {
                    let location = SourceLocation::new(file.unwrap_or(source), n as u32 + 2);
                    let call = Node::Interaction(Interaction {
                        sender: Some("app".to_string().into()),
                        receiver: Some(to.to_string().into()),
                        message: Some(method.to_string()),
                        kind: MessageKind::SynchCall,
                        source_location: location.clone(),
                    });
                    let reply = Node::Interaction(Interaction {
                        sender: Some(to.to_string().into()),
                        receiver: Some("app".to_string().into()),
                        message: None,
                        kind: MessageKind::Reply,
                        source_location: location,
                    });
                    [Some(call), replies.then_some(reply)].into_iter().flatten()
                })
                .collect(),
        },
    }
}

fn links_of(index: &SequenceRefinementIndex) -> Vec<CallLink> {
    call_links(index, &check_sequence_refinement(index))
}

fn expanded_diagram(source: &str, replies: bool) -> SequenceTree {
    let mut result = diagram_with_replies(source, &[("comp", "Get()", None)], replies);
    result.root.items.insert(
        1,
        Node::Interaction(Interaction {
            sender: Some("comp".to_string().into()),
            receiver: Some("comp".to_string().into()),
            message: Some("Fetch()".to_string()),
            kind: MessageKind::SynchCall,
            source_location: SourceLocation::new(source, 3),
        }),
    );
    result
}

#[test]
fn refined_call_links_to_its_sorted_details() {
    let index = index_of(vec![
        diagram("d/l0.puml", &[("sl", "Get(x)", None)]),
        diagram("d/l1_b.puml", &[("comp", "Get()", None)]),
        diagram("d/l1_a.puml", &[("comp", "Get()", None)]),
    ]);

    assert_eq!(
        links_of(&index),
        [CallLink {
            source: "d/l0.puml".to_string(),
            line: 2,
            method: "Get".to_string(),
            targets: vec!["d/l1_a.puml".to_string(), "d/l1_b.puml".to_string()],
        }]
    );
}

#[test]
fn call_without_detail_has_no_link() {
    let index = index_of(vec![
        diagram("d/l0.puml", &[("sl", "Get()", None)]),
        diagram("d/l1.puml", &[("comp", "Other()", None)]),
    ]);

    assert!(links_of(&index).is_empty());
}

#[test]
fn call_written_in_a_fragment_has_no_link() {
    let index = index_of(vec![
        diagram("d/l0.puml", &[("sl", "Get()", Some("d/fragment.iuml"))]),
        diagram("d/l1.puml", &[("comp", "Get()", None)]),
    ]);

    assert!(links_of(&index).is_empty());
}

#[test]
fn detail_failing_the_check_is_not_a_target() {
    let unclosed = diagram_with_replies("d/l1_bad.puml", &[("comp", "Get()", None)], false);
    let index = index_of(vec![
        diagram("d/l0.puml", &[("sl", "Get()", None)]),
        diagram("d/l1_ok.puml", &[("comp", "Get()", None)]),
        unclosed,
    ]);

    let links = links_of(&index);

    assert_eq!(links.len(), 1);
    assert_eq!(links[0].targets, ["d/l1_ok.puml"]);
}

#[test]
fn call_without_any_valid_detail_has_no_link() {
    let unclosed = diagram_with_replies("d/l1.puml", &[("comp", "Get()", None)], false);
    let index = index_of(vec![
        diagram("d/l0.puml", &[("sl", "Get()", None)]),
        unclosed,
    ]);

    assert!(links_of(&index).is_empty());
}

#[test]
fn unclosed_refined_call_has_no_link() {
    let unclosed = diagram_with_replies("d/l0.puml", &[("sl", "Get()", None)], false);
    let index = index_of(vec![
        unclosed,
        diagram("d/l1.puml", &[("comp", "Get()", None)]),
    ]);

    assert!(links_of(&index).is_empty());
}

#[test]
fn same_level_call_links_to_sorted_closed_details_only() {
    let index = index_of(vec![
        diagram("d/overview.puml", &[("comp", "Get(x)", None)]),
        expanded_diagram("d/detail_b.puml", true),
        expanded_diagram("d/detail_bad.puml", false),
        expanded_diagram("d/detail_a.puml", true),
    ]);

    assert_eq!(
        links_of(&index),
        [CallLink {
            source: "d/overview.puml".to_string(),
            line: 2,
            method: "Get".to_string(),
            targets: vec!["d/detail_a.puml".to_string(), "d/detail_b.puml".to_string()],
        }]
    );
}

#[test]
fn same_level_call_written_in_a_fragment_has_no_link() {
    let index = index_of(vec![
        diagram(
            "d/overview.puml",
            &[("comp", "Get()", Some("d/fragment.iuml"))],
        ),
        expanded_diagram("d/detail.puml", true),
    ]);

    assert!(links_of(&index).is_empty());
}
