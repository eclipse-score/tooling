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

use super::super::fixtures::*;
use super::*;
use crate::models::{ComponentDiagramArchitecture, SequenceDiagramInputs};
use sequence_logic::{
    Block, Branch, BranchCase, EarlyExit, Interaction, LifecycleAction, Loop, MessageKind, Node,
    Parallel, ParallelBranch, ParticipantLifecycle, SequenceParticipant, SequenceTree,
};
use std::collections::BTreeSet;

/// Steps of a sequence diagram; `app` is an actor, `platform` and `log` are
/// `<<external>>`.
enum S {
    Call(&'static str, &'static str, &'static str),
    Async(&'static str, &'static str, &'static str),
    Reply(&'static str, &'static str),
    Activate(&'static str),
    Deactivate(&'static str),
    Alt(Vec<S>),
    AltElse(Vec<S>, Vec<S>),
    Loop(Vec<S>),
    Break(Vec<S>),
    Par(Vec<S>, Vec<S>),
}

fn collect_uids(steps: &[S], uids: &mut BTreeSet<&'static str>) {
    for step in steps {
        match step {
            S::Call(from, to, _) | S::Async(from, to, _) | S::Reply(from, to) => {
                uids.extend([from, to]);
            }
            S::Activate(uid) | S::Deactivate(uid) => {
                uids.insert(uid);
            }
            S::Alt(inner) | S::Loop(inner) | S::Break(inner) => collect_uids(inner, uids),
            S::AltElse(first, second) | S::Par(first, second) => {
                collect_uids(first, uids);
                collect_uids(second, uids);
            }
        }
    }
}

fn node(step: &S) -> Node {
    let interaction = |from: &str, to: &str, message: &str, kind| {
        Node::Interaction(Interaction {
            sender: Some(from.to_string().into()),
            receiver: Some(to.to_string().into()),
            message: Some(message.to_string()),
            kind,
            source_location: dummy_source_location(),
        })
    };

    match step {
        S::Call(from, to, method) => interaction(from, to, method, MessageKind::SynchCall),
        S::Async(from, to, method) => interaction(from, to, method, MessageKind::AsynchCall),
        S::Reply(from, to) => interaction(from, to, "", MessageKind::Reply),
        S::Activate(uid) => lifecycle(uid, LifecycleAction::Activate),
        S::Deactivate(uid) => lifecycle(uid, LifecycleAction::Deactivate),
        S::Alt(inner) => Node::Branch(Branch {
            cases: vec![branch_case(inner)],
        }),
        S::AltElse(first, second) => Node::Branch(Branch {
            cases: vec![branch_case(first), branch_case(second)],
        }),
        S::Loop(inner) => Node::Loop(Loop {
            condition: None,
            block: block(inner),
            source_location: dummy_source_location(),
        }),
        S::Break(inner) => Node::EarlyExit(EarlyExit {
            reason: None,
            block: block(inner),
            source_location: dummy_source_location(),
        }),
        S::Par(first, second) => Node::Parallel(Parallel {
            branches: [first, second]
                .into_iter()
                .map(|inner| ParallelBranch {
                    label: None,
                    block: block(inner),
                    source_location: dummy_source_location(),
                })
                .collect(),
        }),
    }
}

fn lifecycle(uid: &str, action: LifecycleAction) -> Node {
    Node::Lifecycle(ParticipantLifecycle {
        participant: uid.to_string().into(),
        action,
        source_location: dummy_source_location(),
    })
}

fn block(steps: &[S]) -> Block {
    Block {
        items: steps.iter().map(node).collect(),
    }
}

fn branch_case(steps: &[S]) -> BranchCase {
    BranchCase {
        condition: None,
        block: block(steps),
        source_location: dummy_source_location(),
    }
}

fn diagram(name: &str, steps: Vec<S>) -> SequenceTree {
    let mut uids = BTreeSet::new();
    collect_uids(&steps, &mut uids);
    let participants: Vec<SequenceParticipant> = uids
        .into_iter()
        .map(|uid| match uid {
            "app" => actor_participant(uid),
            "platform" | "log" => external_participant(uid),
            _ => element_participant(uid),
        })
        .collect();

    SequenceTree {
        source: None,
        name: Some(name.to_string()),
        participants,
        root: block(&steps),
    }
}

/// `sl` (SEooC) > `comp` > `u1`, `u2`; `sl` > `other` > `u9`.
fn architecture() -> ComponentDiagramArchitecture {
    let mut setup_result = ValidationResult::default();
    let architecture = component_diagram(vec![
        seooc("sl"),
        component_with_parent_id("comp", "sl"),
        unit_with_parent_id("u1", "sl.comp"),
        unit_with_parent_id("u2", "sl.comp"),
        component_with_parent_id("other", "sl"),
        unit_with_parent_id("u9", "sl.other"),
    ])
    .to_diagram_architecture(&mut setup_result);
    assert!(setup_result.is_empty(), "{:?}", setup_result.failures);
    architecture
}

fn index_of(diagrams: Vec<SequenceTree>) -> SequenceRefinementIndex {
    let mut setup_result = ValidationResult::default();
    let inputs = SequenceDiagramInputs { diagrams };
    let sequence = inputs.to_sequence_diagram_index(&mut setup_result);
    assert!(setup_result.is_empty(), "{:?}", setup_result.failures);

    SequenceRefinementIndex::build(&inputs.diagrams, &sequence, &architecture())
}

fn validate(diagrams: Vec<SequenceTree>) -> ValidationResult {
    validate_sequence_refinement(&index_of(diagrams))
}

fn assert_findings(result: &ValidationResult, findings: &[&str]) {
    let found: Vec<&str> = findings
        .iter()
        .copied()
        .filter(|finding| {
            result
                .failures
                .iter()
                .any(|failure| failure.contains(finding))
        })
        .collect();
    assert_eq!(
        result.failures.len(),
        findings.len(),
        "{:?}",
        result.failures
    );
    assert_eq!(found, findings, "{:?}", result.failures);
}

fn l0() -> SequenceTree {
    diagram(
        "l0",
        vec![
            S::Call("app", "sl", "Get()"),
            S::Call("sl", "platform", "Log()"),
            S::Reply("sl", "app"),
        ],
    )
}

fn l1() -> SequenceTree {
    diagram(
        "l1",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Call("comp", "platform", "Log()"),
            S::Reply("comp", "app"),
        ],
    )
}

fn l2() -> SequenceTree {
    diagram(
        "l2",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "u2", "Data()"),
            S::Reply("u2", "u1"),
            S::Call("u1", "platform", "Log()"),
            S::Reply("u1", "app"),
        ],
    )
}

fn position(index: &SequenceRefinementIndex, container: &str) -> usize {
    index
        .entries
        .iter()
        .position(|entry| entry.container == container && entry.sender.label() == "app")
        .unwrap_or_else(|| panic!("no entry into {container}"))
}

#[test]
fn links_each_level_to_its_direct_children_only() {
    let index = index_of(vec![l0(), l1(), l2()]);
    let (sl, comp, u1) = (
        position(&index, "sl"),
        position(&index, "sl.comp"),
        position(&index, "sl.comp.u1"),
    );

    assert_eq!(index.details[sl], [comp]);
    assert_eq!(index.details[comp], [u1]);
    assert!(index.details[u1].is_empty());
}

#[test]
fn three_consistent_levels_pass() {
    assert_findings(&validate(vec![l0(), l1(), l2()]), &[]);
}

#[test]
fn call_without_detail_stays_a_black_box() {
    assert_findings(&validate(vec![l0()]), &[]);
}

#[test]
fn method_differs_so_there_is_no_link() {
    let typo = diagram(
        "typo",
        vec![S::Call("app", "comp", "Gett()"), S::Reply("comp", "app")],
    );

    let index = index_of(vec![l0(), typo]);

    assert!(index.details[position(&index, "sl")].is_empty());
    assert_findings(&validate_sequence_refinement(&index), &[]);
}

#[test]
fn reports_exit_missing_in_the_only_detail() {
    let no_log = diagram(
        "no_log",
        vec![S::Call("app", "comp", "Get()"), S::Reply("comp", "app")],
    );

    let result = validate(vec![l0(), no_log]);

    assert_findings(&result, &["ZoomExitMissing"]);
    assert!(result.failures[0].contains("\"sl\" -> \"platform\" : \"Log\""));
    assert!(result.failures[0].contains("\"no_log\""));
}

#[test]
fn exits_may_be_covered_by_different_variants() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Call("comp", "platform", "Log()"),
            S::Call("comp", "log", "Flush()"),
            S::Reply("comp", "app"),
        ],
    );
    let log_only = diagram(
        "log_only",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "platform", "Log()"),
            S::Reply("u1", "app"),
        ],
    );
    let flush_only = diagram(
        "flush_only",
        vec![
            S::Call("app", "u2", "Get()"),
            S::Call("u2", "log", "Flush()"),
            S::Reply("u2", "app"),
        ],
    );

    assert_findings(
        &validate(vec![overview.clone(), log_only.clone(), flush_only]),
        &[],
    );
    assert_findings(&validate(vec![overview, log_only]), &["ZoomExitMissing"]);
}

