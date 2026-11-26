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

//! Refinement links between sequence diagrams.
//!
//! A call into an architecture element `C` (an entry) is refined by calls with
//! the same sender and method into a direct child of `C`, or by an expanded
//! call into `C`. See `docs/specifications/sequence_refinement.md`.

use std::collections::{HashMap, HashSet};

use super::architecture_tree::ArchitectureTree;
use super::{extract_method_name, ComponentDiagramArchitecture, SequenceDiagramIndex, GATE_LABEL};
use sequence_logic::{
    Block, LifecycleAction, MessageKind, Node, ParticipantId, SequenceTree, SourceLocation,
};

/// What a sequence participant stands for.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Lifeline {
    /// Id of an architecture element (SEooC, component or unit).
    Element(String),
    /// An actor, an `<<external>>` participant (by uid) or a gate.
    Environment(String),
    /// A participant that is not exactly one architecture element.
    Unresolved(String),
}

impl Lifeline {
    pub fn label(&self) -> &str {
        match self {
            Lifeline::Element(label)
            | Lifeline::Environment(label)
            | Lifeline::Unresolved(label) => label,
        }
    }
}

/// A call inside the window of an entry.
#[derive(Clone, Debug)]
pub struct WindowCall {
    pub from: Lifeline,
    pub to: Lifeline,
    pub method: String,
    /// `->>`: the sender does not wait for a reply.
    pub asynchronous: bool,
    pub source_location: SourceLocation,
}

/// A call `sender -> container : method` from outside into an architecture
/// element, with the calls handled while the container works on it.
#[derive(Clone, Debug)]
pub struct ZoomEntry {
    pub diagram: String,
    /// Source path of the diagram file.
    pub source: Option<String>,
    pub sender: Lifeline,
    /// Id of the architecture element that receives the call.
    pub container: String,
    pub method: String,
    /// `->>`: needs no reply; without one the window runs to the end of the
    /// diagram.
    pub asynchronous: bool,
    pub source_location: SourceLocation,
    /// Calls between the entry and the end of its window.
    pub calls: Vec<WindowCall>,
    /// The window ends with a reply to the sender or a `deactivate`.
    pub closed: bool,
    /// The call is the first call of its diagram. Only such a call can be the
    /// detail of another call.
    pub leads_diagram: bool,
}

/// Text of the `ref over <receiver>` that ends an asynchronous window.
const WINDOW_END_MARKER: &str = "score-window-end";

/// How the cases of a group continue the path through the diagram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    /// `alt` with `else`: one case runs.
    Alternatives,
    /// `opt`, `alt` without `else`, `loop`, `break`: may be skipped.
    Optional,
    /// `par`: every branch runs.
    Parallel,
}

enum Item {
    Call(WindowCall),
    Reply {
        from: Lifeline,
        to: Lifeline,
    },
    Activate(Lifeline),
    Deactivate(Lifeline),
    WindowEnd(Option<Lifeline>, SourceLocation),
    Open(Group),
    /// Start of the next case of the open group.
    Case,
    Close,
}

/// Entries of all sequence diagrams and the details that refine them.
pub struct SequenceRefinementIndex {
    tree: ArchitectureTree,
    pub entries: Vec<ZoomEntry>,
    /// Per entry: indices of its details, zoom and same-level.
    pub details: Vec<Vec<usize>>,
    /// Per entry: it is a zoom detail of another entry.
    pub is_detail: Vec<bool>,
    /// Per entry: indices of its zoom details.
    zoom_details: Vec<Vec<usize>>,
    /// Asynchronous entries that overlap another entry to the same receiver.
    pub overlapping_async: Vec<usize>,
    /// `score-window-end` references that end no asynchronous entry.
    pub invalid_async_boundaries: Vec<InvalidWindowEnd>,
}

/// A `score-window-end` reference that ends no asynchronous entry.
pub struct InvalidWindowEnd {
    pub diagram: String,
    pub source_location: SourceLocation,
}

