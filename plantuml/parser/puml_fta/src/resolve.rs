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

//! Attribution of root causes to failure modes across the diagrams of one
//! `fault_trees` target, and the completeness findings found on the way.
//!
//! The graph is built over nodes, not trees: every node points upward to its
//! `connection` parent, and a node a `$TransferInGate` transfers into also
//! points upward to that gate.  A root cause belongs to every failure mode
//! reachable on that upward graph.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::model::{is_valid_trlc_fqn, FtaError, FtaModel, FtaNode, NodeKind};

/// What a finding reports.  Findings never fail the build; they end up in the
/// traceability report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingKind {
    /// A `$FailureMode` covers a failure mode that no `failure_modes` target
    /// of the `fault_trees` target defines.
    UnknownFailureMode,
    /// A `$TransferInGate` target matches nothing.
    UnknownTransferTarget,
    /// A `$TransferInGate` shows other text than its target.
    TransferNameMismatch,
    /// A node connects to an alias that its diagram does not define.
    DanglingConnection,
    /// No `$TransferInGate` transfers into a `$TransferOutEvent`.
    UnreferencedTransferOutEvent,
    /// An event or gate has no node connected below it.
    UndevelopedEvent,
    /// One root cause carries different titles.
    RootCauseTitleMismatch,
}

impl FindingKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnknownFailureMode => "UnknownFailureMode",
            Self::UnknownTransferTarget => "UnknownTransferTarget",
            Self::TransferNameMismatch => "TransferNameMismatch",
            Self::DanglingConnection => "DanglingConnection",
            Self::UnreferencedTransferOutEvent => "UnreferencedTransferOutEvent",
            Self::UndevelopedEvent => "UndevelopedEvent",
            Self::RootCauseTitleMismatch => "RootCauseTitleMismatch",
        }
    }
}

/// One completeness finding at a source line of a diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FtaFinding {
    pub kind: FindingKind,
    /// Basename of the diagram.
    pub diagram: String,
    /// 1-based line of the macro call; 0 when unknown.
    pub line: usize,
    pub message: String,
}

/// One generated `RootCause` TRLC record (see [`crate::render_trlc_stub`]),
/// merged across every diagram of an `fta_package`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootCauseStub {
    /// TRLC record name within the generated `fta_package`.
    pub name: String,
    /// Title of the first occurrence (diagram order, then line).
    pub title: String,
    /// Basename of the diagram of the first occurrence.
    pub diagram: String,
    /// Source line of the first occurrence.
    pub line: usize,
    /// Fully-qualified names of the failure modes the root cause hangs under,
    /// sorted and de-duplicated.  Empty when it reaches none.
    pub failure_modes: Vec<String>,
    /// Per failure mode: the `$TransferInGate`s on the paths that reach it.
    /// A failure mode reached without a transfer has no entry.
    pub via: BTreeMap<String, BTreeSet<SourceRef>>,
}

/// A macro call of a diagram.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceRef {
    /// Basename of the diagram.
    pub diagram: String,
    /// 1-based line of the macro call; 0 when unknown.
    pub line: usize,
    /// Alias of the node.
    pub alias: String,
}

/// Result of [`resolve_root_causes`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolution {
    /// One entry per root-cause name, sorted by name.
    pub root_causes: Vec<RootCauseStub>,
    /// Sorted by diagram, line, kind.
    pub findings: Vec<FtaFinding>,
    /// The `$FailureMode` calls covering each known failure mode.
    pub failure_mode_trees: BTreeMap<String, BTreeSet<SourceRef>>,
}

fn describe(node: &FtaNode) -> String {
    format!("{} {}", node.kind.macro_name(), node.alias)
}

struct Graph<'a> {
    diagrams: &'a [(&'a str, &'a FtaModel)],
    /// Every node of every diagram as (diagram index, node).
    nodes: Vec<(usize, &'a FtaNode)>,
    by_alias: HashMap<(usize, &'a str), usize>,
    /// Per diagram: the `$FailureMode` above each node; computed on first use.
    roots: Vec<OnceCell<HashMap<&'a str, Vec<&'a FtaNode>>>>,
}

impl<'a> Graph<'a> {
    fn new(diagrams: &'a [(&'a str, &'a FtaModel)]) -> Self {
        let mut nodes = Vec::new();
        let mut by_alias = HashMap::new();
        for (d, (_, model)) in diagrams.iter().enumerate() {
            for node in &model.nodes {
                by_alias.insert((d, node.alias.as_str()), nodes.len());
                nodes.push((d, node));
            }
        }
        Self {
            diagrams,
            nodes,
            by_alias,
            roots: diagrams.iter().map(|_| OnceCell::new()).collect(),
        }
    }

    fn node(&self, n: usize) -> &'a FtaNode {
        self.nodes[n].1
    }

    fn diagram(&self, n: usize) -> &'a str {
        self.diagrams[self.nodes[n].0].0
    }

    fn line(&self, n: usize) -> usize {
        self.node(n).line
    }

    fn at(&self, n: usize) -> String {
        format!("{}:{}", self.diagram(n), self.line(n))
    }

    fn source_ref(&self, n: usize) -> SourceRef {
        SourceRef {
            diagram: self.diagram(n).to_string(),
            line: self.line(n),
            alias: self.node(n).alias.clone(),
        }
    }

    fn finding(&self, n: usize, kind: FindingKind, message: String) -> FtaFinding {
        FtaFinding {
            kind,
            diagram: self.diagram(n).to_string(),
            line: self.line(n),
            message,
        }
    }

    /// The `$FailureMode`s of the same diagram that `n` hangs under.
    fn root_failure_modes(&self, n: usize) -> &[&'a FtaNode] {
        let d = self.nodes[n].0;
        let roots = self.roots[d].get_or_init(|| self.diagrams[d].1.root_failure_modes());
        roots
            .get(self.node(n).alias.as_str())
            .map_or(&[], Vec::as_slice)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Unvisited,
    OnPath,
    Done,
}

/// The edges of the upward graph.
struct Edges {
    /// Per node: its `connection` parent, then every `$TransferInGate` that
    /// transfers into it.
    up: Vec<Vec<usize>>,
    /// Per node: the `$TransferInGate`s among `up`.
    transfers: Vec<Vec<usize>>,
    /// Per node: whether some node connects to it.
    has_child: Vec<bool>,
}

/// What the `$TransferInGate`s can transfer into.
struct Targets<'a> {
    /// `$TransferOutEvent` by alias.
    transfer_outs: HashMap<&'a str, usize>,
    /// `$FailureMode` nodes by every failure mode they cover.
    failure_mode_nodes: HashMap<&'a str, Vec<usize>>,
}