#[test]
fn detail_may_add_an_exit_to_the_environment() {
    let extra = diagram(
        "extra",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "platform", "Log()"),
            S::Call("u1", "log", "Metric()"),
            S::Reply("u1", "app"),
        ],
    );

    assert_findings(&validate(vec![l1(), extra]), &[]);
}

#[test]
fn reports_lifeline_outside_the_container() {
    let outside = diagram(
        "outside",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "platform", "Log()"),
            S::Call("u1", "u9", "Lookup()"),
            S::Reply("u1", "app"),
        ],
    );

    let result = validate(vec![l1(), outside]);

    assert_findings(&result, &["ZoomParticipantOutsideContainer"]);
    assert!(result.failures[0].contains("\"sl.other.u9\""));
}

#[test]
fn exit_partner_and_its_children_are_allowed_in_the_detail() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Call("comp", "other", "Lookup()"),
            S::Reply("comp", "app"),
        ],
    );
    let detail = diagram(
        "detail",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "u9", "Lookup()"),
            S::Reply("u1", "app"),
        ],
    );

    assert_findings(&validate(vec![overview, detail]), &[]);
}

#[test]
fn elements_called_by_an_exit_partner_are_allowed_in_the_detail() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Call("comp", "u9", "Lookup()"),
            S::Reply("comp", "app"),
        ],
    );
    let downstream = diagram(
        "downstream",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "u9", "Lookup()"),
            S::Call("u9", "other", "Resolve()"),
            S::Reply("u1", "app"),
        ],
    );
    let not_downstream = diagram(
        "not_downstream",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "u9", "Lookup()"),
            S::Call("u1", "other", "Resolve()"),
            S::Reply("u1", "app"),
        ],
    );

    assert_findings(&validate(vec![overview.clone(), downstream]), &[]);
    assert_findings(
        &validate(vec![overview, not_downstream]),
        &["ZoomParticipantOutsideContainer"],
    );
}