/// An entry of one diagram, as needed to find overlapping asynchronous windows.
struct DiagramEntry {
    position: usize,
    /// Index of the call among the items of the diagram.
    start: usize,
    /// Index of the last window end of the entry among the items.
    end: Option<usize>,
    /// Indices of calls into the container made by elements the window called.
    callbacks: HashSet<usize>,
}

impl SequenceRefinementIndex {
    pub fn build(
        diagrams: &[SequenceTree],
        sequence: &SequenceDiagramIndex,
        architecture: &ComponentDiagramArchitecture,
    ) -> Self {
        let tree = ArchitectureTree::builder()
            .component_diagrams(architecture)
            .build();
        let lifeline = |participant: Option<&ParticipantId>| match participant {
            None => Lifeline::Environment(GATE_LABEL.to_string()),
            Some(uid) if is_environment(sequence, uid) => Lifeline::Environment(uid.to_string()),
            Some(uid) => match tree.resolve(uid) {
                Some(id) => Lifeline::Element(id.to_string()),
                None => Lifeline::Unresolved(uid.to_string()),
            },
        };

        let mut entries = Vec::new();
        let mut same_level_windows = Vec::new();
        let mut overlapping_async = Vec::new();
        let mut invalid_async_boundaries = Vec::new();
        // The same file may come with several architectural designs.
        let mut seen: HashSet<(&str, &str)> = HashSet::new();
        for diagram in diagrams {
            let name = diagram.name.as_deref().unwrap_or_default();
            if let Some(source) = diagram.source.as_deref() {
                if !seen.insert((source, name)) {
                    continue;
                }
            }
            let mut items = Vec::new();
            flatten(&diagram.root, &lifeline, &mut items);

            let mut enclosing: Vec<Group> = Vec::new();
            let mut first_call = true;
            let mut diagram_entries = Vec::new();
            let mut used_boundaries = HashSet::new();
            for (index, item) in items.iter().enumerate() {
                match item {
                    Item::Open(group) => enclosing.push(*group),
                    Item::Close => {
                        enclosing.pop();
                    }
                    Item::Call(call) => {
                        let leads_diagram = std::mem::take(&mut first_call);
                        let Lifeline::Element(container) = &call.to else {
                            continue;
                        };
                        if within(&tree, &call.from, container) {
                            continue;
                        }
                        let entry = Entry {
                            diagram: name,
                            source: diagram.source.as_deref(),
                            call,
                            container,
                            leads_diagram,
                        };
                        let following = &items[index + 1..];
                        let normal = window(&tree, &entry, enclosing.clone(), following, false);
                        let (same_level, boundaries, callbacks) = if call.asynchronous {
                            let bounded = window(&tree, &entry, enclosing.clone(), following, true);
                            (bounded.entry, bounded.boundaries, bounded.callbacks)
                        } else {
                            (normal.entry.clone(), Vec::new(), Vec::new())
                        };
                        let absolute = |offset: &usize| index + 1 + offset;
                        used_boundaries.extend(boundaries.iter().map(absolute));
                        diagram_entries.push(DiagramEntry {
                            position: entries.len(),
                            start: index,
                            end: boundaries.iter().map(absolute).max(),
                            callbacks: callbacks.iter().map(absolute).collect(),
                        });
                        entries.push(normal.entry);
                        same_level_windows.push(same_level);
                    }
                    _ => {}
                }
            }
            for (position, item) in items.iter().enumerate() {
                if let Item::WindowEnd(_, source) = item {
                    if !used_boundaries.contains(&position) {
                        invalid_async_boundaries.push(InvalidWindowEnd {
                            diagram: name.to_string(),
                            source_location: source.clone(),
                        });
                    }
                }
            }
            for entry in &diagram_entries {
                let Some(end) = entry.end else {
                    continue;
                };
                let container = &entries[entry.position].container;
                let inside: Vec<&DiagramEntry> = diagram_entries
                    .iter()
                    .filter(|other| {
                        other.start > entry.start
                            && other.start <= end
                            && !entry.callbacks.contains(&other.start)
                            && entries[other.position].container == *container
                    })
                    .collect();
                if inside.is_empty() {
                    continue;
                }
                overlapping_async.push(entry.position);
                overlapping_async.extend(
                    inside
                        .iter()
                        .filter(|other| entries[other.position].asynchronous)
                        .map(|other| other.position),
                );
            }
        }

        overlapping_async.sort_unstable();
        overlapping_async.dedup();
        let (details, zoom_details) =
            details_of(&tree, &entries, &same_level_windows, &overlapping_async);
        let mut is_detail = vec![false; entries.len()];
        for detail in zoom_details.iter().flatten() {
            is_detail[*detail] = true;
        }

        Self {
            tree,
            entries,
            details,
            is_detail,
            zoom_details,
            overlapping_async,
            invalid_async_boundaries,
        }
    }

