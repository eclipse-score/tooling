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

//! Fault-Tree-Analysis (FTA) model and emitters.
//!
//! Consumes the procedure parser's [`ProcedureFile`] (the stream of
//! `$FailureMode(...)` / `$RootCause(...)` / gate macro calls produced after
//! `fta_metamodel.puml` has been inlined) and turns it into the generated
//! `FailureMode`/`RootCause` TRLC stub records (`fta_events.trlc`, see
//! [`render_trlc_stub`]) consumed by the `safety_analysis` rule.

use std::collections::HashMap;

use log::warn;
use procedure_preprocessor::{Arg, MacroCallDef, ProcedureFile, Statement};
use serde::Serialize;

/// Procedure macro names recognised in an FTA diagram.
const FAILURE_MODE: &str = "$FailureMode";
const INTERMEDIATE_EVENT: &str = "$IntermediateEvent";
const ROOT_CAUSE: &str = "$RootCause";
const AND_GATE: &str = "$AndGate";
const OR_GATE: &str = "$OrGate";
const TRANSFER_IN_GATE: &str = "$TransferInGate";

/// The kind of a node in a fault tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum NodeKind {
    /// `$FailureMode` — the root of the tree; covers one or more failure modes.
    FailureMode,
    /// `$IntermediateEvent` — a named intermediate node.
    IntermediateEvent,
    /// `$RootCause` — a leaf root cause / control measure.
    RootCause,
    /// `$AndGate`, `$OrGate`, `$TransferInGate` — a logic gate.  Use
    /// [`FtaNode::gate_kind`] to distinguish which one.
    Gate,
}

/// Which specific gate macro produced a [`NodeKind::Gate`] node.
///
/// `NodeKind::Gate` alone does not distinguish an internal `$AndGate`/`$OrGate`
/// from a `$TransferInGate` (which links to another diagram's failure mode).
/// Consumers that need that distinction (e.g. `puml_idmap`) must match on this
/// field rather than guessing from the node's `alias` shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GateKind {
    And,
    Or,
    /// Transfers into another diagram's failure mode; `alias` is that
    /// failure mode's TRLC fully-qualified name.
    TransferIn,
}

/// One node of a fault tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FtaNode {
    pub kind: NodeKind,
    /// Human readable display name (events only; gates carry `None`).
    pub name: Option<String>,
    /// Alias / identifier.  For a failure mode this is always the same as the
    /// first entry of `failure_modes` (a TRLC fully-qualified name); for a
    /// root cause it is a plain TRLC identifier.
    pub alias: String,
    /// Alias of the parent node this node connects upward to.  `None` for the
    /// failure mode (the root).
    pub connection: Option<String>,
    /// `Some` only when `kind == NodeKind::Gate`; identifies which gate macro
    /// produced this node. `None` for all other kinds.
    pub gate_kind: Option<GateKind>,
    /// `NodeKind::FailureMode` only: TRLC fully-qualified names of the failure
    /// modes this failure mode covers.  Empty for every other kind.
    pub failure_modes: Vec<String>,
    /// 1-based line of the macro call in its source diagram.
    /// `None` when the line is unavailable (e.g. synthesised nodes in tests).
    pub line: Option<usize>,
}

/// A fully parsed fault tree for a single diagram.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FtaModel {
    pub nodes: Vec<FtaNode>,
}