#[test]
fn exits_of_two_entries_in_one_diagram_stay_apart() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Call("comp", "platform", "Log()"),
            S::Reply("comp", "app"),
            S::Call("app", "comp", "Reset()"),
            S::Call("comp", "log", "Flush()"),
            S::Reply("comp", "app"),
        ],
    );

    assert_findings(&validate(vec![overview, l2()]), &[]);
}

#[test]
fn exit_inside_a_branch_counts() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Alt(vec![S::Call("comp", "platform", "Log()")]),
            S::Reply("comp", "app"),
        ],
    );
    let no_log = diagram(
        "no_log",
        vec![S::Call("app", "u1", "Get()"), S::Reply("u1", "app")],
    );

    assert_findings(&validate(vec![overview, no_log]), &["ZoomExitMissing"]);
}

#[test]
fn unresolved_participants_are_not_reported() {
    let unresolved = diagram(
        "unresolved",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "platform", "Log()"),
            S::Call("u1", "ghost", "Haunt()"),
            S::Reply("u1", "app"),
        ],
    );

    assert_findings(&validate(vec![l1(), unresolved]), &[]);
}

#[test]
fn reports_refined_call_that_is_never_closed() {
    let open = diagram(
        "open",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Call("comp", "platform", "Log()"),
        ],
    );

    let result = validate(vec![open, l2()]);

    assert_findings(&result, &["ZoomWindowNotClosed"]);
}