    /// The lifeline is `container` or one of its descendants.
    pub fn within(&self, lifeline: &Lifeline, container: &str) -> bool {
        within(&self.tree, lifeline, container)
    }

    /// The lifeline is the exit partner `partner` or inside it. An ancestor of
    /// `container` counts as itself only, else it would take in the siblings
    /// of `container`.
    pub fn within_partner(&self, lifeline: &Lifeline, partner: &str, container: &str) -> bool {
        if self.tree.is_inside(container, partner) {
            matches!(lifeline, Lifeline::Element(id) if id == partner)
        } else {
            within(&self.tree, lifeline, partner)
        }
    }

    /// The lifeline is a descendant of `container`.
    pub fn inside(&self, lifeline: &Lifeline, container: &str) -> bool {
        matches!(lifeline, Lifeline::Element(id) if self.tree.is_inside(id, container))
    }

    /// The entry refines or is refined by a zoom detail.
    pub fn is_zoom_active(&self, entry: usize) -> bool {
        self.is_detail[entry] || !self.zoom_details[entry].is_empty()
    }

    pub fn zoom_details(&self, entry: usize) -> &[usize] {
        &self.zoom_details[entry]
    }

    /// Calls of the window that leave the container for an architecture
    /// element outside of it or for the environment.
    pub fn exits<'a>(&'a self, entry: &'a ZoomEntry) -> impl Iterator<Item = &'a WindowCall> {
        entry.calls.iter().filter(|call| {
            within(&self.tree, &call.from, &entry.container)
                && match &call.to {
                    Lifeline::Environment(_) => true,
                    Lifeline::Element(id) => {
                        id != &entry.container && !self.tree.is_inside(id, &entry.container)
                    }
                    Lifeline::Unresolved(_) => false,
                }
        })
    }
}

/// Role of an entry window in same-level matching.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SameLevel {
    /// Closed window without calls from the container or its descendants.
    Overview,
    /// Closed window with such calls.
    Detail,
    /// Unclosed or overlapping window.
    Excluded,
}

fn same_level_role(
    tree: &ArchitectureTree,
    entry: &ZoomEntry,
    window: &ZoomEntry,
    overlapping: bool,
) -> SameLevel {
    if !window.closed || overlapping {
        SameLevel::Excluded
    } else if window
        .calls
        .iter()
        .any(|call| within(tree, &call.from, &entry.container))
    {
        SameLevel::Detail
    } else {
        SameLevel::Overview
    }
}