/// One record to emit into the generated TRLC stub package (see
/// [`render_trlc_stub`]).  Produced per-diagram by [`FtaModel::stub_events`]
/// and merged across every diagram of an `fta_package` by
/// [`merge_stub_events`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StubEvent {
    /// `NodeKind::FailureMode` or `NodeKind::RootCause` — never any other kind.
    pub kind: NodeKind,
    /// TRLC record name within the generated `fta_package` (see [`stub_name`]).
    pub stub_name: String,
    pub title: String,
    /// Basename of the source `.puml` diagram (first diagram, if merged).
    pub diagram: String,
    /// Source line in `diagram` (first diagram, if merged).
    pub line: usize,
    /// `FailureMode` only: fully-qualified names of the failure modes it covers.
    pub failure_modes: Vec<String>,
    /// `RootCause` only: `stub_name`s of the failure modes it contributes to,
    /// merged (order-preserving, de-duplicated) across every diagram.
    pub fta_failure_modes: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum FtaError {
    #[error("FTA macro {macro_name} requires at least {expected} argument(s)")]
    MissingArgs { macro_name: String, expected: usize },
    #[error("FTA macro {macro_name} expected a string argument at position {index}")]
    NonStringArg { macro_name: String, index: usize },
    #[error(
        "FTA macro $FailureMode at line {line}: failure mode {fm_fqn:?} is not a valid TRLC \
         fully-qualified name (expected 'Package.Name')"
    )]
    InvalidFailureModeFqn { line: usize, fm_fqn: String },
    #[error(
        "FTA {diagram}:{line}: $RootCause alias {alias:?} must be a plain TRLC identifier -- \
         dotted 'Package.Name' aliases are no longer supported for root causes"
    )]
    InvalidRootCauseAlias {
        diagram: String,
        line: usize,
        alias: String,
    },
    #[error(
        "FTA stub {stub_name:?} is declared as both a FailureMode (in {first_diagram}) and a \
         RootCause (in {second_diagram}); stub names must be unique across all diagrams \
         sharing an fta_package"
    )]
    StubKindClash {
        stub_name: String,
        first_diagram: String,
        second_diagram: String,
    },
    #[error(
        "FTA stub {stub_name:?} has a different title in {first_diagram} ({first_title:?}) than \
         in {second_diagram} ({second_title:?}); use the same title everywhere the same root \
         cause or failure mode is referenced"
    )]
    StubTitleMismatch {
        stub_name: String,
        first_diagram: String,
        first_title: String,
        second_diagram: String,
        second_title: String,
    },
}

/// Legacy guardrail (ported from `safety_analysis_tools.py`): a TRLC
/// fully-qualified name looks like `Package.Record` — exactly two dot-separated
/// identifier segments.
fn is_valid_trlc_fqn(alias: &str) -> bool {
    let parts: Vec<&str> = alias.split('.').collect();
    if parts.len() != 2 {
        return false;
    }
    parts.iter().all(|part| is_valid_identifier(part))
}

/// A single TRLC identifier: `[A-Za-z_][A-Za-z0-9_]*`.
fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    let first_ok = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_');
    first_ok && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Derive the local TRLC stub identifier from a diagram alias: a legacy
/// dotted alias (`Package.Name`) collapses to its last segment; any other
/// alias (new-style diagrams) is used verbatim.
fn stub_name(alias: &str) -> &str {
    alias.rsplit('.').next().unwrap_or(alias)
}

fn string_arg(call: &MacroCallDef, index: usize) -> Result<String, FtaError> {
    let arg = call.args.get(index).ok_or_else(|| FtaError::MissingArgs {
        macro_name: call.name.clone(),
        expected: index + 1,
    })?;
    match arg {
        Arg::String(s) => Ok(s.clone()),
        _ => Err(FtaError::NonStringArg {
            macro_name: call.name.clone(),
            index,
        }),
    }
}