#[test]
fn asynchronous_call_is_closed_by_deactivate() {
    let overview = diagram(
        "overview",
        vec![
            S::Async("app", "comp", "Get()"),
            S::Call("comp", "platform", "Log()"),
            S::Deactivate("comp"),
        ],
    );
    let detail = diagram(
        "detail",
        vec![
            S::Async("app", "u1", "Get()"),
            S::Call("u1", "platform", "Log()"),
            S::Deactivate("u1"),
        ],
    );

    assert_findings(&validate(vec![overview, detail]), &[]);
}

#[test]
fn asynchronous_call_needs_no_close() {
    let overview = diagram(
        "overview",
        vec![
            S::Async("app", "comp", "Get()"),
            S::Call("comp", "platform", "Log()"),
        ],
    );
    let detail = diagram(
        "detail",
        vec![
            S::Async("app", "u1", "Get()"),
            S::Call("u1", "platform", "Log()"),
        ],
    );

    assert_findings(&validate(vec![overview, detail]), &[]);
}

#[test]
fn asynchronous_window_without_close_runs_to_the_end_of_the_diagram() {
    let overview = diagram(
        "overview",
        vec![
            S::Async("app", "comp", "Get()"),
            S::Call("comp", "platform", "Log()"),
        ],
    );
    let detail = diagram("detail", vec![S::Async("app", "u1", "Get()")]);

    assert_findings(&validate(vec![overview, detail]), &["ZoomExitMissing"]);
}

#[test]
fn asynchronous_callback_inside_the_window_needs_no_reply() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Async("platform", "comp", "OnEvent()"),
            S::Reply("comp", "app"),
        ],
    );
    let detail = diagram(
        "detail",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Async("platform", "u1", "OnEvent()"),
            S::Reply("u1", "app"),
        ],
    );

    assert_findings(&validate(vec![overview, detail]), &[]);
}

#[test]
fn reply_with_deactivate_to_a_callback_does_not_close_the_outer_window() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "sl", "Get()"),
            S::Activate("sl"),
            S::Call("platform", "sl", "OnEvent()"),
            S::Activate("sl"),
            S::Reply("sl", "platform"),
            S::Deactivate("sl"),
            S::Call("sl", "log", "Write()"),
            S::Reply("sl", "app"),
        ],
    );
    let detail = diagram(
        "detail",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Call("platform", "comp", "OnEvent()"),
            S::Reply("comp", "platform"),
            S::Reply("comp", "app"),
        ],
    );

    assert_findings(&validate(vec![overview, detail]), &["ZoomExitMissing"]);
}

#[test]
fn deactivating_the_container_ends_the_window_despite_a_pending_callback() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "sl", "Get()"),
            S::Call("platform", "comp", "OnEvent()"),
            S::Deactivate("sl"),
            S::Call("sl", "log", "Write()"),
        ],
    );
    let detail = diagram(
        "detail",
        vec![S::Call("app", "comp", "Get()"), S::Reply("comp", "app")],
    );

    assert_findings(&validate(vec![overview, detail]), &[]);
}

#[test]
fn call_that_is_not_refined_may_stay_open() {
    let open = diagram("open", vec![S::Call("app", "comp", "Get()")]);

    assert_findings(&validate(vec![open]), &[]);
}

fn without_log() -> SequenceTree {
    diagram(
        "no_log",
        vec![S::Call("app", "u1", "Get()"), S::Reply("u1", "app")],
    )
}

#[test]
fn early_reply_in_a_branch_does_not_end_the_window() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Alt(vec![S::Reply("comp", "app")]),
            S::Call("comp", "platform", "Log()"),
            S::Reply("comp", "app"),
        ],
    );

    assert_findings(
        &validate(vec![overview, without_log()]),
        &["ZoomExitMissing"],
    );
}

#[test]
fn early_reply_in_a_detail_branch_does_not_hide_outside_participants() {
    let detail = diagram(
        "detail",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Alt(vec![S::Reply("u1", "app")]),
            S::Call("u1", "platform", "Log()"),
            S::Call("u1", "u9", "Lookup()"),
            S::Reply("u1", "app"),
        ],
    );

    assert_findings(
        &validate(vec![l1(), detail]),
        &["ZoomParticipantOutsideContainer"],
    );
}

#[test]
fn reply_in_every_case_of_an_alt_with_else_closes_the_window() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::AltElse(
                vec![
                    S::Call("comp", "platform", "Log()"),
                    S::Reply("comp", "app"),
                ],
                vec![S::Reply("comp", "app")],
            ),
            S::Call("comp", "log", "Late()"),
        ],
    );

    assert_findings(&validate(vec![overview, l2()]), &[]);
}

