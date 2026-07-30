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
//! Logic parser module for constructing and visualizing sequence node trees

use sequence_logic::*;
use sequence_parser::*;

const EXTERNAL_ENDPOINT_NAME: &str = "ExternalEndpoint";

fn endpoint_name(endpoint: &MessageEndpoint) -> String {
    match endpoint {
        MessageEndpoint::Participant(identifier) => identifier
            .alias
            .as_deref()
            .unwrap_or(&identifier.display_name)
            .to_string(),
        MessageEndpoint::LostFound(_) => EXTERNAL_ENDPOINT_NAME.to_string(),
    }
}

/// Convert a syntax-level `GroupKind` into the metamodel `ConditionType`.
fn group_kind_to_condition(kind: &GroupKind) -> ConditionType {
    match kind {
        GroupKind::Opt => ConditionType::Opt,
        GroupKind::Alt => ConditionType::Alt,
        GroupKind::Loop => ConditionType::Loop,
        GroupKind::Par => ConditionType::Par,
        GroupKind::Break => ConditionType::Break,
        GroupKind::Critical => ConditionType::Critical,
        GroupKind::Group => ConditionType::Group,
    }
}

/// Build a tree of SequenceNodes from a list of statements
pub fn build_tree(statements: &[Statement]) -> Vec<SequenceNode> {
    let mut nodes = Vec::new();
    let mut i = 0;

    while i < statements.len() {
        if let Some((node, consumed)) = build_node(&statements[i..]) {
            nodes.push(node);
            i += consumed;
        } else {
            // Skip over branch/end markers that are not handled
            if let Some(Statement::GroupCmd(g)) = statements.get(i) {
                if matches!(g, GroupCmd::Else(_) | GroupCmd::End(_)) {
                    i += 1;
                    continue;
                }
            }
            i += 1;
        }
    }

    nodes
}

/// Helper function to box sequence nodes
pub(crate) fn box_nodes(nodes: Vec<SequenceNode>) -> Vec<SequenceNode> {
    nodes
}

fn is_group_node(group: &GroupCmd) -> bool {
    matches!(group, GroupCmd::Start(_) | GroupCmd::Else(_))
}

fn collect_group_statements(statements: &[Statement]) -> (Vec<Statement>, usize) {
    let mut group_statements = Vec::new();
    let mut consumed = 1;
    let mut nesting_depth = 0;

    for stmt in &statements[1..] {
        if let Statement::GroupCmd(group) = stmt {
            match group {
                GroupCmd::End(_) => {
                    if nesting_depth > 0 {
                        nesting_depth -= 1;
                        group_statements.push(stmt.clone());
                    } else {
                        break;
                    }
                }
                GroupCmd::Else(_) => {
                    if nesting_depth > 0 {
                        group_statements.push(stmt.clone());
                    } else {
                        break;
                    }
                }
                GroupCmd::Start(_) => {
                    nesting_depth += 1;
                    group_statements.push(stmt.clone());
                }
            }
        } else {
            group_statements.push(stmt.clone());
        }
        consumed += 1;
    }

    (group_statements, consumed)
}

fn build_group_node(statements: &[Statement], group: &GroupCmd) -> (SequenceNode, usize) {
    let (condition, source_location) = group_condition_and_location(group);
    let (group_statements, consumed) = collect_group_statements(statements);

    (
        SequenceNode {
            event: Event::Condition(condition),
            source_location,
            branches_node: box_nodes(build_tree(&group_statements)),
        },
        consumed,
    )
}

fn group_condition_and_location(group: &GroupCmd) -> (Condition, SourceLocation) {
    match group {
        GroupCmd::Start(start) => (
            Condition {
                condition_type: group_kind_to_condition(&start.kind),
                condition_value: start.label.clone().unwrap_or_default(),
            },
            start.source_location.clone(),
        ),
        GroupCmd::Else(else_cmd) => (
            Condition {
                condition_type: ConditionType::Else,
                condition_value: else_cmd.label.clone().unwrap_or_default(),
            },
            else_cmd.source_location.clone(),
        ),
        GroupCmd::End(end) => (
            Condition {
                condition_type: ConditionType::End,
                condition_value: String::new(),
            },
            end.source_location.clone(),
        ),
    }
}