/// Find zoom and same-level details that lead a different diagram. Returns all
/// details and the zoom details per entry.
fn details_of(
    tree: &ArchitectureTree,
    entries: &[ZoomEntry],
    same_level_windows: &[ZoomEntry],
    overlapping_async: &[usize],
) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let roles: Vec<SameLevel> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            same_level_role(
                tree,
                entry,
                &same_level_windows[index],
                overlapping_async.binary_search(&index).is_ok(),
            )
        })
        .collect();

    let mut by_parent: HashMap<(&Lifeline, &str, &str), Vec<usize>> = HashMap::new();
    let mut by_receiver: HashMap<(&Lifeline, &str, &str, bool), Vec<usize>> = HashMap::new();
    for (index, entry) in entries.iter().enumerate() {
        if !entry.leads_diagram {
            continue;
        }
        if let Some(parent) = tree.parent(&entry.container) {
            by_parent
                .entry((&entry.sender, &entry.method, parent))
                .or_default()
                .push(index);
        }
        if roles[index] == SameLevel::Detail
            && !entry.method.is_empty()
            && !matches!(&entry.sender, Lifeline::Unresolved(_))
        {
            by_receiver
                .entry((
                    &entry.sender,
                    &entry.method,
                    &entry.container,
                    entry.asynchronous,
                ))
                .or_default()
                .push(index);
        }
    }

    let pairs: Vec<_> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let zoom: Vec<usize> = by_parent
                .get(&(
                    &entry.sender,
                    entry.method.as_str(),
                    entry.container.as_str(),
                ))
                .into_iter()
                .flatten()
                .copied()
                .filter(|&detail| !same_diagram(entry, &entries[detail]))
                .collect();
            let mut details = zoom.clone();
            if roles[index] == SameLevel::Overview {
                details.extend(
                    by_receiver
                        .get(&(
                            &entry.sender,
                            entry.method.as_str(),
                            entry.container.as_str(),
                            entry.asynchronous,
                        ))
                        .into_iter()
                        .flatten()
                        .copied()
                        .filter(|&detail| !same_diagram(entry, &entries[detail])),
                );
            }
            (details, zoom)
        })
        .collect();
    pairs.into_iter().unzip()
}

fn same_diagram(a: &ZoomEntry, b: &ZoomEntry) -> bool {
    a.diagram == b.diagram && a.source == b.source
}

fn is_environment(sequence: &SequenceDiagramIndex, uid: &str) -> bool {
    sequence
        .participants()
        .get(uid)
        .is_some_and(|info| info.environment.is_some())
}

fn within(tree: &ArchitectureTree, lifeline: &Lifeline, container: &str) -> bool {
    matches!(lifeline, Lifeline::Element(id) if id == container || tree.is_inside(id, container))
}

/// Source-ordered calls, replies and activations; groups are marked by
/// `Open`, `Case` and `Close`.
fn flatten(
    block: &Block,
    lifeline: &impl Fn(Option<&ParticipantId>) -> Lifeline,
    items: &mut Vec<Item>,
) {
    for node in &block.items {
        match node {
            Node::Interaction(interaction) => {
                let from = lifeline(interaction.sender.as_ref());
                let to = lifeline(interaction.receiver.as_ref());
                match interaction.kind {
                    MessageKind::SynchCall | MessageKind::AsynchCall => {
                        items.push(Item::Call(WindowCall {
                            from,
                            to,
                            method: extract_method_name(
                                interaction.message.as_deref().unwrap_or_default(),
                            )
                            .to_string(),
                            asynchronous: interaction.kind == MessageKind::AsynchCall,
                            source_location: interaction.source_location.clone(),
                        }));
                    }
                    MessageKind::Reply => items.push(Item::Reply { from, to }),
                    MessageKind::Create | MessageKind::Delete => {}
                }
            }
            Node::Branch(branch) => {
                let group = if branch.cases.len() > 1 {
                    Group::Alternatives
                } else {
                    Group::Optional
                };
                items.push(Item::Open(group));
                for (index, case) in branch.cases.iter().enumerate() {
                    if index > 0 {
                        items.push(Item::Case);
                    }
                    flatten(&case.block, lifeline, items);
                }
                items.push(Item::Close);
            }
            Node::Loop(loop_node) => {
                items.push(Item::Open(Group::Optional));
                flatten(&loop_node.block, lifeline, items);
                items.push(Item::Close);
            }
            Node::Parallel(parallel) => {
                items.push(Item::Open(Group::Parallel));
                for (index, branch) in parallel.branches.iter().enumerate() {
                    if index > 0 {
                        items.push(Item::Case);
                    }
                    flatten(&branch.block, lifeline, items);
                }
                items.push(Item::Close);
            }
            Node::EarlyExit(early_exit) => {
                items.push(Item::Open(Group::Optional));
                flatten(&early_exit.block, lifeline, items);
                items.push(Item::Close);
            }
            Node::Lifecycle(lifecycle) => match lifecycle.action {
                LifecycleAction::Activate => {
                    items.push(Item::Activate(lifeline(Some(&lifecycle.participant))));
                }
                LifecycleAction::Deactivate => {
                    items.push(Item::Deactivate(lifeline(Some(&lifecycle.participant))));
                }
                LifecycleAction::Create | LifecycleAction::Destroy => {}
            },
            Node::Reference(reference)
                if reference.text.as_deref().map(str::trim) == Some(WINDOW_END_MARKER) =>
            {
                let participant = (reference.participants.len() == 1)
                    .then(|| lifeline(reference.participants.first()));
                items.push(Item::WindowEnd(
                    participant,
                    reference.source_location.clone(),
                ));
            }
            Node::Reference(_) => {}
        }
    }
}