impl FtaModel {
    /// Build a model from a parsed procedure file, ignoring procedure
    /// definitions and plain text — only the macro *calls* describe topology.
    pub fn from_procedure_file(file: &ProcedureFile) -> Result<Self, FtaError> {
        let mut nodes = Vec::new();
        for stmt in &file.stmts {
            let Statement::MacroCall(call) = stmt else {
                continue;
            };
            let line = call.line;

            let node = match call.name.as_str() {
                FAILURE_MODE => {
                    if call.args.len() < 2 {
                        return Err(FtaError::MissingArgs {
                            macro_name: call.name.clone(),
                            expected: 2,
                        });
                    }
                    let name = string_arg(call, 0)?;
                    let mut failure_modes = Vec::with_capacity(call.args.len() - 1);
                    for i in 1..call.args.len() {
                        let fm_fqn = string_arg(call, i)?;
                        if !is_valid_trlc_fqn(&fm_fqn) {
                            return Err(FtaError::InvalidFailureModeFqn {
                                line: line.unwrap_or(0),
                                fm_fqn,
                            });
                        }
                        if !failure_modes.contains(&fm_fqn) {
                            failure_modes.push(fm_fqn);
                        }
                    }
                    // Non-empty: the loop above ran at least once (`args.len() >= 2`).
                    let alias = failure_modes[0].clone();
                    FtaNode {
                        kind: NodeKind::FailureMode,
                        name: Some(name),
                        alias,
                        connection: None,
                        gate_kind: None,
                        failure_modes,
                        line,
                    }
                }
                INTERMEDIATE_EVENT => FtaNode {
                    kind: NodeKind::IntermediateEvent,
                    name: Some(string_arg(call, 0)?),
                    alias: string_arg(call, 1)?,
                    connection: Some(string_arg(call, 2)?),
                    gate_kind: None,
                    failure_modes: Vec::new(),
                    line,
                },
                ROOT_CAUSE => FtaNode {
                    kind: NodeKind::RootCause,
                    name: Some(string_arg(call, 0)?),
                    alias: string_arg(call, 1)?,
                    connection: Some(string_arg(call, 2)?),
                    gate_kind: None,
                    failure_modes: Vec::new(),
                    line,
                },
                AND_GATE | OR_GATE | TRANSFER_IN_GATE => {
                    let gate_kind = match call.name.as_str() {
                        AND_GATE => GateKind::And,
                        OR_GATE => GateKind::Or,
                        TRANSFER_IN_GATE => GateKind::TransferIn,
                        _ => unreachable!("matched only by the outer arm pattern"),
                    };
                    FtaNode {
                        kind: NodeKind::Gate,
                        name: None,
                        alias: string_arg(call, 0)?,
                        connection: Some(string_arg(call, 1)?),
                        gate_kind: Some(gate_kind),
                        failure_modes: Vec::new(),
                        line,
                    }
                }
                // Unknown / cosmetic macros are not part of the topology.
                _ => continue,
            };
            nodes.push(node);
        }

        Ok(Self { nodes })
    }

    fn iter_kind(&self, kind: NodeKind) -> impl Iterator<Item = &FtaNode> {
        self.nodes.iter().filter(move |n| n.kind == kind)
    }

    /// Index nodes by alias for O(1) parent lookups during the upward walk.
    /// On a duplicate alias the last node wins; a warning is emitted so
    /// malformed diagrams with repeated aliases are visible in the build log.
    fn alias_index(&self) -> HashMap<&str, &FtaNode> {
        let mut map = HashMap::with_capacity(self.nodes.len());
        for node in &self.nodes {
            if let Some(prev) = map.insert(node.alias.as_str(), node) {
                warn!(
                    "FTA diagram has duplicate alias {:?} at lines {} and {}; \
                     the later definition wins",
                    node.alias,
                    prev.line.unwrap_or(0),
                    node.line.unwrap_or(0),
                );
            }
        }
        map
    }

    /// Resolve the top-event (failure-mode) alias a node ultimately connects to
    /// by walking the `connection` parent links upward.  Returns `None` when the
    /// chain does not terminate at a known failure mode (dangling or cyclic
    /// diagram).  `by_alias` is the precomputed [`alias_index`], so each step is
    /// O(1).
    fn root_for(&self, start: &FtaNode, by_alias: &HashMap<&str, &FtaNode>) -> Option<String> {
        let mut current = start;
        // Bound the walk by node count to defend against cyclic connections.
        for _ in 0..=self.nodes.len() {
            if current.kind == NodeKind::FailureMode {
                return Some(current.alias.clone());
            }
            let parent_alias = current.connection.as_deref()?;
            current = by_alias.get(parent_alias).copied()?;
        }
        None
    }