#[test]
fn reply_in_one_case_of_an_alt_with_else_keeps_the_window_open() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::AltElse(vec![S::Reply("comp", "app")], vec![]),
            S::Call("comp", "platform", "Log()"),
            S::Reply("comp", "app"),
        ],
    );

    assert_findings(
        &validate(vec![overview, without_log()]),
        &["ZoomExitMissing"],
    );
}

#[test]
fn reply_in_a_loop_or_break_does_not_close_the_window() {
    for wrap in [S::Loop, S::Break] {
        let overview = diagram(
            "overview",
            vec![
                S::Call("app", "comp", "Get()"),
                wrap(vec![S::Reply("comp", "app")]),
                S::Call("comp", "platform", "Log()"),
                S::Reply("comp", "app"),
            ],
        );

        assert_findings(
            &validate(vec![overview, without_log()]),
            &["ZoomExitMissing"],
        );
    }
}

#[test]
fn reply_in_one_parallel_branch_closes_the_window() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Par(
                vec![S::Call("comp", "platform", "Log()")],
                vec![S::Reply("comp", "app")],
            ),
            S::Call("comp", "log", "Late()"),
        ],
    );

    assert_findings(&validate(vec![overview, l2()]), &[]);
}

#[test]
fn entry_inside_a_case_skips_the_other_cases() {
    let overview = diagram(
        "overview",
        vec![S::AltElse(
            vec![
                S::Call("app", "comp", "Get()"),
                S::Call("comp", "platform", "Log()"),
                S::Reply("comp", "app"),
            ],
            vec![S::Call("comp", "log", "Flush()")],
        )],
    );

    assert_findings(&validate(vec![overview, l2()]), &[]);
}

#[test]
fn deactivate_of_a_nested_activation_does_not_close_the_window() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Activate("comp"),
            S::Call("comp", "comp", "Self()"),
            S::Activate("comp"),
            S::Deactivate("comp"),
            S::Call("comp", "platform", "Log()"),
            S::Reply("comp", "app"),
        ],
    );

    assert_findings(
        &validate(vec![overview, without_log()]),
        &["ZoomExitMissing"],
    );
}

#[test]
fn reply_of_a_child_to_its_own_caller_does_not_close_the_entry() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Call("app", "u1", "Peek()"),
            S::Reply("u1", "app"),
            S::Call("comp", "platform", "Log()"),
            S::Reply("comp", "app"),
        ],
    );

    assert_findings(
        &validate(vec![overview, without_log()]),
        &["ZoomExitMissing"],
    );
}

#[test]
fn reports_detail_that_is_never_closed() {
    let open = diagram(
        "open",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "platform", "Log()"),
        ],
    );

    assert_findings(&validate(vec![l1(), open]), &["ZoomWindowNotClosed"]);
}

#[test]
fn reports_the_refined_container_in_a_detail() {
    let detail = diagram(
        "detail",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "platform", "Log()"),
            S::Call("u1", "comp", "Up()"),
            S::Reply("u1", "app"),
        ],
    );

    let result = validate(vec![l1(), detail]);

    assert_findings(&result, &["ZoomContainerInDetail"]);
    assert!(!result.failures[0].contains("is not inside"));
}

#[test]
fn call_that_does_not_lead_its_diagram_is_no_detail() {
    let scenario = diagram(
        "scenario",
        vec![
            S::Call("app", "other", "Prepare()"),
            S::Reply("other", "app"),
            S::Call("app", "u1", "Get()"),
            S::Reply("u1", "app"),
        ],
    );

    let index = index_of(vec![l1(), scenario]);

    assert!(index.details[position(&index, "sl.comp")].is_empty());
    assert_findings(&validate_sequence_refinement(&index), &[]);
}

fn with_source(tree: SequenceTree) -> SequenceTree {
    SequenceTree {
        source: Some(format!("{}.puml", tree.name.clone().unwrap())),
        ..tree
    }
}

#[test]
fn diagram_given_twice_counts_once() {
    let detail = with_source(l1());

    let index = index_of(vec![l0(), detail.clone(), detail]);

    assert_eq!(index.details[position(&index, "sl")].len(), 1);
    assert_findings(&validate_sequence_refinement(&index), &[]);
}