/// An entry call and the diagram it is in.
struct Entry<'a> {
    diagram: &'a str,
    source: Option<&'a str>,
    call: &'a WindowCall,
    container: &'a str,
    leads_diagram: bool,
}

/// A call into the container that is not replied to yet.
#[derive(Clone, Copy)]
struct Pending<'a> {
    caller: &'a Lifeline,
    callee: &'a Lifeline,
    /// The call activated its callee (`++` or `activate` right after it).
    activated: bool,
}

/// Pending replies at the start of a group and at the end of each case.
struct Frame<'a> {
    group: Group,
    base: Vec<Pending<'a>>,
    ends: Vec<Vec<Pending<'a>>>,
}

impl<'a> Frame<'a> {
    /// Pending replies after the group, as in the parser: an alternative that
    /// did not reply keeps the path open (the case with most pending replies
    /// wins), a parallel branch that replied closes it (the branch with fewest
    /// pending replies wins), an optional group changes nothing.
    fn merge(self) -> Vec<Pending<'a>> {
        match self.group {
            Group::Optional => self.base,
            Group::Alternatives => self
                .ends
                .into_iter()
                .reduce(|a, b| if b.len() > a.len() { b } else { a })
                .unwrap_or_default(),
            Group::Parallel => self
                .ends
                .into_iter()
                .reduce(|a, b| if b.len() < a.len() { b } else { a })
                .unwrap_or(self.base),
        }
    }
}

/// The window of an entry. `boundaries` and `callbacks` are offsets into the
/// items after the entry.
struct Window {
    entry: ZoomEntry,
    /// Window ends over the receiver (only with `async_boundary`).
    boundaries: Vec<usize>,
    /// Calls into the container from elements that the window called.
    callbacks: Vec<usize>,
}

