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

use sequence_logic::{
    Block, Branch, BranchCase, EarlyExit, Loop, Node, Parallel, ParallelBranch, SourceLocation,
};
use sequence_parser::{GroupElse, GroupEnd, GroupKind, GroupStart};

use crate::error::SequenceResolverError;

struct GroupContext {
    kind: GroupKind,
    source_location: SourceLocation,
    mode: GroupMode,
}

enum GroupMode {
    Loop(Loop),
    Break(EarlyExit),
    Branch {
        node: Branch,
        active_case: usize,
    },
    Parallel {
        node: Parallel,
        active_branch: usize,
    },
    Transparent {
        block: Block,
    }, // group
    Ignored, // critical
}

pub(crate) struct SequenceTreeBuilder {
    root: Block,
    group_stack: Vec<GroupContext>,
}

impl SequenceTreeBuilder {
    pub(crate) fn new() -> Self {
        Self {
            root: Block::default(),
            group_stack: Vec::new(),
        }
    }

    pub(crate) fn finish(self) -> Result<Block, SequenceResolverError> {
        if let Some(context) = self.group_stack.last() {
            return Err(SequenceResolverError::UnterminatedGroup {
                source_location: context.source_location.clone(),
            });
        }

        Ok(self.root)
    }

    pub(crate) fn push(&mut self, node: Node) {
        let Some(context) = self.group_stack.last_mut() else {
            self.root.items.push(node);
            return;
        };

        match &mut context.mode {
            GroupMode::Loop(loop_node) => loop_node.block.items.push(node),
            GroupMode::Break(exit) => exit.block.items.push(node),
            GroupMode::Branch {
                node: branch,
                active_case,
            } => branch.cases[*active_case].block.items.push(node),
            GroupMode::Parallel {
                node: parallel,
                active_branch,
            } => parallel.branches[*active_branch].block.items.push(node),
            GroupMode::Transparent { block } => block.items.push(node),
            GroupMode::Ignored => {}
        }
    }

    pub(crate) fn start_group(&mut self, start: &GroupStart) {
        let mode = match start.kind {
            GroupKind::Loop => GroupMode::Loop(Loop {
                condition: start.label.clone(),
                block: Block::default(),
                source_location: start.source_location.clone(),
            }),
            GroupKind::Break => GroupMode::Break(EarlyExit {
                reason: start.label.clone(),
                block: Block::default(),
                source_location: start.source_location.clone(),
            }),
            GroupKind::Alt | GroupKind::Opt => GroupMode::Branch {
                node: Branch {
                    cases: vec![BranchCase {
                        condition: start.label.clone(),
                        block: Block::default(),
                        source_location: start.source_location.clone(),
                    }],
                },
                active_case: 0,
            },
            GroupKind::Par => GroupMode::Parallel {
                node: Parallel {
                    branches: vec![ParallelBranch {
                        label: start.label.clone(),
                        block: Block::default(),
                        source_location: start.source_location.clone(),
                    }],
                },
                active_branch: 0,
            },
            // A visual group still needs a frame so its matching `end` is
            // consumed and nested groups remain structurally balanced.
            GroupKind::Group => GroupMode::Transparent {
                block: Block::default(),
            },
            // Critical is parsed and validated, but its contents are not
            // represented in the current logic model.
            GroupKind::Critical => GroupMode::Ignored,
        };

        self.group_stack.push(GroupContext {
            kind: start.kind,
            source_location: start.source_location.clone(),
            mode,
        });
    }

    pub(crate) fn else_group(&mut self, command: &GroupElse) -> Result<(), SequenceResolverError> {
        let Some(context) = self.group_stack.last_mut() else {
            // Although PlantUML preview rejects a top-level else, the resolver still rejects it explicitly rather than silently ignoring it — as a safeguard.
            return Err(SequenceResolverError::ElseOutsideGroup {
                source_location: command.source_location.clone(),
            });
        };

        match &mut context.mode {
            GroupMode::Branch {
                node: branch,
                active_case,
            } => {
                if context.kind != GroupKind::Alt {
                    return Err(SequenceResolverError::ElseNotAllowedInGroup {
                        kind: context.kind,
                        source_location: command.source_location.clone(),
                    });
                }
                branch.cases.push(BranchCase {
                    condition: command.label.clone(),
                    block: Block::default(),
                    source_location: command.source_location.clone(),
                });
                *active_case = branch.cases.len() - 1;
            }
            GroupMode::Parallel {
                node: parallel,
                active_branch,
            } => {
                parallel.branches.push(ParallelBranch {
                    label: command.label.clone(),
                    block: Block::default(),
                    source_location: command.source_location.clone(),
                });
                *active_branch = parallel.branches.len() - 1;
            }
            GroupMode::Loop(_)
            | GroupMode::Break(_)
            | GroupMode::Transparent { .. }
            | GroupMode::Ignored => {
                return Err(SequenceResolverError::ElseNotAllowedInGroup {
                    kind: context.kind,
                    source_location: command.source_location.clone(),
                });
            }
        }

        Ok(())
    }

    pub(crate) fn end_group(&mut self, end: &GroupEnd) -> Result<(), SequenceResolverError> {
        let Some(context) = self.group_stack.pop() else {
            return Ok(());
        };

        if let Some(found) = end.kind {
            if found != context.kind {
                return Err(SequenceResolverError::MismatchedGroupEnd {
                    expected: context.kind,
                    found,
                    source_location: end.source_location.clone(),
                });
            }
        }

        match context.mode {
            GroupMode::Loop(loop_node) => self.push(Node::Loop(loop_node)),
            GroupMode::Break(exit) => self.push(Node::EarlyExit(exit)),
            GroupMode::Branch { node: branch, .. } => self.push(Node::Branch(branch)),
            GroupMode::Parallel { node: parallel, .. } => self.push(Node::Parallel(parallel)),
            GroupMode::Transparent { block } => {
                for node in block.items {
                    self.push(node);
                }
            }
            GroupMode::Ignored => {}
        }

        Ok(())
    }
}