    /// Assemble the [`StubEvent`]s this diagram contributes to the generated
    /// TRLC stub package (see [`render_trlc_stub`]).  One entry per failure mode
    /// (with its `failure_modes`) and one per root cause reachable from a top
    /// event (with the `stub_name` of that failure mode); root causes that do
    /// not connect to any failure mode are dropped with a `warn!` naming the
    /// alias, diagram and line so a malformed fault tree is visible in the
    /// build log rather than quietly losing a root cause from the safety
    /// chain.
    pub fn stub_events(&self, puml_basename: &str) -> Result<Vec<StubEvent>, FtaError> {
        let by_alias = self.alias_index();
        let mut out = Vec::new();

        for te in self.iter_kind(NodeKind::FailureMode) {
            let line = te.line.unwrap_or(0);
            // `te.alias` is `te.failure_modes[0]`, already validated as a TRLC
            // fully-qualified name by `from_procedure_file`, so its last
            // segment (see `stub_name`) is guaranteed to be a valid identifier.
            let name = stub_name(&te.alias);
            out.push(StubEvent {
                kind: NodeKind::FailureMode,
                stub_name: name.to_string(),
                title: te.name.clone().unwrap_or_default(),
                diagram: puml_basename.to_string(),
                line,
                failure_modes: te.failure_modes.clone(),
                fta_failure_modes: Vec::new(),
            });
        }

        for be in self.iter_kind(NodeKind::RootCause) {
            let line = be.line.unwrap_or(0);
            if !is_valid_identifier(&be.alias) {
                return Err(FtaError::InvalidRootCauseAlias {
                    diagram: puml_basename.to_string(),
                    line,
                    alias: be.alias.clone(),
                });
            }
            let Some(root_alias) = self.root_for(be, &by_alias) else {
                warn!(
                    "FTA {}:{}: root cause {:?} does not connect to any failure mode; \
                     it is dropped from the generated TRLC stub",
                    puml_basename, line, be.alias,
                );
                continue;
            };
            out.push(StubEvent {
                kind: NodeKind::RootCause,
                stub_name: be.alias.clone(),
                title: be.name.clone().unwrap_or_default(),
                diagram: puml_basename.to_string(),
                line,
                failure_modes: Vec::new(),
                fta_failure_modes: vec![stub_name(&root_alias).to_string()],
            });
        }

        Ok(out)
    }
}

/// Merge [`StubEvent`]s collected across every diagram of one `fta_package`
/// into a single, deduplicated set (one entry per `stub_name`): a root cause
/// shared by several diagrams keeps a single record with its `fta_failure_modes`
/// merged. Returns an error when the same `stub_name` is used for both a top
/// and a root cause, or with two different titles.
pub fn merge_stub_events(events: Vec<StubEvent>) -> Result<Vec<StubEvent>, FtaError> {
    let mut merged: HashMap<String, StubEvent> = HashMap::new();
    for ev in events {
        match merged.get_mut(&ev.stub_name) {
            None => {
                merged.insert(ev.stub_name.clone(), ev);
            }
            Some(existing) => {
                if existing.kind != ev.kind {
                    return Err(FtaError::StubKindClash {
                        stub_name: ev.stub_name,
                        first_diagram: existing.diagram.clone(),
                        second_diagram: ev.diagram,
                    });
                }
                if existing.title != ev.title {
                    return Err(FtaError::StubTitleMismatch {
                        stub_name: ev.stub_name,
                        first_diagram: existing.diagram.clone(),
                        first_title: existing.title.clone(),
                        second_diagram: ev.diagram,
                        second_title: ev.title,
                    });
                }
                for fm in ev.failure_modes {
                    if !existing.failure_modes.contains(&fm) {
                        existing.failure_modes.push(fm);
                    }
                }
                for te in ev.fta_failure_modes {
                    if !existing.fta_failure_modes.contains(&te) {
                        existing.fta_failure_modes.push(te);
                    }
                }
            }
        }
    }

    let mut result: Vec<StubEvent> = merged.into_values().collect();
    result.sort_by(|a, b| {
        let rank = |k: NodeKind| if k == NodeKind::FailureMode { 0 } else { 1 };
        (rank(a.kind), &a.stub_name).cmp(&(rank(b.kind), &b.stub_name))
    });
    Ok(result)
}

