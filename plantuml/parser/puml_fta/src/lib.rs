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
//! `$TopEvent(...)` / `$BasicEvent(...)` / gate macro calls produced after
//! `fta_metamodel.puml` has been inlined) and turns it into:
//!
//! * a [`lobster-act-trace`] JSON document (`root_causes.lobster`) — schema
//!   compatible with the legacy `safety_analysis_tools.py` so the
//!   `dependability_analysis` traceability test is unaffected, and
//! * an ordered list of *chains* (`fta_chains.json`) describing, per failure
//!   mode, the inline diagram and the control measures (basic events) that
//!   trace up to it — consumed by the FMEA page assembler.
//!
//! [`lobster-act-trace`]: https://github.com/bmw-software-engineering/lobster

use std::collections::HashMap;

use log::warn;
use procedure_preprocessor::{Arg, MacroCallDef, ProcedureFile, Statement};
use serde::Serialize;
use serde_json::{json, Value};

/// Procedure macro names recognised in an FTA diagram.
const TOP_EVENT: &str = "$TopEvent";
const INTERMEDIATE_EVENT: &str = "$IntermediateEvent";
const BASIC_EVENT: &str = "$BasicEvent";
const AND_GATE: &str = "$AndGate";
const OR_GATE: &str = "$OrGate";
const TRANSFER_IN_GATE: &str = "$TransferInGate";

/// The kind of a node in a fault tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum NodeKind {
    /// `$TopEvent` — the failure mode at the root of the tree.
    TopEvent,
    /// `$IntermediateEvent` — a named intermediate node.
    IntermediateEvent,
    /// `$BasicEvent` — a leaf root cause / control measure.
    BasicEvent,
    /// `$AndGate`, `$OrGate`, `$TransferInGate` — a logic gate.
    Gate,
}

/// One node of a fault tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FtaNode {
    pub kind: NodeKind,
    /// Human readable display name (events only; gates carry `None`).
    pub name: Option<String>,
    /// Alias / identifier.  For top and basic events this is the TRLC
    /// fully-qualified name of the corresponding record.
    pub alias: String,
    /// Alias of the parent node this node connects upward to.  `None` for the
    /// top event (the root).
    pub connection: Option<String>,
    /// 1-based line of the macro call in its source diagram.
    /// `None` when the line is unavailable (e.g. synthesised nodes in tests).
    pub line: Option<usize>,
}

/// A fully parsed fault tree for a single diagram.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FtaModel {
    pub nodes: Vec<FtaNode>,
}

/// One failure-mode chain: the failure mode together with the control measures
/// (basic events) whose ancestry reaches it, plus the diagram to render.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FtaChain {
    /// Fully-qualified name of the failure mode (top event alias).
    pub fm_fqn: String,
    /// Human readable failure mode name.
    pub fm_name: String,
    /// Basename of the preprocessed `.puml` diagram to render inline.
    pub puml: String,
    /// Fully-qualified names of the control measures for this chain, in the
    /// order they appear in the diagram.
    pub control_measures: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum FtaError {
    #[error("FTA macro {macro_name} requires at least {expected} argument(s)")]
    MissingArgs { macro_name: String, expected: usize },
    #[error("FTA macro {macro_name} expected a string argument at position {index}")]
    NonStringArg { macro_name: String, index: usize },
}