/// Build a single sequence node and return how many statements were consumed
fn build_node(statements: &[Statement]) -> Option<(SequenceNode, usize)> {
    if statements.is_empty() {
        return None;
    }

    match &statements[0] {
        Statement::Message(msg) => {
            // Determine if this is an Interaction or Return based on arrow
            let event = message_to_event(msg)?;

            // For interactions, collect child nodes until we hit the matching return
            let mut branches = Vec::new();
            let mut consumed = 1;

            if let Event::Interaction(ref interaction) = event {
                // Look ahead for nested content and the matching return
                let caller = &interaction.caller;
                let callee = &interaction.callee;
                let mut found_return = false;
                let mut i = 1;

                while i < statements.len() {
                    match &statements[i] {
                        Statement::Message(m) => {
                            // Check if this is the matching return
                            if is_return_arrow(m) {
                                if let Some(Event::Return(ret)) = message_to_event(m) {
                                    if &ret.caller == caller && &ret.callee == callee {
                                        // Found our return - add it as the last branch node
                                        branches.push(SequenceNode {
                                            event: Event::Return(ret),
                                            source_location: m.source_location.clone(),
                                            branches_node: Vec::new(),
                                        });
                                        consumed = i + 1;
                                        found_return = true;
                                        break;
                                    }
                                }
                            }

                            // Not our return, process it as a child node
                            if let Some((child_node, child_consumed)) = build_node(&statements[i..])
                            {
                                branches.push(child_node);
                                i += child_consumed;
                            } else {
                                i += 1;
                            }
                        }
                        Statement::GroupCmd(_group) => {
                            // Process branches (alt/else/opt/loop)
                            if let Some((branch_node, branch_consumed)) =
                                build_node(&statements[i..])
                            {
                                branches.push(branch_node);
                                i += branch_consumed;
                            } else {
                                i += 1;
                            }
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }

                // If no matching return found, we still consumed what we collected
                if !found_return {
                    consumed = i;
                }
            }

            Some((
                SequenceNode {
                    event,
                    source_location: msg.source_location.clone(),
                    branches_node: branches,
                },
                consumed,
            ))
        }
        Statement::GroupCmd(group) => {
            // Handle group commands (alt, opt, loop, else, etc.)
            match group {
                GroupCmd::End(_) => None, // End markers signal the close of a branch
                _ if is_group_node(group) => Some(build_group_node(statements, group)),
                _ => None,
            }
        }
        _ => None, // Skip non-message, non-group statements
    }
}

/// Convert a message statement to an Event (Interaction or Return)
fn message_to_event(msg: &Message) -> Option<Event> {
    let method = msg.description.clone().unwrap_or_default();

    // Check if arrow left decorator points left (reverse arrow like <--)
    let is_reverse = msg
        .arrow
        .left
        .as_ref()
        .map(|d| d.raw.contains("<"))
        .unwrap_or(false);

    // Determine actual caller and callee based on arrow direction.
    let (actual_from, actual_to) = if is_reverse {
        // Arrow points left: from right participant to left participant.
        // "A <-- B" means B sends to A.
        (endpoint_name(&msg.right), endpoint_name(&msg.left))
    } else {
        // Arrow points right: from left participant to right participant.
        // "A -> B" means A sends to B.
        (endpoint_name(&msg.left), endpoint_name(&msg.right))
    };

    // Check arrow type to determine Interaction vs Return.
    if is_return_arrow_from_arrow(&msg.arrow) {
        // For returns: actual_from is the sender (callee), actual_to is the receiver (caller).
        Some(Event::Return(Return {
            caller: actual_to,
            callee: actual_from,
            return_content: method,
        }))
    } else {
        Some(Event::Interaction(Interaction {
            caller: actual_from,
            callee: actual_to,
            method,
        }))
    }
}

/// Check if a message represents a return arrow
fn is_return_arrow(msg: &Message) -> bool {
    is_return_arrow_from_arrow(&msg.arrow)
}

/// Check if an arrow represents a return (dashed arrow)
fn is_return_arrow_from_arrow(arrow: &Arrow) -> bool {
    // Return arrows are typically dashed: "-->"
    arrow.line.raw.contains("--")
}

#[cfg(test)]
mod return_arrow_detection_tests {
    use super::*;
    use parser_core::common_ast::{Arrow, ArrowDecor, ArrowLine};

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

    /// "->" is a solid call arrow and must NOT be classified as a return.
    #[test]
    fn test_solid_call_arrow_is_not_return() {
        assert!(!is_return_arrow_from_arrow(&arrow("-", Some(">"))));
    }

    /// "-->" is a dashed return arrow and MUST be classified as a return.
    #[test]
    fn test_dashed_return_arrow_is_return() {
        assert!(is_return_arrow_from_arrow(&arrow("--", Some(">"))));
    }

    /// "->>" (solid with double-headed arrow) must NOT be classified as a return.
    #[test]
    fn test_solid_double_headed_arrow_is_not_return() {
        assert!(!is_return_arrow_from_arrow(&arrow("-", Some(">>"))));
    }

    /// "-->>" (dashed with double-headed arrow) MUST be classified as a return.
    #[test]
    fn test_dashed_double_headed_arrow_is_return() {
        assert!(is_return_arrow_from_arrow(&arrow("--", Some(">>"))));
    }
}
