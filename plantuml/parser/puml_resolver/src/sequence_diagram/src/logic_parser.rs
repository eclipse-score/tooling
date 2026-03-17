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

use crate::logic_ast::*;
use sequence_parser::*;

/// Build a tree of SequenceNodes from a list of statements
pub fn build_tree(statements: &[Statement]) -> Vec<SequenceNode> {
    let mut nodes = Vec::new();
    let mut i = 0;

    while i < statements.len() {
        if let Some((node, consumed)) = build_node(&statements[i..]) {
            nodes.push(node);
            i += consumed;
        } else {
            // Skip over else/also/end that are not handled
            if let Some(Statement::GroupCmd(g)) = statements.get(i) {
                if matches!(
                    g.group_type,
                    GroupType::Else | GroupType::Also | GroupType::End
                ) {
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
                    branches_node: branches,
                },
                consumed,
            ))
        }
        Statement::GroupCmd(group) => {
            // Handle group commands (alt, opt, loop, else, etc.)
            match group.group_type {
                GroupType::End => {
                    None // End markers signal the close of a branch
                }
                GroupType::Alt
                | GroupType::Opt
                | GroupType::Loop
                | GroupType::Par
                | GroupType::Par2
                | GroupType::Break
                | GroupType::Critical
                | GroupType::Group
                | GroupType::Else
                | GroupType::Also => {
                    // These start a new branch
                    let condition = Condition {
                        condition_type: group.group_type.clone(),
                        condition_value: group.text.clone().unwrap_or_default(),
                    };

                    // Collect statements until we hit else/also/end at the same nesting level
                    let mut branch_statements = Vec::new();
                    let mut consumed = 1;
                    let mut nesting_depth = 0;

                    for stmt in &statements[1..] {
                        if let Statement::GroupCmd(g) = stmt {
                            match g.group_type {
                                GroupType::Alt
                                | GroupType::Opt
                                | GroupType::Loop
                                | GroupType::Par
                                | GroupType::Par2
                                | GroupType::Break
                                | GroupType::Critical
                                | GroupType::Group => {
                                    // Entering a nested group
                                    nesting_depth += 1;
                                    branch_statements.push(stmt.clone());
                                }
                                GroupType::End => {
                                    if nesting_depth > 0 {
                                        // This is the end of a nested group
                                        nesting_depth -= 1;
                                        branch_statements.push(stmt.clone());
                                    } else {
                                        // This is the end of our group - stop here
                                        break;
                                    }
                                }
                                GroupType::Else | GroupType::Also => {
                                    if nesting_depth > 0 {
                                        // This else/also belongs to a nested group
                                        branch_statements.push(stmt.clone());
                                    } else {
                                        // This else/also belongs to our level - stop here
                                        break;
                                    }
                                }
                            }
                        } else {
                            branch_statements.push(stmt.clone());
                        }
                        consumed += 1;
                    }

                    Some((
                        SequenceNode {
                            event: Event::Condition(condition),
                            branches_node: box_nodes(build_tree(&branch_statements)),
                        },
                        consumed,
                    ))
                }
            }
        }
        _ => None, // Skip non-message, non-group statements
    }
}

/// Convert a message statement to an Event (Interaction or Return)
fn message_to_event(msg: &Message) -> Option<Event> {
    match &msg.content {
        MessageContent::WithTargets { left, arrow, right } => {
            let method = msg.description.clone().unwrap_or_default();

            // Check if arrow left decorator points left (reverse arrow like <--)
            let is_reverse = arrow
                .left
                .as_ref()
                .map(|d| d.raw.contains("<"))
                .unwrap_or(false);

            // Determine actual caller and callee based on arrow direction
            let (actual_from, actual_to) = if is_reverse {
                // Arrow points left: from right participant to left participant
                // "A <-- B" means B sends to A
                (right.clone(), left.clone())
            } else {
                // Arrow points right: from left participant to right participant
                // "A -> B" means A sends to B
                (left.clone(), right.clone())
            };

            // Check arrow type to determine Interaction vs Return
            if is_return_arrow_from_arrow(arrow) {
                // For returns: actual_from is the sender (callee), actual_to is the receiver (caller)
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
        } // Note: MessageContent only has WithTargets. Incomplete arrows (missing
          // left or right participant) are represented as WithTargets with an empty
          // string for the missing side, so no separate variant is needed.
    }
}

/// Check if a message represents a return arrow
fn is_return_arrow(msg: &Message) -> bool {
    match &msg.content {
        MessageContent::WithTargets { arrow, .. } => is_return_arrow_from_arrow(arrow),
    }
}

/// Check if an arrow represents a return (dashed arrow)
fn is_return_arrow_from_arrow(arrow: &Arrow) -> bool {
    // Return arrows are typically dashed: "-->"
    arrow.line.raw.contains("--")
}