/// Failure modes per node, memoised over the upward graph.
struct Solver<'s, 'g> {
    graph: &'s Graph<'g>,
    edges: &'s Edges,
    state: Vec<State>,
    failure_modes: Vec<BTreeSet<String>>,
    /// Per node and failure mode: the transfer gates (node indices) passed.
    via: Vec<BTreeMap<String, BTreeSet<usize>>>,
    /// Depth-first stack: a node and the index of the next parent to visit.
    path: Vec<(usize, usize)>,
}

struct Solved {
    failure_modes: Vec<BTreeSet<String>>,
    via: Vec<BTreeMap<String, BTreeSet<usize>>>,
}

impl<'s, 'g> Solver<'s, 'g> {
    /// `own` are the failure modes each node covers itself.
    fn new(graph: &'s Graph<'g>, edges: &'s Edges, own: Vec<BTreeSet<String>>) -> Self {
        let count = own.len();
        Self {
            graph,
            edges,
            state: vec![State::Unvisited; count],
            failure_modes: own,
            via: vec![BTreeMap::new(); count],
            path: Vec::new(),
        }
    }

    /// Fails with a cycle through the upward graph.
    fn solve(mut self) -> Result<Solved, FtaError> {
        for start in 0..self.state.len() {
            if self.state[start] == State::Unvisited {
                self.walk(start)?;
            }
        }
        Ok(Solved {
            failure_modes: self.failure_modes,
            via: self.via,
        })
    }

    /// Iterative post-order walk from `start`; a chain of any length is fine.
    fn walk(&mut self, start: usize) -> Result<(), FtaError> {
        self.enter(start);
        while let Some(&(n, next)) = self.path.last() {
            let Some(&parent) = self.edges.up[n].get(next) else {
                self.state[n] = State::Done;
                self.path.pop();
                continue;
            };
            match self.state[parent] {
                State::Unvisited => self.enter(parent),
                State::OnPath => return Err(self.cycle(parent)),
                State::Done => {
                    self.absorb(n, parent);
                    if let Some(top) = self.path.last_mut() {
                        top.1 += 1;
                    }
                }
            }
        }
        Ok(())
    }

    fn enter(&mut self, n: usize) {
        self.state[n] = State::OnPath;
        self.path.push((n, 0));
    }

    /// `n` also covers the failure modes of its finished `parent`.
    fn absorb(&mut self, n: usize, parent: usize) {
        let inherited = self.failure_modes[parent].clone();
        let inherited_via = self.via[parent].clone();
        let transfers_in = self.edges.transfers[n].contains(&parent);
        for (failure_mode, gates) in inherited_via {
            self.via[n].entry(failure_mode).or_default().extend(gates);
        }
        if transfers_in {
            for failure_mode in &inherited {
                self.via[n]
                    .entry(failure_mode.clone())
                    .or_default()
                    .insert(parent);
            }
        }
        self.failure_modes[n].extend(inherited);
    }

    /// The error for reaching on-path node `back` again.
    fn cycle(&self, back: usize) -> FtaError {
        let from = self
            .path
            .iter()
            .position(|&(p, _)| p == back)
            .expect("an on-path node is on the path");
        let steps: Vec<String> = self.path[from..]
            .iter()
            .map(|&(p, _)| p)
            .chain(std::iter::once(back))
            .map(|p| format!("{} {}", self.graph.at(p), self.graph.node(p).alias))
            .collect();
        FtaError::Cycle {
            cycle: steps.join(" -> "),
        }
    }
}

/// Connect every node to the node its `connection` names.
fn connect(graph: &Graph, findings: &mut Vec<FtaFinding>) -> Edges {
    let count = graph.nodes.len();
    let mut edges = Edges {
        up: vec![Vec::new(); count],
        transfers: vec![Vec::new(); count],
        has_child: vec![false; count],
    };
    for n in 0..count {
        let node = graph.node(n);
        for connection in &node.connections {
            match graph.by_alias.get(&(graph.nodes[n].0, connection.as_str())) {
                Some(&parent) => {
                    edges.up[n].push(parent);
                    edges.has_child[parent] = true;
                }
                None => findings.push(graph.finding(
                    n,
                    FindingKind::DanglingConnection,
                    format!(
                        "{} connects to {connection:?}, which the diagram does not define",
                        describe(node)
                    ),
                )),
            }
        }
    }
    edges
}

/// Index what a `$TransferInGate` can transfer into.  Fails on a
/// `$TransferOutEvent` alias defined twice.
fn index_targets<'a>(graph: &Graph<'a>) -> Result<Targets<'a>, FtaError> {
    let mut targets = Targets {
        transfer_outs: HashMap::new(),
        failure_mode_nodes: HashMap::new(),
    };
    for n in 0..graph.nodes.len() {
        let node = graph.node(n);
        match &node.kind {
            NodeKind::TransferOutEvent => {
                if let Some(&first) = targets.transfer_outs.get(node.alias.as_str()) {
                    return Err(FtaError::DuplicateTransferOutEvent {
                        alias: node.alias.clone(),
                        first: graph.at(first),
                        second: graph.at(n),
                    });
                }
                targets.transfer_outs.insert(node.alias.as_str(), n);
            }
            NodeKind::FailureMode { failure_modes } => {
                for fqn in failure_modes {
                    targets
                        .failure_mode_nodes
                        .entry(fqn.as_str())
                        .or_default()
                        .push(n);
                }
            }
            _ => {}
        }
    }
    Ok(targets)
}

enum Lookup {
    Found(Vec<usize>),
    /// Why nothing matches.
    Missing(String),
}

/// The node of `failure_mode`'s diagram that a `Package.FailureMode.<alias>`
/// target names, or why it is none.
fn event_below(graph: &Graph, failure_mode: usize, alias: &str) -> Result<usize, String> {
    let fm_alias = &graph.node(failure_mode).alias;
    let diagram = graph.diagram(failure_mode);
    let Some(&e) = graph.by_alias.get(&(graph.nodes[failure_mode].0, alias)) else {
        return Err(format!(
            "{diagram} has no node {alias} for $FailureMode {fm_alias}"
        ));
    };
    let node = graph.node(e);
    if !matches!(node.kind, NodeKind::IntermediateEvent) {
        return Err(format!(
            "{} {alias} is a {}, not an $IntermediateEvent",
            graph.at(e),
            node.kind.macro_name()
        ));
    }
    let under_failure_mode = graph
        .root_failure_modes(e)
        .iter()
        .any(|root| root.alias == *fm_alias);
    if !under_failure_mode {
        return Err(format!(
            "$IntermediateEvent {alias} at {} does not hang under $FailureMode {fm_alias}",
            graph.at(e)
        ));
    }
    Ok(e)
}