/// Escape a Rust string as a TRLC string literal (including the surrounding
/// quotes).
fn trlc_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Render merged [`StubEvent`]s (see [`merge_stub_events`]) as one TRLC file:
/// a `FailureMode`/`RootCause` record per event, importing `ScoreReq` plus every
/// package referenced by a `failure_modes` fully-qualified name. Output order
/// is deterministic (see [`merge_stub_events`]) and every record carries a
/// `<diagram>:<line>` comment pointing back at its source macro call.
pub fn render_trlc_stub(package: &str, events: &[StubEvent]) -> String {
    let mut fm_packages: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for ev in events {
        for fm in &ev.failure_modes {
            if let Some((pkg, _)) = fm.rsplit_once('.') {
                fm_packages.insert(pkg);
            }
        }
    }

    let mut out = String::new();
    out.push_str("// GENERATED by puml_cli from FTA diagrams -- do not edit\n");
    out.push_str(&format!("package {}\n\n", package));
    out.push_str("import ScoreReq\n");
    for pkg in fm_packages {
        out.push_str(&format!("import {}\n", pkg));
    }
    out.push('\n');

    for ev in events {
        out.push_str(&format!("// {}:{}\n", ev.diagram, ev.line));
        match ev.kind {
            NodeKind::FailureMode => {
                out.push_str(&format!(
                    "ScoreReq.FtaFailureMode {} {{\n  title = {}\n  diagram = {}\n  line = {}\n  failure_modes = [{}]\n}}\n\n",
                    ev.stub_name,
                    trlc_string_literal(&ev.title),
                    trlc_string_literal(&ev.diagram),
                    ev.line,
                    ev.failure_modes.join(", "),
                ));
            }
            NodeKind::RootCause => {
                out.push_str(&format!(
                    "ScoreReq.RootCause {} {{\n  title = {}\n  diagram = {}\n  line = {}\n  failure_modes = [{}]\n}}\n\n",
                    ev.stub_name,
                    trlc_string_literal(&ev.title),
                    trlc_string_literal(&ev.diagram),
                    ev.line,
                    ev.fta_failure_modes.join(", "),
                ));
            }
            _ => unreachable!("StubEvent::kind is always FailureMode or RootCause"),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser_core::DiagramParser;
    use procedure_preprocessor::{
        Arg, MacroCallDef, ProcedureFile, ProcedureParserService, Statement,
    };
    use puml_utils::LogLevel;
    use std::path::PathBuf;
    use std::rc::Rc;

    fn mk_call(name: &str, args: &[&str], line: usize) -> Statement {
        Statement::MacroCall(MacroCallDef {
            name: name.to_string(),
            args: args.iter().map(|a| Arg::String(a.to_string())).collect(),
            line: Some(line),
        })
    }

    fn model_from_stmts(stmts: Vec<Statement>) -> FtaModel {
        FtaModel::from_procedure_file(&ProcedureFile { stmts }).expect("fta model")
    }

    const SAMPLE: &str = r#"
!procedure $FailureMode($name, $alias)
  rectangle "$name" as $alias
!endprocedure
!procedure $IntermediateEvent($name, $alias, $connection)
  rectangle "$name" as $alias
!endprocedure
!procedure $RootCause($name, $alias, $connection)
  usecase "$name" as $alias
!endprocedure
!procedure $OrGate($alias, $connection)
  rectangle " " as $alias
!endprocedure
!procedure $AndGate($alias, $connection)
  rectangle " " as $alias
!endprocedure
$FailureMode("SampleFailureMode takes over the world", "SampleLibrary.SampleFailureMode")
$OrGate("OG1", "SampleLibrary.SampleFailureMode")
$IntermediateEvent("SampleFailureMode is Angry", "IEF", "OG1")
$RootCause("Just bad luck", "JustBadLuck", "OG1")
$AndGate("AG2", "IEF")
$RootCause("No More Cookies", "NoMoreCookies", "AG2")
$RootCause("No More Coffee", "NoMoreCoffee", "AG2")
"#;

    fn model_from(content: &str) -> FtaModel {
        let path = Rc::new(PathBuf::from("sample_fta.puml"));
        let parsed = ProcedureParserService
            .parse_file(&path, content, LogLevel::Warn)
            .expect("procedure parse");
        FtaModel::from_procedure_file(&parsed).expect("fta model")
    }

    #[test]
    fn builds_all_topology_nodes() {
        let model = model_from(SAMPLE);
        assert_eq!(model.iter_kind(NodeKind::FailureMode).count(), 1);
        assert_eq!(model.iter_kind(NodeKind::RootCause).count(), 3);
        assert_eq!(model.iter_kind(NodeKind::Gate).count(), 2);
        assert_eq!(model.iter_kind(NodeKind::IntermediateEvent).count(), 1);
    }

    #[test]
    fn stub_events_groups_root_causes_under_failure_mode() {
        let model = model_from(SAMPLE);
        let stubs = model.stub_events("sample_fta.puml").expect("stub events");
        let mut root_causes: Vec<&str> = stubs
            .iter()
            .filter(|e| e.kind == NodeKind::RootCause)
            .map(|e| e.stub_name.as_str())
            .collect();
        root_causes.sort_unstable();
        assert_eq!(
            root_causes,
            vec!["JustBadLuck", "NoMoreCoffee", "NoMoreCookies"]
        );
        assert!(stubs
            .iter()
            .filter(|e| e.kind == NodeKind::RootCause)
            .all(|e| e.fta_failure_modes == vec!["SampleFailureMode".to_string()]));
    }

    #[test]
    fn is_valid_trlc_fqn_matches_package_record() {
        assert!(is_valid_trlc_fqn("Pkg.Record"));
        assert!(is_valid_trlc_fqn("_Lib.Foo_1"));
        assert!(!is_valid_trlc_fqn("NoDot"));
        assert!(!is_valid_trlc_fqn("A.B.C"));
        assert!(!is_valid_trlc_fqn(""));
        assert!(!is_valid_trlc_fqn("."));
        assert!(!is_valid_trlc_fqn("1Bad.Record"));
    }

    #[test]
    fn multiple_fta_failure_modes_produce_independent_stub_entries() {
        let model = model_from_stmts(vec![
            mk_call(FAILURE_MODE, &["FM one", "Lib.FmA"], 1),
            mk_call(OR_GATE, &["OGA", "Lib.FmA"], 2),
            mk_call(ROOT_CAUSE, &["cm a", "CmA", "OGA"], 3),
            mk_call(FAILURE_MODE, &["FM two", "Lib.FmB"], 4),
            mk_call(OR_GATE, &["OGB", "Lib.FmB"], 5),
            mk_call(ROOT_CAUSE, &["cm b", "CmB", "OGB"], 6),
        ]);
        let stubs = model.stub_events("d.puml").expect("stub events");
        let a = stubs.iter().find(|e| e.stub_name == "CmA").unwrap();
        assert_eq!(a.fta_failure_modes, vec!["FmA".to_string()]);
        let b = stubs.iter().find(|e| e.stub_name == "CmB").unwrap();
        assert_eq!(b.fta_failure_modes, vec!["FmB".to_string()]);
    }

    #[test]
    fn cyclic_connections_terminate_without_hanging() {
        // G1 -> G2 -> G1 cycle, no reachable failure mode.  The bounded walk must
        // return without looping forever, and the root cause is dropped.
        let model = model_from_stmts(vec![
            mk_call(OR_GATE, &["G1", "G2"], 1),
            mk_call(OR_GATE, &["G2", "G1"], 2),
            mk_call(ROOT_CAUSE, &["cm", "Cm", "G1"], 3),
        ]);
        let stubs = model.stub_events("d.puml").expect("stub events");
        assert!(stubs.is_empty());
    }

    #[test]
    fn missing_argument_is_an_error() {
        let file = ProcedureFile {
            stmts: vec![mk_call(FAILURE_MODE, &["only name"], 1)],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(err, FtaError::MissingArgs { .. }));
    }

    #[test]
    fn non_string_argument_is_an_error() {
        let file = ProcedureFile {
            stmts: vec![Statement::MacroCall(MacroCallDef {
                name: FAILURE_MODE.to_string(),
                args: vec![Arg::Number(1), Arg::String("Lib.Fm".to_string())],
                line: Some(1),
            })],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(err, FtaError::NonStringArg { index: 0, .. }));
    }

    #[test]
    fn duplicate_alias_last_write_wins_and_root_cause_is_attributed_correctly() {
        // Two failure modes happen to share the same failure-mode alias; only
        // the last one is reachable via `alias_index`, so the root cause's
        // ancestry resolves through it regardless of which node "wins".
        let model = model_from_stmts(vec![
            mk_call(FAILURE_MODE, &["FM first", "Lib.Fm"], 1),
            mk_call(FAILURE_MODE, &["FM second", "Lib.Fm"], 2),
            mk_call(OR_GATE, &["OG", "Lib.Fm"], 3),
            mk_call(ROOT_CAUSE, &["cm", "Cm", "OG"], 4),
        ]);
        let stubs = model.stub_events("d.puml").expect("stub events");
        let top_names: Vec<&str> = stubs
            .iter()
            .filter(|e| e.kind == NodeKind::FailureMode)
            .map(|e| e.stub_name.as_str())
            .collect();
        assert_eq!(top_names, vec!["Fm", "Fm"]);
        let be = stubs
            .iter()
            .find(|e| e.kind == NodeKind::RootCause)
            .unwrap();
        assert_eq!(be.stub_name, "Cm");
        assert_eq!(be.fta_failure_modes, vec!["Fm".to_string()]);
    }

    #[test]
    fn failure_mode_alias_equals_first_failure_mode() {
        let model = model_from_stmts(vec![mk_call(FAILURE_MODE, &["FM", "Lib.Fm"], 1)]);
        let te = model.iter_kind(NodeKind::FailureMode).next().unwrap();
        assert_eq!(te.alias, "Lib.Fm");
        assert_eq!(te.failure_modes, vec!["Lib.Fm".to_string()]);
    }

    #[test]
    fn failure_mode_with_multiple_failure_modes_dedups_and_keeps_order() {
        let model = model_from_stmts(vec![mk_call(
            FAILURE_MODE,
            &["Top", "Lib.A", "Lib.B", "Lib.A"],
            1,
        )]);
        let te = model.iter_kind(NodeKind::FailureMode).next().unwrap();
        assert_eq!(te.alias, "Lib.A");
        assert_eq!(
            te.failure_modes,
            vec!["Lib.A".to_string(), "Lib.B".to_string()]
        );
    }

    #[test]
    fn failure_mode_rejects_invalid_failure_mode_fqn() {
        let file = ProcedureFile {
            stmts: vec![mk_call(FAILURE_MODE, &["Top", "NotDotted"], 2)],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(
            err,
            FtaError::InvalidFailureModeFqn { line: 2, .. }
        ));
    }

    #[test]
    fn stub_events_collapses_failure_mode_alias_to_last_segment() {
        let model = model_from_stmts(vec![
            mk_call(FAILURE_MODE, &["FM", "Lib.Fm"], 1),
            mk_call(OR_GATE, &["OG", "Lib.Fm"], 2),
            mk_call(ROOT_CAUSE, &["cm", "Cm", "OG"], 3),
        ]);
        let stubs = model.stub_events("d.puml").expect("stub events");
        let te = stubs
            .iter()
            .find(|e| e.kind == NodeKind::FailureMode)
            .unwrap();
        assert_eq!(te.stub_name, "Fm");
        assert_eq!(te.failure_modes, vec!["Lib.Fm".to_string()]);
        let be = stubs
            .iter()
            .find(|e| e.kind == NodeKind::RootCause)
            .unwrap();
        assert_eq!(be.stub_name, "Cm");
        assert_eq!(be.fta_failure_modes, vec!["Fm".to_string()]);
    }

    #[test]
    fn stub_events_new_style_diagram_uses_plain_names() {
        let model = model_from_stmts(vec![
            mk_call(FAILURE_MODE, &["Top", "Lib.A", "Lib.B"], 1),
            mk_call(OR_GATE, &["OG", "Lib.A"], 4),
            mk_call(ROOT_CAUSE, &["Root", "TooBig", "OG"], 5),
        ]);
        let stubs = model.stub_events("d.puml").expect("stub events");
        let te = stubs
            .iter()
            .find(|e| e.kind == NodeKind::FailureMode)
            .unwrap();
        assert_eq!(te.stub_name, "A");
        assert_eq!(
            te.failure_modes,
            vec!["Lib.A".to_string(), "Lib.B".to_string()]
        );
        let be = stubs
            .iter()
            .find(|e| e.kind == NodeKind::RootCause)
            .unwrap();
        assert_eq!(be.stub_name, "TooBig");
        assert_eq!(be.fta_failure_modes, vec!["A".to_string()]);
    }

    #[test]
    fn root_cause_rejects_dotted_alias() {
        let model = model_from_stmts(vec![
            mk_call(FAILURE_MODE, &["FM", "Lib.Fm"], 1),
            mk_call(OR_GATE, &["OG", "Lib.Fm"], 2),
            mk_call(ROOT_CAUSE, &["cm", "Lib.Cm", "OG"], 3),
        ]);
        let err = model.stub_events("d.puml").unwrap_err();
        assert!(matches!(
            err,
            FtaError::InvalidRootCauseAlias { line: 3, .. }
        ));
    }

    #[test]
    fn stub_events_drops_dangling_root_cause_with_warning() {
        let model = model_from_stmts(vec![
            mk_call(FAILURE_MODE, &["FM", "Lib.Fm"], 1),
            mk_call(ROOT_CAUSE, &["cm", "Cm", "MissingGate"], 2),
        ]);
        let stubs = model.stub_events("d.puml").expect("stub events");
        assert!(!stubs.iter().any(|e| e.kind == NodeKind::RootCause));
    }

    #[test]
    fn merge_stub_events_unions_fta_failure_modes_for_shared_root_cause() {
        let a = StubEvent {
            kind: NodeKind::RootCause,
            stub_name: "Shared".into(),
            title: "shared root cause".into(),
            diagram: "a.puml".into(),
            line: 1,
            failure_modes: vec![],
            fta_failure_modes: vec!["TeA".into()],
        };
        let b = StubEvent {
            fta_failure_modes: vec!["TeB".into()],
            diagram: "b.puml".into(),
            line: 2,
            ..a.clone()
        };
        let merged = merge_stub_events(vec![a, b]).expect("merge");
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].fta_failure_modes,
            vec!["TeA".to_string(), "TeB".to_string()]
        );
    }

    #[test]
    fn merge_stub_events_errors_on_kind_clash() {
        let te = StubEvent {
            kind: NodeKind::FailureMode,
            stub_name: "X".into(),
            title: "t".into(),
            diagram: "a.puml".into(),
            line: 1,
            failure_modes: vec!["Lib.Fm".into()],
            fta_failure_modes: vec![],
        };
        let be = StubEvent {
            kind: NodeKind::RootCause,
            diagram: "b.puml".into(),
            line: 2,
            failure_modes: vec![],
            fta_failure_modes: vec!["Other".into()],
            ..te.clone()
        };
        let err = merge_stub_events(vec![te, be]).unwrap_err();
        assert!(matches!(err, FtaError::StubKindClash { .. }));
    }

    #[test]
    fn merge_stub_events_errors_on_title_mismatch() {
        let a = StubEvent {
            kind: NodeKind::RootCause,
            stub_name: "X".into(),
            title: "one title".into(),
            diagram: "a.puml".into(),
            line: 1,
            failure_modes: vec![],
            fta_failure_modes: vec!["Te".into()],
        };
        let b = StubEvent {
            title: "different title".into(),
            diagram: "b.puml".into(),
            line: 2,
            ..a.clone()
        };
        let err = merge_stub_events(vec![a, b]).unwrap_err();
        assert!(matches!(err, FtaError::StubTitleMismatch { .. }));
    }

    #[test]
    fn render_trlc_stub_emits_package_imports_and_records() {
        let events = vec![
            StubEvent {
                kind: NodeKind::FailureMode,
                stub_name: "TeOne".into(),
                title: "top \"quoted\"".into(),
                diagram: "d.puml".into(),
                line: 4,
                failure_modes: vec!["Lib.A".into(), "Other.B".into()],
                fta_failure_modes: vec![],
            },
            StubEvent {
                kind: NodeKind::RootCause,
                stub_name: "BeOne".into(),
                title: "root cause".into(),
                diagram: "d.puml".into(),
                line: 8,
                failure_modes: vec![],
                fta_failure_modes: vec!["TeOne".into()],
            },
        ];
        let out = render_trlc_stub("MyFta", &events);
        assert!(out.contains("package MyFta"));
        assert!(out.contains("import ScoreReq"));
        assert!(out.contains("import Lib"));
        assert!(out.contains("import Other"));
        assert!(out.contains("ScoreReq.FtaFailureMode TeOne {"));
        assert!(out.contains("failure_modes = [Lib.A, Other.B]"));
        assert!(out.contains("top \\\"quoted\\\""));
        assert!(out.contains("ScoreReq.RootCause BeOne {"));
        assert!(out.contains("failure_modes = [TeOne]"));
        assert!(out.contains("// d.puml:4"));
    }
}