/// Follows an entry to its reply/deactivation, or with `async_boundary` to its
/// `score-window-end`. Other cases of an enclosing `alt` are not part of the
/// window.
fn window(
    tree: &ArchitectureTree,
    entry: &Entry<'_>,
    mut enclosing: Vec<Group>,
    following: &[Item],
    async_boundary: bool,
) -> Window {
    let container = entry.container;
    let receiver = &entry.call.to;
    let mut calls = Vec::new();
    let mut pending = vec![Pending {
        caller: &entry.call.from,
        callee: receiver,
        activated: false,
    }];
    let mut frames: Vec<Frame<'_>> = Vec::new();
    let mut skipping: Option<usize> = None;
    let mut nested_activations = 0usize;
    // Receiver of a call that just became pending: its activation is that call's.
    let mut activated_by_call = Some(receiver);
    // Callee whose activation the reply just before this item ended, as in
    // `C --> Y --`: the `deactivate` that follows belongs to that call.
    let mut replied_activation: Option<&Lifeline> = None;
    let mut closed = false;
    let mut boundaries = Vec::new();
    let mut callbacks = Vec::new();
    let mut called: Vec<&Lifeline> = Vec::new();

    for (offset, item) in following.iter().enumerate() {
        if let Some(depth) = skipping.as_mut() {
            match item {
                Item::Open(_) => *depth += 1,
                Item::Close if *depth == 0 => {
                    skipping = None;
                    enclosing.pop();
                }
                Item::Close => *depth -= 1,
                _ => {}
            }
            continue;
        }

        let activated = activated_by_call.take();
        let replied = replied_activation.take();
        match item {
            Item::Open(group) => frames.push(Frame {
                group: *group,
                base: pending.clone(),
                ends: Vec::new(),
            }),
            Item::Case => match frames.last_mut() {
                Some(frame) => {
                    let end = std::mem::replace(&mut pending, frame.base.clone());
                    frame.ends.push(end);
                }
                None if enclosing.last() != Some(&Group::Parallel) => skipping = Some(0),
                None => {}
            },
            Item::Close => match frames.pop() {
                Some(mut frame) => {
                    frame.ends.push(std::mem::take(&mut pending));
                    pending = frame.merge();
                }
                None => {
                    enclosing.pop();
                }
            },
            // The path ended: the rest of its case is after the window.
            _ if pending.is_empty() => {}
            Item::Call(call) => {
                calls.push(call.clone());
                if within(tree, &call.from, container) {
                    called.push(&call.to);
                } else if within(tree, &call.to, container) && called.contains(&&call.from) {
                    callbacks.push(offset);
                }
                if !call.asynchronous
                    && within(tree, &call.to, container)
                    && !within(tree, &call.from, container)
                {
                    pending.push(Pending {
                        caller: &call.from,
                        callee: &call.to,
                        activated: false,
                    });
                    activated_by_call = Some(&call.to);
                }
            }
            Item::Reply { from, to } => {
                if within(tree, from, container)
                    && pending.last().is_some_and(|call| call.caller == to)
                    && (!async_boundary || pending.len() > 1)
                {
                    if let Some(answered) = pending.pop().filter(|call| call.activated) {
                        replied_activation = Some(answered.callee);
                    }
                }
            }
            Item::Activate(participant) => {
                if activated == Some(participant) {
                    if let Some(call) = pending.last_mut().filter(|c| c.callee == participant) {
                        call.activated = true;
                    }
                } else if participant == receiver {
                    nested_activations += 1;
                }
            }
            Item::Deactivate(participant) => {
                if replied == Some(participant) {
                    // Ends the activation of the call that was just replied to.
                } else if participant == receiver && nested_activations > 0 {
                    nested_activations -= 1;
                } else if let Some(index) = pending.iter().rposition(|c| c.callee == participant) {
                    if index == 0 && !async_boundary {
                        pending.clear();
                    } else if index != 0 {
                        pending.remove(index);
                    }
                }
            }
            Item::WindowEnd(participant, _) => {
                if async_boundary && participant.as_ref() == Some(receiver) {
                    boundaries.push(offset);
                    pending.clear();
                }
            }
        }

        if pending.is_empty() && frames.is_empty() {
            closed = true;
            break;
        }
    }

    Window {
        entry: ZoomEntry {
            diagram: entry.diagram.to_string(),
            source: entry.source.map(str::to_string),
            sender: entry.call.from.clone(),
            container: container.to_string(),
            method: entry.call.method.clone(),
            asynchronous: entry.call.asynchronous,
            source_location: entry.call.source_location.clone(),
            calls,
            closed,
            leads_diagram: entry.leads_diagram,
        },
        boundaries,
        callbacks,
    }
}