/// Find what the `$TransferInGate` `gate` transfers into.  A name without a dot
/// is a `$TransferOutEvent` alias; a dotted one is looked up both as a
/// `$FailureMode` fully-qualified name and as `<$FailureMode>.<event alias>`.
///
/// A failure mode is a TRLC record, so every `$FailureMode` node covering it is
/// the same failure mode and all of them match.  An event alias is local to its
/// diagram, so a target that matches several events, or an event and a failure
/// mode, is ambiguous.
fn lookup_target(
    graph: &Graph,
    targets: &Targets,
    gate: usize,
    target: &str,
) -> Result<Lookup, FtaError> {
    let Some((prefix, alias)) = target.rsplit_once('.') else {
        return Ok(match targets.transfer_outs.get(target) {
            Some(&t) => Lookup::Found(vec![t]),
            None => Lookup::Missing(format!(
                "no $TransferOutEvent with the alias {target} exists"
            )),
        });
    };

    let failure_modes = targets
        .failure_mode_nodes
        .get(target)
        .cloned()
        .unwrap_or_default();
    let prefix_nodes = targets
        .failure_mode_nodes
        .get(prefix)
        .map_or(&[][..], Vec::as_slice);
    let mut events = Vec::new();
    let mut why_not = Vec::new();
    for &f in prefix_nodes {
        match event_below(graph, f, alias) {
            Ok(e) => events.push(e),
            Err(reason) => why_not.push(reason),
        }
    }

    if (!failure_modes.is_empty() && !events.is_empty()) || events.len() > 1 {
        return Err(FtaError::AmbiguousTransferTarget {
            diagram: graph.diagram(gate).to_string(),
            line: graph.line(gate),
            target: target.to_string(),
            candidates: failure_modes
                .iter()
                .chain(&events)
                .map(|&n| graph.at(n))
                .collect::<Vec<_>>()
                .join(", "),
        });
    }

    let found: Vec<usize> = failure_modes.into_iter().chain(events).collect();
    if !found.is_empty() {
        return Ok(Lookup::Found(found));
    }
    let mut reasons = Vec::new();
    if is_valid_trlc_fqn(target) {
        reasons.push(format!("no $FailureMode covers {target}"));
    }
    if prefix_nodes.is_empty() && is_valid_trlc_fqn(prefix) {
        reasons.push(format!("no $FailureMode covers {prefix}"));
    }
    reasons.extend(why_not);
    Ok(Lookup::Missing(reasons.join("; ")))
}

/// Resolve every `$TransferInGate`: add the edge from each node it transfers
/// into up to the gate, and report a target that is unknown or shown with other
/// text.  Returns which nodes some gate transfers into.
fn resolve_transfers(
    graph: &Graph,
    targets: &Targets,
    edges: &mut Edges,
    findings: &mut Vec<FtaFinding>,
) -> Result<Vec<bool>, FtaError> {
    let mut referenced = vec![false; graph.nodes.len()];
    for g in 0..graph.nodes.len() {
        let gate = graph.node(g);
        let Some(target) = gate.transfer_target() else {
            continue;
        };
        let found = match lookup_target(graph, targets, g, target)? {
            Lookup::Found(found) => found,
            Lookup::Missing(reason) => {
                findings.push(graph.finding(
                    g,
                    FindingKind::UnknownTransferTarget,
                    format!(
                        "$TransferInGate {} transfers into {target:?}, but {reason}",
                        gate.alias
                    ),
                ));
                continue;
            }
        };
        let shown = gate.name.as_deref().unwrap_or_default();
        let mut read: Vec<&str> = found
            .iter()
            .map(|&t| graph.node(t).name.as_deref().unwrap_or_default())
            .collect();
        if !read.contains(&shown) {
            read.sort_unstable();
            read.dedup();
            let read: Vec<String> = read.iter().map(|text| format!("{text:?}")).collect();
            findings.push(graph.finding(
                g,
                FindingKind::TransferNameMismatch,
                format!(
                    "$TransferInGate {} shows {shown:?} but its target {target:?} reads {}",
                    gate.alias,
                    read.join(" or ")
                ),
            ));
        }
        for t in found {
            edges.up[t].push(g);
            edges.transfers[t].push(g);
            referenced[t] = true;
        }
    }
    Ok(referenced)
}

fn is_known(known: Option<&HashSet<String>>, failure_mode: &str) -> bool {
    known.is_none_or(|known| known.contains(failure_mode))
}

/// The failure modes each node covers itself; an unknown one is reported and
/// dropped.
fn own_failure_modes(
    graph: &Graph,
    known: Option<&HashSet<String>>,
    findings: &mut Vec<FtaFinding>,
) -> Vec<BTreeSet<String>> {
    let mut own = vec![BTreeSet::new(); graph.nodes.len()];
    for (n, own) in own.iter_mut().enumerate() {
        for fqn in graph.node(n).failure_modes() {
            if is_known(known, fqn) {
                own.insert(fqn.clone());
            } else {
                findings.push(graph.finding(
                    n,
                    FindingKind::UnknownFailureMode,
                    format!(
                        "$FailureMode covers {fqn}, which no failure_modes target of this \
                         fault_trees target defines"
                    ),
                ));
            }
        }
    }
    own
}

/// One stub per root-cause alias over all diagrams.  A title that differs from
/// the first occurrence's is reported.
fn merge_root_causes(
    graph: &Graph,
    solved: &Solved,
    findings: &mut Vec<FtaFinding>,
) -> Vec<RootCauseStub> {
    let mut merged: BTreeMap<String, RootCauseStub> = BTreeMap::new();
    for n in 0..graph.nodes.len() {
        let node = graph.node(n);
        if !matches!(node.kind, NodeKind::RootCause) {
            continue;
        }
        let title = node.name.clone().unwrap_or_default();
        let stub = merged
            .entry(node.alias.clone())
            .or_insert_with(|| RootCauseStub {
                name: node.alias.clone(),
                title: title.clone(),
                diagram: graph.diagram(n).to_string(),
                line: graph.line(n),
                failure_modes: Vec::new(),
                via: BTreeMap::new(),
            });
        if stub.title != title {
            findings.push(graph.finding(
                n,
                FindingKind::RootCauseTitleMismatch,
                format!(
                    "$RootCause {} is titled {title:?} here but {:?} at {}:{}",
                    node.alias, stub.title, stub.diagram, stub.line
                ),
            ));
        }
        let mut all: BTreeSet<String> = stub.failure_modes.iter().cloned().collect();
        all.extend(solved.failure_modes[n].iter().cloned());
        stub.failure_modes = all.into_iter().collect();
        for (failure_mode, gates) in &solved.via[n] {
            stub.via
                .entry(failure_mode.clone())
                .or_default()
                .extend(gates.iter().map(|&g| graph.source_ref(g)));
        }
    }
    merged.into_values().collect()
}