#[test]
fn call_is_not_refined_by_its_own_diagram() {
    let both = diagram(
        "both",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Reply("comp", "app"),
            S::Call("app", "sl", "Get()"),
            S::Reply("sl", "app"),
        ],
    );

    let index = index_of(vec![both]);

    assert!(index.details[position(&index, "sl")].is_empty());
}

#[test]
fn exit_to_an_ancestor_does_not_allow_its_other_children() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Call("comp", "sl", "Callback()"),
            S::Reply("comp", "app"),
        ],
    );
    let ok = diagram(
        "ok",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "sl", "Callback()"),
            S::Reply("u1", "app"),
        ],
    );
    let sibling = diagram(
        "sibling",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "sl", "Callback()"),
            S::Call("u1", "u9", "Lookup()"),
            S::Reply("u1", "app"),
        ],
    );

    assert_findings(&validate(vec![overview.clone(), ok]), &[]);

    let result = validate(vec![overview, sibling]);

    assert_findings(&result, &["ZoomParticipantOutsideContainer"]);
    assert!(result.failures[0].contains("\"sl.other.u9\""));
}

#[test]
fn deactivate_of_an_earlier_activation_ends_the_window() {
    let overview = diagram(
        "overview",
        vec![
            S::Call("app", "comp", "Prepare()"),
            S::Activate("comp"),
            S::Call("app", "comp", "Get()"),
            S::Deactivate("comp"),
            S::Call("comp", "platform", "Log()"),
            S::Reply("comp", "app"),
        ],
    );
    let detail = diagram(
        "detail",
        vec![S::Call("app", "u1", "Get()"), S::Reply("u1", "app")],
    );

    assert_findings(&validate(vec![overview, detail]), &[]);
}

#[test]
fn accepts_details_without_findings() {
    let index = index_of(vec![l0(), l1(), l2()]);
    let check = check_sequence_refinement(&index);
    let (sl, comp, u1) = (
        position(&index, "sl"),
        position(&index, "sl.comp"),
        position(&index, "sl.comp.u1"),
    );

    assert!(check.accepts(sl, comp));
    assert!(check.accepts(comp, u1));
}

#[test]
fn rejects_the_detail_that_leaves_the_container() {
    let outside = diagram(
        "outside",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "platform", "Log()"),
            S::Call("u1", "u9", "Lookup()"),
            S::Reply("u1", "app"),
        ],
    );
    let index = index_of(vec![l1(), l2(), outside]);
    let check = check_sequence_refinement(&index);
    let comp = position(&index, "sl.comp");
    let [ok, bad] = index.details[comp][..] else {
        panic!("expected two details: {:?}", index.details[comp]);
    };

    assert_eq!(index.entries[ok].diagram, "l2");
    assert!(check.accepts(comp, ok));
    assert!(!check.accepts(comp, bad));
}

#[test]
fn rejects_all_details_of_a_call_with_a_missing_exit() {
    let index = index_of(vec![l0(), without_log_in_comp()]);
    let check = check_sequence_refinement(&index);
    let (sl, comp) = (position(&index, "sl"), position(&index, "sl.comp"));

    assert!(!check.accepts(sl, comp));
}

#[test]
fn rejects_entry_and_detail_with_an_unclosed_window() {
    let open_entry = diagram(
        "open_entry",
        vec![
            S::Call("app", "comp", "Get()"),
            S::Call("comp", "platform", "Log()"),
        ],
    );
    let open_detail = diagram(
        "open_detail",
        vec![
            S::Call("app", "u1", "Get()"),
            S::Call("u1", "platform", "Log()"),
        ],
    );

    let index = index_of(vec![open_entry, l2()]);
    let check = check_sequence_refinement(&index);
    assert!(!check.accepts(position(&index, "sl.comp"), position(&index, "sl.comp.u1")));

    let index = index_of(vec![l1(), open_detail]);
    let check = check_sequence_refinement(&index);
    assert!(!check.accepts(position(&index, "sl.comp"), position(&index, "sl.comp.u1")));
}

fn without_log_in_comp() -> SequenceTree {
    diagram(
        "no_log",
        vec![S::Call("app", "comp", "Get()"), S::Reply("comp", "app")],
    )
}