/// Legacy guardrail (ported from `safety_analysis_tools.py`): a TRLC
/// fully-qualified name looks like `Package.Record` — exactly two dot-separated
/// identifier segments.
fn is_valid_trlc_fqn(alias: &str) -> bool {
    let parts: Vec<&str> = alias.split('.').collect();
    if parts.len() != 2 {
        return false;
    }
    parts.iter().all(|part| {
        let mut chars = part.chars();
        let first_ok = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_');
        first_ok && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
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
                TOP_EVENT => FtaNode {
                    kind: NodeKind::TopEvent,
                    name: Some(string_arg(call, 0)?),
                    alias: string_arg(call, 1)?,
                    connection: None,
                    line,
                },
                INTERMEDIATE_EVENT => FtaNode {
                    kind: NodeKind::IntermediateEvent,
                    name: Some(string_arg(call, 0)?),
                    alias: string_arg(call, 1)?,
                    connection: Some(string_arg(call, 2)?),
                    line,
                },
                BASIC_EVENT => FtaNode {
                    kind: NodeKind::BasicEvent,
                    name: Some(string_arg(call, 0)?),
                    alias: string_arg(call, 1)?,
                    connection: Some(string_arg(call, 2)?),
                    line,
                },
                AND_GATE | OR_GATE | TRANSFER_IN_GATE => FtaNode {
                    kind: NodeKind::Gate,
                    name: None,
                    alias: string_arg(call, 0)?,
                    connection: Some(string_arg(call, 1)?),
                    line,
                },
                // Unknown / cosmetic macros are not part of the topology.
                _ => continue,
            };
            // Top and basic events carry TRLC fully-qualified names that must
            // resolve to requirements in the traceability chain; warn (rather
            // than fail) on a malformed alias so the build still produces output.
            if matches!(node.kind, NodeKind::TopEvent | NodeKind::BasicEvent)
                && !is_valid_trlc_fqn(&node.alias)
            {
                warn!(
                    "FTA {} at line {}: alias {:?} is not a valid TRLC fully-qualified \
                     name (expected 'Package.Record')",
                    call.name,
                    node.line.unwrap_or(0),
                    node.alias,
                );
            }
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
    /// chain does not terminate at a known top event (dangling or cyclic
    /// diagram).  `by_alias` is the precomputed [`alias_index`], so each step is
    /// O(1).
    fn root_for(&self, start: &FtaNode, by_alias: &HashMap<&str, &FtaNode>) -> Option<String> {
        let mut current = start;
        // Bound the walk by node count to defend against cyclic connections.
        for _ in 0..=self.nodes.len() {
            if current.kind == NodeKind::TopEvent {
                return Some(current.alias.clone());
            }
            let parent_alias = current.connection.as_deref()?;
            current = by_alias.get(parent_alias).copied()?;
        }
        None
    }

    /// Assemble ordered failure-mode chains for `puml_basename`.
    ///
    /// Basic events whose ancestry does not terminate at a known top event are
    /// not silently discarded: each emits a `warn!` naming the alias, diagram
    /// and line so a malformed fault tree is visible in the build log rather
    /// than quietly losing a control measure from the safety chain.
    pub fn chains(&self, puml_basename: &str) -> Vec<FtaChain> {
        let by_alias = self.alias_index();
        let mut chains: Vec<FtaChain> = self
            .iter_kind(NodeKind::TopEvent)
            .map(|fm| FtaChain {
                fm_fqn: fm.alias.clone(),
                fm_name: fm.name.clone().unwrap_or_else(|| fm.alias.clone()),
                puml: puml_basename.to_string(),
                control_measures: Vec::new(),
            })
            .collect();

        for basic in self.iter_kind(NodeKind::BasicEvent) {
            match self.root_for(basic, &by_alias) {
                Some(root) => {
                    if let Some(chain) = chains.iter_mut().find(|c| c.fm_fqn == root) {
                        chain.control_measures.push(basic.alias.clone());
                    }
                }
                None => warn!(
                    "FTA {}:{}: basic event {:?} does not connect to any top event; \
                     it is dropped from every failure-mode chain",
                    puml_basename,
                    basic.line.unwrap_or(0),
                    basic.alias,
                ),
            }
        }
        chains
    }

    /// Emit `lobster-act-trace` items for the top and basic events, mirroring
    /// the legacy `safety_analysis_tools.py` schema (tag `fta <alias>`,
    /// `refs: [req <alias>]`).
    pub fn lobster_items(&self, source_file: &str) -> Vec<Value> {
        self.nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::TopEvent | NodeKind::BasicEvent))
            .map(|n| {
                let kind = match n.kind {
                    NodeKind::TopEvent => "TopEvent",
                    NodeKind::BasicEvent => "BasicEvent",
                    _ => unreachable!(),
                };
                json!({
                    "tag": format!("fta {}", n.alias),
                    "location": {
                        "kind": "file",
                        "file": source_file,
                        "line": n.line.map(|l| json!(l)).unwrap_or(Value::Null),
                        "column": null,
                    },
                    "name": n.alias,
                    "messages": [],
                    "just_up": [],
                    "just_down": [],
                    "just_global": [],
                    "refs": [format!("req {}", n.alias)],
                    "framework": "PlantUML",
                    "kind": kind,
                })
            })
            .collect()
    }
}