/// Report an event or gate with nothing below it, and a `$TransferOutEvent`
/// that no gate transfers into.
fn completeness_findings(
    graph: &Graph,
    edges: &Edges,
    referenced: &[bool],
    findings: &mut Vec<FtaFinding>,
) {
    for (n, &is_referenced) in referenced.iter().enumerate() {
        let node = graph.node(n);
        let developed = matches!(
            node.kind,
            NodeKind::RootCause | NodeKind::TransferInGate { .. }
        ) || edges.has_child[n];
        if !developed {
            findings.push(graph.finding(
                n,
                FindingKind::UndevelopedEvent,
                format!("{} has nothing connected below it", describe(node)),
            ));
        }
        if matches!(node.kind, NodeKind::TransferOutEvent) && !is_referenced {
            findings.push(graph.finding(
                n,
                FindingKind::UnreferencedTransferOutEvent,
                format!(
                    "$TransferOutEvent {} is not the target of any $TransferInGate",
                    node.alias
                ),
            ));
        }
    }
}

/// The `$FailureMode` calls covering each known failure mode.
fn failure_mode_trees(
    graph: &Graph,
    targets: &Targets,
    known: Option<&HashSet<String>>,
) -> BTreeMap<String, BTreeSet<SourceRef>> {
    let mut trees: BTreeMap<String, BTreeSet<SourceRef>> = BTreeMap::new();
    for (fqn, nodes) in &targets.failure_mode_nodes {
        if is_known(known, fqn) {
            trees
                .entry(fqn.to_string())
                .or_default()
                .extend(nodes.iter().map(|&n| graph.source_ref(n)));
        }
    }
    trees
}

/// Attribute every root cause to the failure modes it hangs under.
///
/// `diagrams` are the basenames and models of every diagram of one
/// `fault_trees` target, in the order that decides which occurrence of a root
/// cause names its record.  `known_failure_modes` are the fully-qualified names
/// that exist; `None` accepts every name.
///
/// Fails on structural errors: a `$TransferInGate` target that is ambiguous, a
/// `$TransferOutEvent` alias defined twice, a cycle.  Everything that only
/// leaves the tracing incomplete is a finding.
pub fn resolve_root_causes(
    diagrams: &[(&str, &FtaModel)],
    known_failure_modes: Option<&HashSet<String>>,
) -> Result<Resolution, FtaError> {
    let graph = Graph::new(diagrams);
    let mut findings: Vec<FtaFinding> = Vec::new();

    let mut edges = connect(&graph, &mut findings);
    let targets = index_targets(&graph)?;
    let referenced = resolve_transfers(&graph, &targets, &mut edges, &mut findings)?;
    let own = own_failure_modes(&graph, known_failure_modes, &mut findings);
    let solved = Solver::new(&graph, &edges, own).solve()?;
    let root_causes = merge_root_causes(&graph, &solved, &mut findings);
    completeness_findings(&graph, &edges, &referenced, &mut findings);
    let failure_mode_trees = failure_mode_trees(&graph, &targets, known_failure_modes);

    findings.sort_by(|a, b| {
        (&a.diagram, a.line, a.kind, &a.message).cmp(&(&b.diagram, b.line, b.kind, &b.message))
    });
    Ok(Resolution {
        root_causes,
        findings,
        failure_mode_trees,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_util::*;
    use crate::model::{
        AND_GATE, FAILURE_MODE, INTERMEDIATE_EVENT, OR_GATE, ROOT_CAUSE, TRANSFER_IN_GATE,
        TRANSFER_OUT_EVENT,
    };
    use procedure_preprocessor::Statement;

    fn resolve(diagrams: Vec<(&str, Vec<Statement>)>) -> Result<Resolution, FtaError> {
        let models: Vec<(String, FtaModel)> = diagrams
            .into_iter()
            .map(|(name, stmts)| (name.to_string(), model_from_stmts(stmts)))
            .collect();
        let refs: Vec<(&str, &FtaModel)> = models.iter().map(|(n, m)| (n.as_str(), m)).collect();
        resolve_root_causes(&refs, None)
    }

    fn fms<'a>(resolution: &'a Resolution, rc: &str) -> Vec<&'a str> {
        resolution
            .root_causes
            .iter()
            .find(|r| r.name == rc)
            .unwrap_or_else(|| panic!("no root cause {rc}"))
            .failure_modes
            .iter()
            .map(String::as_str)
            .collect()
    }

    fn kinds(resolution: &Resolution) -> Vec<FindingKind> {
        resolution.findings.iter().map(|f| f.kind).collect()
    }

    /// FM -> OG -> {Cm}, nothing else: a complete tree.
    fn simple_tree(fm: &str, rc: &str) -> Vec<Statement> {
        vec![
            mk_call(FAILURE_MODE, &["fm", fm], 1),
            mk_call(OR_GATE, &["OG", fm], 2),
            mk_call(ROOT_CAUSE, &["cause", rc, "OG"], 3),
        ]
    }

    #[test]
    fn root_cause_belongs_to_the_failure_mode_above_it() {
        let r = resolve(vec![("a.puml", simple_tree("Lib.A", "Cm"))]).unwrap();
        assert_eq!(fms(&r, "Cm"), vec!["Lib.A"]);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn failure_mode_with_several_failure_modes_attributes_all_of_them() {
        let r = resolve(vec![(
            "a.puml",
            vec![
                mk_call(FAILURE_MODE, &["fm", "Lib.A", "Lib.B"], 1),
                mk_call(OR_GATE, &["OG", "Lib.A"], 2),
                mk_call(ROOT_CAUSE, &["cause", "Cm", "OG"], 3),
            ],
        )])
        .unwrap();
        assert_eq!(fms(&r, "Cm"), vec!["Lib.A", "Lib.B"]);
    }

    #[test]
    fn root_cause_shared_by_two_diagrams_unions_failure_modes_and_keeps_first_occurrence() {
        let r = resolve(vec![
            ("a.puml", simple_tree("Lib.A", "Cm")),
            ("b.puml", simple_tree("Lib.B", "Cm")),
        ])
        .unwrap();
        assert_eq!(r.root_causes.len(), 1);
        assert_eq!(fms(&r, "Cm"), vec!["Lib.A", "Lib.B"]);
        assert_eq!(r.root_causes[0].diagram, "a.puml");
        assert_eq!(r.root_causes[0].line, 3);
    }

    #[test]
    fn root_causes_are_sorted_by_name() {
        let r = resolve(vec![(
            "a.puml",
            vec![
                mk_call(FAILURE_MODE, &["fm", "Lib.A"], 1),
                mk_call(ROOT_CAUSE, &["z", "Zed", "Lib.A"], 2),
                mk_call(ROOT_CAUSE, &["a", "Alpha", "Lib.A"], 3),
            ],
        )])
        .unwrap();
        let names: Vec<&str> = r.root_causes.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Zed"]);
    }

    #[test]
    fn differing_root_cause_titles_are_a_finding_and_the_first_title_wins() {
        let mut b = simple_tree("Lib.B", "Cm");
        b[2] = mk_call(ROOT_CAUSE, &["other title", "Cm", "OG"], 3);
        let r = resolve(vec![("a.puml", simple_tree("Lib.A", "Cm")), ("b.puml", b)]).unwrap();
        assert_eq!(kinds(&r), vec![FindingKind::RootCauseTitleMismatch]);
        assert_eq!(r.findings[0].diagram, "b.puml");
        assert_eq!(r.root_causes[0].title, "cause");
    }

    #[test]
    fn root_cause_reaching_no_failure_mode_is_still_emitted() {
        let r = resolve(vec![(
            "a.puml",
            vec![
                mk_call(FAILURE_MODE, &["fm", "Lib.A"], 1),
                mk_call(ROOT_CAUSE, &["cause", "Cm", "Missing"], 2),
            ],
        )])
        .unwrap();
        assert!(fms(&r, "Cm").is_empty());
        assert!(kinds(&r).contains(&FindingKind::DanglingConnection));
    }

    #[test]
    fn dangling_connection_is_a_finding_at_its_line() {
        let r = resolve(vec![(
            "a.puml",
            vec![
                mk_call(FAILURE_MODE, &["fm", "Lib.A"], 1),
                mk_call(OR_GATE, &["OG", "Missing"], 7),
            ],
        )])
        .unwrap();
        let dangling: Vec<_> = r
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::DanglingConnection)
            .collect();
        assert_eq!(dangling.len(), 1);
        assert_eq!(
            (dangling[0].diagram.as_str(), dangling[0].line),
            ("a.puml", 7)
        );
    }

    #[test]
    fn unknown_failure_mode_is_dropped_and_reported() {
        let models = [(
            "a.puml".to_string(),
            model_from_stmts(simple_tree("Lib.A", "Cm")),
        )];
        let refs: Vec<(&str, &FtaModel)> = models.iter().map(|(n, m)| (n.as_str(), m)).collect();
        let known: HashSet<String> = HashSet::new();
        let r = resolve_root_causes(&refs, Some(&known)).unwrap();
        assert!(fms(&r, "Cm").is_empty());
        assert_eq!(kinds(&r), vec![FindingKind::UnknownFailureMode]);
        assert_eq!(r.findings[0].line, 1);
    }

    #[test]
    fn known_failure_modes_filter_only_the_unknown_ones() {
        let models = [(
            "a.puml".to_string(),
            model_from_stmts(vec![
                mk_call(FAILURE_MODE, &["fm", "Lib.A", "Lib.Gone"], 1),
                mk_call(ROOT_CAUSE, &["cause", "Cm", "Lib.A"], 2),
            ]),
        )];
        let refs: Vec<(&str, &FtaModel)> = models.iter().map(|(n, m)| (n.as_str(), m)).collect();
        let known: HashSet<String> = ["Lib.A".to_string()].into();
        let r = resolve_root_causes(&refs, Some(&known)).unwrap();
        assert_eq!(fms(&r, "Cm"), vec!["Lib.A"]);
        assert_eq!(kinds(&r), vec![FindingKind::UnknownFailureMode]);
    }

    #[test]
    fn undeveloped_event_gate_and_failure_mode_are_findings() {
        let r = resolve(vec![(
            "a.puml",
            vec![
                mk_call(FAILURE_MODE, &["fm", "Lib.A"], 1),
                mk_call(INTERMEDIATE_EVENT, &["ie", "Ie", "Lib.A"], 2),
                mk_call(AND_GATE, &["AG", "Ie"], 3),
                mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 4),
            ],
        )])
        .unwrap();
        let undeveloped: Vec<usize> = r
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::UndevelopedEvent)
            .map(|f| f.line)
            .collect();
        assert_eq!(undeveloped, vec![3, 4]);
    }

    #[test]
    fn transfer_in_gate_is_not_an_undeveloped_event() {
        let r = resolve(vec![(
            "a.puml",
            vec![
                mk_call(FAILURE_MODE, &["fm", "Lib.A"], 1),
                mk_call(TRANSFER_IN_GATE, &["Out", "Out1", "T1", "Lib.A"], 2),
                mk_call(TRANSFER_OUT_EVENT, &["Out", "Out1"], 3),
                mk_call(ROOT_CAUSE, &["cause", "Cm", "Out1"], 4),
            ],
        )])
        .unwrap();
        assert!(r.findings.is_empty(), "{:?}", r.findings);
    }

    // Diagram a: FM A <- T -> target. Diagram b: the target tree.
    fn transferring(target: &str, shown: &str) -> Vec<Statement> {
        vec![
            mk_call(FAILURE_MODE, &["fm a", "Lib.A"], 1),
            mk_call(TRANSFER_IN_GATE, &[shown, target, "T1", "Lib.A"], 2),
        ]
    }

    #[test]
    fn transfer_into_failure_mode_inherits_its_root_causes() {
        let r = resolve(vec![
            ("a.puml", transferring("Lib.B", "fm b")),
            ("b.puml", {
                let mut s = simple_tree("Lib.B", "CmB");
                s[0] = mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1);
                s
            }),
        ])
        .unwrap();
        assert_eq!(fms(&r, "CmB"), vec!["Lib.A", "Lib.B"]);
        assert!(r.findings.is_empty(), "{:?}", r.findings);
    }

    fn b_tree() -> Vec<Statement> {
        let mut s = simple_tree("Lib.B", "CmB");
        s[0] = mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1);
        s
    }

    #[test]
    fn root_cause_lists_the_transfer_gates_it_reaches_a_failure_mode_through() {
        let r = resolve(vec![
            ("a.puml", transferring("Lib.B", "fm b")),
            ("b.puml", b_tree()),
        ])
        .unwrap();
        let rc = &r.root_causes[0];
        assert_eq!(rc.via.keys().collect::<Vec<_>>(), vec!["Lib.A"]);
        let gates: Vec<_> = rc.via["Lib.A"]
            .iter()
            .map(|g| (g.diagram.as_str(), g.line, g.alias.as_str()))
            .collect();
        assert_eq!(gates, vec![("a.puml", 2, "T1")]);
    }

    #[test]
    fn root_cause_without_a_transfer_has_no_via() {
        let r = resolve(vec![("a.puml", simple_tree("Lib.A", "Cm"))]).unwrap();
        assert!(r.root_causes[0].via.is_empty());
    }

    #[test]
    fn via_keeps_every_gate_of_a_transfer_chain() {
        let r = resolve(vec![
            (
                "a.puml",
                vec![
                    mk_call(FAILURE_MODE, &["fm a", "Lib.A"], 1),
                    mk_call(TRANSFER_IN_GATE, &["One", "Out1", "T1", "Lib.A"], 2),
                ],
            ),
            (
                "b.puml",
                vec![
                    mk_call(TRANSFER_OUT_EVENT, &["One", "Out1"], 1),
                    mk_call(TRANSFER_IN_GATE, &["Two", "Out2", "T2", "Out1"], 2),
                ],
            ),
            (
                "c.puml",
                vec![
                    mk_call(TRANSFER_OUT_EVENT, &["Two", "Out2"], 1),
                    mk_call(ROOT_CAUSE, &["cause", "Cm", "Out2"], 2),
                ],
            ),
        ])
        .unwrap();
        let aliases: Vec<_> = r.root_causes[0].via["Lib.A"]
            .iter()
            .map(|g| g.alias.as_str())
            .collect();
        assert_eq!(aliases, vec!["T1", "T2"]);
    }

    #[test]
    fn failure_mode_trees_name_every_failure_mode_call() {
        let r = resolve(vec![
            ("a.puml", simple_tree("Lib.A", "Cm")),
            ("b.puml", simple_tree("Lib.A", "Cn")),
        ])
        .unwrap();
        let trees: Vec<_> = r.failure_mode_trees["Lib.A"]
            .iter()
            .map(|t| (t.diagram.as_str(), t.line))
            .collect();
        assert_eq!(trees, vec![("a.puml", 1), ("b.puml", 1)]);
    }

    #[test]
    fn unknown_failure_modes_have_no_tree() {
        let models = [(
            "a.puml".to_string(),
            model_from_stmts(simple_tree("Lib.A", "Cm")),
        )];
        let refs: Vec<(&str, &FtaModel)> = models.iter().map(|(n, m)| (n.as_str(), m)).collect();
        let r = resolve_root_causes(&refs, Some(&HashSet::new())).unwrap();
        assert!(r.failure_mode_trees.is_empty());
    }

    #[test]
    fn transfer_into_any_failure_mode_of_the_target_node_works() {
        let r = resolve(vec![
            ("a.puml", transferring("Lib.B2", "fm b")),
            (
                "b.puml",
                vec![
                    mk_call(FAILURE_MODE, &["fm b", "Lib.B1", "Lib.B2"], 1),
                    mk_call(ROOT_CAUSE, &["cause", "CmB", "Lib.B1"], 2),
                ],
            ),
        ])
        .unwrap();
        assert_eq!(fms(&r, "CmB"), vec!["Lib.A", "Lib.B1", "Lib.B2"]);
    }

    #[test]
    fn transfer_into_a_failure_mode_covered_by_several_nodes_inherits_for_all() {
        let r = resolve(vec![
            ("a.puml", transferring("Lib.B", "fm b")),
            (
                "b1.puml",
                vec![
                    mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1),
                    mk_call(ROOT_CAUSE, &["c1", "Cm1", "Lib.B"], 2),
                ],
            ),
            (
                "b2.puml",
                vec![
                    mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1),
                    mk_call(ROOT_CAUSE, &["c2", "Cm2", "Lib.B"], 2),
                ],
            ),
        ])
        .unwrap();
        assert_eq!(fms(&r, "Cm1"), vec!["Lib.A", "Lib.B"]);
        assert_eq!(fms(&r, "Cm2"), vec!["Lib.A", "Lib.B"]);
    }

    #[test]
    fn transfer_into_event_inherits_only_root_causes_below_it() {
        let r = resolve(vec![
            ("a.puml", transferring("Lib.B.Ie", "the event")),
            (
                "b.puml",
                vec![
                    mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1),
                    mk_call(OR_GATE, &["OG", "Lib.B"], 2),
                    mk_call(INTERMEDIATE_EVENT, &["the event", "Ie", "OG"], 3),
                    mk_call(ROOT_CAUSE, &["below", "Below", "Ie"], 4),
                    mk_call(ROOT_CAUSE, &["beside", "Beside", "OG"], 5),
                ],
            ),
        ])
        .unwrap();
        assert_eq!(fms(&r, "Below"), vec!["Lib.A", "Lib.B"]);
        assert_eq!(fms(&r, "Beside"), vec!["Lib.B"]);
        assert!(r.findings.is_empty(), "{:?}", r.findings);
    }

    #[test]
    fn transfer_into_event_in_the_same_diagram_works() {
        let r = resolve(vec![(
            "a.puml",
            vec![
                mk_call(FAILURE_MODE, &["fm a", "Lib.A"], 1),
                mk_call(TRANSFER_IN_GATE, &["event", "Lib.B.Ie", "T1", "Lib.A"], 2),
                mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 3),
                mk_call(INTERMEDIATE_EVENT, &["event", "Ie", "Lib.B"], 4),
                mk_call(ROOT_CAUSE, &["cause", "Cm", "Ie"], 5),
            ],
        )])
        .unwrap();
        assert_eq!(fms(&r, "Cm"), vec!["Lib.A", "Lib.B"]);
    }

    #[test]
    fn partial_fta_shared_by_two_failure_modes() {
        let partial = vec![
            mk_call(TRANSFER_OUT_EVENT, &["Shared", "Out1"], 1),
            mk_call(ROOT_CAUSE, &["cause", "Cm", "Out1"], 2),
        ];
        let r = resolve(vec![
            ("a.puml", transferring("Out1", "Shared")),
            ("b.puml", {
                let mut s = transferring("Out1", "Shared");
                s[0] = mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1);
                s[1] = mk_call(TRANSFER_IN_GATE, &["Shared", "Out1", "T1", "Lib.B"], 2);
                s
            }),
            ("c.puml", partial),
        ])
        .unwrap();
        assert_eq!(fms(&r, "Cm"), vec!["Lib.A", "Lib.B"]);
        assert!(r.findings.is_empty(), "{:?}", r.findings);
    }

    #[test]
    fn nested_partial_ftas_pass_failure_modes_down_the_chain() {
        let r = resolve(vec![
            ("a.puml", transferring("Out1", "One")),
            (
                "b.puml",
                vec![
                    mk_call(TRANSFER_OUT_EVENT, &["One", "Out1"], 1),
                    mk_call(TRANSFER_IN_GATE, &["Two", "Out2", "T2", "Out1"], 2),
                ],
            ),
            (
                "c.puml",
                vec![
                    mk_call(TRANSFER_OUT_EVENT, &["Two", "Out2"], 1),
                    mk_call(ROOT_CAUSE, &["cause", "Cm", "Out2"], 2),
                ],
            ),
        ])
        .unwrap();
        assert_eq!(fms(&r, "Cm"), vec!["Lib.A"]);
    }

    #[test]
    fn mutual_transfers_into_different_events_are_not_a_cycle() {
        let r = resolve(vec![
            (
                "a.puml",
                vec![
                    mk_call(FAILURE_MODE, &["fm a", "Lib.A"], 1),
                    mk_call(OR_GATE, &["OGA", "Lib.A"], 2),
                    mk_call(TRANSFER_IN_GATE, &["y", "Lib.B.Y", "TA", "OGA"], 3),
                    mk_call(INTERMEDIATE_EVENT, &["w", "W", "OGA"], 4),
                    mk_call(ROOT_CAUSE, &["cw", "CmW", "W"], 5),
                ],
            ),
            (
                "b.puml",
                vec![
                    mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1),
                    mk_call(OR_GATE, &["OGB", "Lib.B"], 2),
                    mk_call(INTERMEDIATE_EVENT, &["y", "Y", "OGB"], 3),
                    mk_call(ROOT_CAUSE, &["cy", "CmY", "Y"], 4),
                    mk_call(TRANSFER_IN_GATE, &["w", "Lib.A.W", "TB", "OGB"], 5),
                ],
            ),
        ])
        .unwrap();
        assert_eq!(fms(&r, "CmY"), vec!["Lib.A", "Lib.B"]);
        assert_eq!(fms(&r, "CmW"), vec!["Lib.A", "Lib.B"]);
    }

    #[test]
    fn transfer_cycle_is_an_error_naming_the_nodes() {
        let err = resolve(vec![
            ("a.puml", transferring("Lib.B", "fm b")),
            (
                "b.puml",
                vec![
                    mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1),
                    mk_call(TRANSFER_IN_GATE, &["fm a", "Lib.A", "T2", "Lib.B"], 2),
                ],
            ),
        ])
        .unwrap_err();
        let FtaError::Cycle { cycle } = err else {
            panic!("expected Cycle, got {err:?}");
        };
        assert!(
            cycle.contains("a.puml:1") && cycle.contains("b.puml:2"),
            "{cycle}"
        );
    }

    #[test]
    fn transfer_into_own_tree_is_a_cycle() {
        let err = resolve(vec![("a.puml", transferring("Lib.A", "fm a"))]).unwrap_err();
        assert!(matches!(err, FtaError::Cycle { .. }));
    }

    #[test]
    fn connection_cycle_is_an_error() {
        let err = resolve(vec![(
            "a.puml",
            vec![
                mk_call(OR_GATE, &["G1", "G2"], 1),
                mk_call(OR_GATE, &["G2", "G1"], 2),
            ],
        )])
        .unwrap_err();
        assert!(matches!(err, FtaError::Cycle { .. }));
    }

    #[test]
    fn unknown_transfer_target_is_a_finding_for_every_form() {
        for (target, reason) in [
            ("Nope", "no $TransferOutEvent with the alias Nope exists"),
            ("Lib.Nope", "no $FailureMode covers Lib.Nope"),
            ("Lib.Nope.Ie", "no $FailureMode covers Lib.Nope"),
        ] {
            let r = resolve(vec![("a.puml", transferring(target, "x"))]).unwrap();
            assert_eq!(
                kinds(&r),
                vec![FindingKind::UnknownTransferTarget],
                "{target}"
            );
            assert_eq!(r.findings[0].line, 2);
            assert!(r.findings[0].message.contains(reason), "{:?}", r.findings);
        }
    }

    #[test]
    fn event_target_must_hang_under_the_named_failure_mode() {
        let r = resolve(vec![
            ("a.puml", transferring("Lib.B.Ie", "event")),
            (
                "b.puml",
                vec![
                    mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1),
                    mk_call(FAILURE_MODE, &["fm c", "Lib.C"], 2),
                    mk_call(INTERMEDIATE_EVENT, &["event", "Ie", "Lib.C"], 3),
                ],
            ),
        ])
        .unwrap();
        let unknown: Vec<_> = r
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::UnknownTransferTarget)
            .collect();
        assert_eq!(unknown.len(), 1);
        assert!(
            unknown[0].message.contains(
                "$IntermediateEvent Ie at b.puml:3 does not hang under $FailureMode Lib.B"
            ),
            "{:?}",
            r.findings
        );
    }

    #[test]
    fn event_target_must_be_an_intermediate_event() {
        let r = resolve(vec![
            ("a.puml", transferring("Lib.B.Cm", "cause")),
            ("b.puml", simple_tree("Lib.B", "Cm")),
        ])
        .unwrap();
        assert_eq!(kinds(&r), vec![FindingKind::UnknownTransferTarget]);
        assert!(
            r.findings[0]
                .message
                .contains("b.puml:3 Cm is a $RootCause, not an $IntermediateEvent"),
            "{:?}",
            r.findings
        );
    }

    #[test]
    fn event_target_missing_below_the_failure_mode_names_the_diagram() {
        let r = resolve(vec![
            ("a.puml", transferring("Lib.B.Gone", "event")),
            ("b.puml", simple_tree("Lib.B", "Cm")),
        ])
        .unwrap();
        assert!(
            r.findings[0]
                .message
                .contains("b.puml has no node Gone for $FailureMode Lib.B"),
            "{:?}",
            r.findings
        );
    }

    #[test]
    fn event_target_matching_in_several_diagrams_is_an_error() {
        let event_tree = |line_offset: usize| {
            vec![
                mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1),
                mk_call(
                    INTERMEDIATE_EVENT,
                    &["event", "Ie", "Lib.B"],
                    2 + line_offset,
                ),
            ]
        };
        let err = resolve(vec![
            ("a.puml", transferring("Lib.B.Ie", "event")),
            ("b1.puml", event_tree(0)),
            ("b2.puml", event_tree(0)),
        ])
        .unwrap_err();
        let FtaError::AmbiguousTransferTarget { candidates, .. } = err else {
            panic!("expected AmbiguousTransferTarget, got {err:?}");
        };
        assert_eq!(candidates, "b1.puml:2, b2.puml:2");
    }

    #[test]
    fn target_matching_a_failure_mode_and_an_event_is_an_error() {
        let node = |kind, name: &str, alias: &str, connection: Option<&str>, line| FtaNode {
            kind,
            name: Some(name.to_string()),
            alias: alias.to_string(),
            connections: connection.map(str::to_string).into_iter().collect(),
            line,
        };
        let covering = |fqn: &str| NodeKind::FailureMode {
            failure_modes: vec![fqn.to_string()],
        };
        let model = FtaModel {
            nodes: vec![
                node(covering("Lib"), "fm lib", "Lib", None, 1),
                node(NodeKind::IntermediateEvent, "event", "B", Some("Lib"), 2),
                node(covering("Lib.B"), "fm b", "Lib.B", None, 3),
                node(
                    NodeKind::TransferInGate {
                        target: "Lib.B".to_string(),
                    },
                    "event",
                    "T1",
                    Some("Lib.B"),
                    4,
                ),
            ],
        };
        let err = resolve_root_causes(&[("a.puml", &model)], None).unwrap_err();
        let FtaError::AmbiguousTransferTarget {
            line, candidates, ..
        } = err
        else {
            panic!("expected AmbiguousTransferTarget, got {err:?}");
        };
        assert_eq!((line, candidates.as_str()), (4, "a.puml:3, a.puml:2"));
    }

    #[test]
    fn transfer_name_mismatch_lists_every_distinct_target_text() {
        let r = resolve(vec![
            ("a.puml", transferring("Lib.B", "other")),
            (
                "b1.puml",
                vec![mk_call(FAILURE_MODE, &["first", "Lib.B"], 1)],
            ),
            (
                "b2.puml",
                vec![mk_call(FAILURE_MODE, &["second", "Lib.B"], 1)],
            ),
            (
                "b3.puml",
                vec![mk_call(FAILURE_MODE, &["first", "Lib.B"], 1)],
            ),
        ])
        .unwrap();
        let mismatch: Vec<_> = r
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::TransferNameMismatch)
            .collect();
        assert_eq!(mismatch.len(), 1);
        assert!(
            mismatch[0]
                .message
                .ends_with("reads \"first\" or \"second\""),
            "{}",
            mismatch[0].message
        );
    }

    #[test]
    fn root_cause_repeated_under_two_gates_reaches_both_failure_modes() {
        let r = resolve(vec![(
            "a.puml",
            vec![
                mk_call(FAILURE_MODE, &["first", "Lib.A"], 1),
                mk_call(FAILURE_MODE, &["second", "Lib.B"], 2),
                mk_call(OR_GATE, &["OGA", "Lib.A"], 3),
                mk_call(OR_GATE, &["OGB", "Lib.B"], 4),
                mk_call(ROOT_CAUSE, &["shared", "Cm", "OGA"], 5),
                mk_call(ROOT_CAUSE, &["shared", "Cm", "OGB"], 6),
            ],
        )])
        .unwrap();
        assert_eq!(fms(&r, "Cm"), vec!["Lib.A", "Lib.B"]);
        assert!(r.findings.is_empty(), "{:?}", r.findings);
    }

    #[test]
    fn a_very_long_connection_chain_does_not_overflow_the_stack() {
        let depth = 50_000;
        let node = |kind, name: Option<&str>, alias: String, connection: Option<String>| FtaNode {
            kind,
            name: name.map(str::to_string),
            alias,
            connections: connection.into_iter().collect(),
            line: 1,
        };
        // The deepest node comes first so that the walk starts at the bottom.
        let mut nodes = vec![node(
            NodeKind::RootCause,
            Some("cause"),
            "Cm".to_string(),
            Some(format!("G{}", depth - 1)),
        )];
        for i in (0..depth).rev() {
            let connection = if i == 0 {
                "Lib.A".to_string()
            } else {
                format!("G{}", i - 1)
            };
            nodes.push(node(
                NodeKind::OrGate,
                None,
                format!("G{i}"),
                Some(connection),
            ));
        }
        nodes.push(node(
            NodeKind::FailureMode {
                failure_modes: vec!["Lib.A".to_string()],
            },
            Some("fm"),
            "Lib.A".to_string(),
            None,
        ));
        let r = resolve_root_causes(&[("a.puml", &FtaModel { nodes })], None).unwrap();
        assert_eq!(fms(&r, "Cm"), vec!["Lib.A"]);
    }

    #[test]
    fn transfer_out_event_defined_twice_is_an_error() {
        let out = vec![mk_call(TRANSFER_OUT_EVENT, &["Out", "Out1"], 1)];
        let err = resolve(vec![("a.puml", out.clone()), ("b.puml", out)]).unwrap_err();
        assert!(matches!(
            &err,
            FtaError::DuplicateTransferOutEvent { alias, first, second }
                if alias == "Out1" && first == "a.puml:1" && second == "b.puml:1"
        ));
    }

    #[test]
    fn transfer_name_must_equal_the_target_text_exactly() {
        let target = vec![
            mk_call(FAILURE_MODE, &["fm b", "Lib.B"], 1),
            mk_call(ROOT_CAUSE, &["cause", "Cm", "Lib.B"], 2),
        ];
        let ok = resolve(vec![
            ("a.puml", transferring("Lib.B", "fm b")),
            ("b.puml", target.clone()),
        ])
        .unwrap();
        assert!(ok.findings.is_empty());
        for shown in ["Fm b", "fm b ", "fm  b"] {
            let r = resolve(vec![
                ("a.puml", transferring("Lib.B", shown)),
                ("b.puml", target.clone()),
            ])
            .unwrap();
            assert_eq!(
                kinds(&r),
                vec![FindingKind::TransferNameMismatch],
                "{shown:?}"
            );
        }
    }

    #[test]
    fn unreferenced_transfer_out_event_is_a_finding() {
        let r = resolve(vec![(
            "a.puml",
            vec![
                mk_call(TRANSFER_OUT_EVENT, &["Out", "Out1"], 1),
                mk_call(ROOT_CAUSE, &["cause", "Cm", "Out1"], 2),
            ],
        )])
        .unwrap();
        assert_eq!(kinds(&r), vec![FindingKind::UnreferencedTransferOutEvent]);
        assert!(fms(&r, "Cm").is_empty());
    }

    #[test]
    fn findings_are_sorted_by_diagram_then_line() {
        let r = resolve(vec![
            ("b.puml", vec![mk_call(OR_GATE, &["OG", "Missing"], 5)]),
            ("a.puml", vec![mk_call(OR_GATE, &["OG", "Missing"], 9)]),
        ])
        .unwrap();
        let places: Vec<(&str, usize)> = r
            .findings
            .iter()
            .map(|f| (f.diagram.as_str(), f.line))
            .collect();
        assert_eq!(
            places,
            vec![("a.puml", 9), ("a.puml", 9), ("b.puml", 5), ("b.puml", 5)]
        );
    }
}