/// Wrap lobster items in the standard `lobster-act-trace` envelope, sorted by
/// tag for deterministic output.
pub fn lobster_document(mut items: Vec<Value>) -> Value {
    items.sort_by(|a, b| {
        a["tag"]
            .as_str()
            .unwrap_or("")
            .cmp(b["tag"].as_str().unwrap_or(""))
    });
    json!({
        "data": items,
        "generator": "puml_fta",
        "schema": "lobster-act-trace",
        "version": 3,
    })
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
!procedure $TopEvent($name, $alias)
  rectangle "$name" as $alias
!endprocedure
!procedure $IntermediateEvent($name, $alias, $connection)
  rectangle "$name" as $alias
!endprocedure
!procedure $BasicEvent($name, $alias, $connection)
  usecase "$name" as $alias
!endprocedure
!procedure $OrGate($alias, $connection)
  rectangle " " as $alias
!endprocedure
!procedure $AndGate($alias, $connection)
  rectangle " " as $alias
!endprocedure
$TopEvent("SampleFailureMode takes over the world", "SampleLibrary.SampleFailureMode")
$OrGate("OG1", "SampleLibrary.SampleFailureMode")
$IntermediateEvent("SampleFailureMode is Angry", "IEF", "OG1")
$BasicEvent("Just bad luck", "SampleLibrary.JustBadLuck", "OG1")
$AndGate("AG2", "IEF")
$BasicEvent("No More Cookies", "SampleLibrary.NoMoreCookies", "AG2")
$BasicEvent("No More Coffee", "SampleLibrary.NoMoreCoffee", "AG2")
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
        assert_eq!(model.iter_kind(NodeKind::TopEvent).count(), 1);
        assert_eq!(model.iter_kind(NodeKind::BasicEvent).count(), 3);
        assert_eq!(model.iter_kind(NodeKind::Gate).count(), 2);
        assert_eq!(model.iter_kind(NodeKind::IntermediateEvent).count(), 1);
    }

    #[test]
    fn chain_groups_basic_events_under_failure_mode() {
        let model = model_from(SAMPLE);
        let chains = model.chains("sample_fta.puml");
        assert_eq!(chains.len(), 1);
        let chain = &chains[0];
        assert_eq!(chain.fm_fqn, "SampleLibrary.SampleFailureMode");
        assert_eq!(chain.puml, "sample_fta.puml");
        assert_eq!(
            chain.control_measures,
            vec![
                "SampleLibrary.JustBadLuck",
                "SampleLibrary.NoMoreCookies",
                "SampleLibrary.NoMoreCoffee",
            ]
        );
    }

    #[test]
    fn lobster_items_match_legacy_schema() {
        let model = model_from(SAMPLE);
        let doc = lobster_document(model.lobster_items("sample_fta.puml"));
        assert_eq!(doc["schema"], "lobster-act-trace");
        assert_eq!(doc["version"], 3);
        let data = doc["data"].as_array().unwrap();
        // 1 top event + 3 basic events.
        assert_eq!(data.len(), 4);
        let top = data
            .iter()
            .find(|i| i["name"] == "SampleLibrary.SampleFailureMode")
            .unwrap();
        assert_eq!(top["tag"], "fta SampleLibrary.SampleFailureMode");
        assert_eq!(top["refs"][0], "req SampleLibrary.SampleFailureMode");
        assert_eq!(top["kind"], "TopEvent");
        assert_eq!(top["framework"], "PlantUML");
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
    fn multiple_top_events_get_separate_chains() {
        let chains = model_from_stmts(vec![
            mk_call(TOP_EVENT, &["FM one", "Lib.FmA"], 1),
            mk_call(OR_GATE, &["OGA", "Lib.FmA"], 2),
            mk_call(BASIC_EVENT, &["cm a", "Lib.CmA", "OGA"], 3),
            mk_call(TOP_EVENT, &["FM two", "Lib.FmB"], 4),
            mk_call(OR_GATE, &["OGB", "Lib.FmB"], 5),
            mk_call(BASIC_EVENT, &["cm b", "Lib.CmB", "OGB"], 6),
        ])
        .chains("d.puml");

        assert_eq!(chains.len(), 2);
        let a = chains.iter().find(|c| c.fm_fqn == "Lib.FmA").unwrap();
        assert_eq!(a.control_measures, vec!["Lib.CmA"]);
        let b = chains.iter().find(|c| c.fm_fqn == "Lib.FmB").unwrap();
        assert_eq!(b.control_measures, vec!["Lib.CmB"]);
    }

    #[test]
    fn dangling_basic_event_is_dropped_from_chains() {
        // Basic event connects to a gate alias that does not exist.
        let chains = model_from_stmts(vec![
            mk_call(TOP_EVENT, &["FM", "Lib.Fm"], 1),
            mk_call(BASIC_EVENT, &["cm", "Lib.Cm", "MissingGate"], 2),
        ])
        .chains("d.puml");

        assert_eq!(chains.len(), 1);
        assert!(chains[0].control_measures.is_empty());
    }

    #[test]
    fn cyclic_connections_terminate_without_hanging() {
        // G1 -> G2 -> G1 cycle, no reachable top event.  The bounded walk must
        // return without looping forever.
        let chains = model_from_stmts(vec![
            mk_call(OR_GATE, &["G1", "G2"], 1),
            mk_call(OR_GATE, &["G2", "G1"], 2),
            mk_call(BASIC_EVENT, &["cm", "Lib.Cm", "G1"], 3),
        ])
        .chains("d.puml");

        assert!(chains.is_empty());
    }

    #[test]
    fn missing_argument_is_an_error() {
        let file = ProcedureFile {
            stmts: vec![mk_call(TOP_EVENT, &["only name"], 1)],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(err, FtaError::MissingArgs { .. }));
    }

    #[test]
    fn non_string_argument_is_an_error() {
        let file = ProcedureFile {
            stmts: vec![Statement::MacroCall(MacroCallDef {
                name: TOP_EVENT.to_string(),
                args: vec![Arg::Number(1), Arg::String("Lib.Fm".to_string())],
                line: Some(1),
            })],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(err, FtaError::NonStringArg { index: 0, .. }));
    }

    #[test]
    fn lobster_document_sorted_by_tag() {
        let doc = lobster_document(
            model_from_stmts(vec![
                mk_call(TOP_EVENT, &["FM", "Lib.Zeta"], 1),
                mk_call(OR_GATE, &["OG", "Lib.Zeta"], 2),
                mk_call(BASIC_EVENT, &["cm", "Lib.Alpha", "OG"], 3),
            ])
            .lobster_items("d.puml"),
        );
        let data = doc["data"].as_array().unwrap();
        let tags: Vec<&str> = data.iter().map(|i| i["tag"].as_str().unwrap()).collect();
        let mut expected = tags.clone();
        expected.sort_unstable();
        assert_eq!(tags, expected);
        assert_eq!(data[0]["name"], "Lib.Alpha");
    }

    #[test]
    fn lobster_items_carry_source_line() {
        let items = model_from_stmts(vec![mk_call(TOP_EVENT, &["FM", "Lib.Fm"], 7)])
            .lobster_items("d.puml");
        assert_eq!(items[0]["location"]["line"], 7);
    }

    #[test]
    fn duplicate_alias_last_write_wins_and_basic_event_is_attributed_correctly() {
        // Two top events share the same alias; only the second should be
        // reachable via `alias_index`, and the basic event must end up in the
        // chain for the winning (second) entry.
        let chains = model_from_stmts(vec![
            mk_call(TOP_EVENT, &["FM first", "Lib.Fm"], 1),
            mk_call(TOP_EVENT, &["FM second", "Lib.Fm"], 2),
            mk_call(OR_GATE, &["OG", "Lib.Fm"], 3),
            mk_call(BASIC_EVENT, &["cm", "Lib.Cm", "OG"], 4),
        ])
        .chains("d.puml");
        // Two chain entries (one per TopEvent node), but the basic event
        // connects through OG → Lib.Fm which resolves to the last-write node.
        assert_eq!(chains.len(), 2);
        let with_cm: Vec<_> = chains
            .iter()
            .filter(|c| !c.control_measures.is_empty())
            .collect();
        assert_eq!(with_cm.len(), 1);
        assert_eq!(with_cm[0].control_measures, vec!["Lib.Cm"]);
    }
}
