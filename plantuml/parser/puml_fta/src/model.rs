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

//! Per-diagram fault-tree model built from the FTA macro calls.

use std::collections::{HashMap, HashSet};

use procedure_preprocessor::{Arg, MacroCallDef, ProcedureFile, Statement};

/// Procedure macro names recognised in an FTA diagram.
pub(crate) const FAILURE_MODE: &str = "$FailureMode";
pub(crate) const INTERMEDIATE_EVENT: &str = "$IntermediateEvent";
pub(crate) const ROOT_CAUSE: &str = "$RootCause";
pub(crate) const AND_GATE: &str = "$AndGate";
pub(crate) const OR_GATE: &str = "$OrGate";
pub(crate) const TRANSFER_OUT_EVENT: &str = "$TransferOutEvent";
pub(crate) const TRANSFER_IN_GATE: &str = "$TransferInGate";

/// The kind of a node in a fault tree, with the data only that kind carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// `$FailureMode` — the root of a tree.  `failure_modes` are the TRLC
    /// fully-qualified names it covers.
    FailureMode {
        failure_modes: Vec<String>,
    },
    /// `$IntermediateEvent` — a named intermediate node.
    IntermediateEvent,
    /// `$RootCause` — a leaf root cause / control measure.
    RootCause,
    /// `$TransferOutEvent` — the root of a partial tree that other trees
    /// transfer into.
    TransferOutEvent,
    AndGate,
    OrGate,
    /// `$TransferInGate` — `target` is what it transfers into: either
    /// `<TransferOutEvent alias>`, `<Package>.<FailureMode>` or
    /// `<Package>.<FailureMode>.<IntermediateEvent alias>`.
    TransferInGate {
        target: String,
    },
}

impl NodeKind {
    /// The procedure macro that produces this kind of node.
    pub fn macro_name(&self) -> &'static str {
        match self {
            Self::FailureMode { .. } => FAILURE_MODE,
            Self::IntermediateEvent => INTERMEDIATE_EVENT,
            Self::RootCause => ROOT_CAUSE,
            Self::TransferOutEvent => TRANSFER_OUT_EVENT,
            Self::AndGate => AND_GATE,
            Self::OrGate => OR_GATE,
            Self::TransferInGate { .. } => TRANSFER_IN_GATE,
        }
    }
}

/// One node of a fault tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FtaNode {
    pub kind: NodeKind,
    /// Display text (`None` for `$AndGate`/`$OrGate`).
    pub name: Option<String>,
    /// Identifier, unique within its diagram.  For a failure mode it is the
    /// first of its failure modes (a TRLC fully-qualified name); for every
    /// other kind a plain identifier.
    pub alias: String,
    /// Aliases of the parent nodes this node connects upward to; several when
    /// the same node is called under several parents.  Empty for
    /// `$FailureMode` and `$TransferOutEvent` (the roots of a tree).
    pub connections: Vec<String>,
    /// 1-based line of the macro call in its source diagram (the first call of
    /// a repeated node); 0 when unknown.
    pub line: usize,
}

impl FtaNode {
    /// The failure modes a `$FailureMode` covers; empty for every other kind.
    pub fn failure_modes(&self) -> &[String] {
        match &self.kind {
            NodeKind::FailureMode { failure_modes } => failure_modes,
            _ => &[],
        }
    }

    /// What a `$TransferInGate` transfers into; `None` for every other kind.
    pub fn transfer_target(&self) -> Option<&str> {
        match &self.kind {
            NodeKind::TransferInGate { target } => Some(target),
            _ => None,
        }
    }
}

/// A fully parsed fault tree for a single diagram.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FtaModel {
    pub nodes: Vec<FtaNode>,
}

#[derive(Debug, thiserror::Error)]
pub enum FtaError {
    #[error("FTA macro {macro_name} at line {line} requires at least {expected} argument(s)")]
    MissingArgs {
        macro_name: String,
        line: usize,
        expected: usize,
    },
    #[error(
        "FTA macro {macro_name} at line {line}: expected a string argument at position {index}"
    )]
    NonStringArg {
        macro_name: String,
        line: usize,
        index: usize,
    },
    #[error(
        "FTA macro $FailureMode at line {line}: failure mode {fm_fqn:?} is not a valid TRLC \
         fully-qualified name (expected 'Package.Name')"
    )]
    InvalidFailureModeFqn { line: usize, fm_fqn: String },
    #[error(
        "FTA macro {macro_name} at line {line}: alias {alias:?} must be a plain identifier \
         ([A-Za-z_][A-Za-z0-9_]*, no dots)"
    )]
    InvalidAlias {
        macro_name: String,
        line: usize,
        alias: String,
    },
    #[error(
        "FTA macro $TransferInGate at line {line}: target {target:?} is not valid; expected \
         '<TransferOutEvent alias>', '<Package>.<FailureMode>' or \
         '<Package>.<FailureMode>.<IntermediateEvent alias>'"
    )]
    InvalidTransferTarget { line: usize, target: String },
    #[error(
        "FTA macro at line {line}: alias {alias:?} is already used at line {first_line} by a \
         different node; only an identical call may repeat an alias, to add another parent"
    )]
    DuplicateAlias {
        line: usize,
        first_line: usize,
        alias: String,
    },
    #[error(
        "FTA $TransferOutEvent alias {alias:?} is defined twice ({first} and {second}); the \
         alias must be unique within the diagrams of one fault_trees target"
    )]
    DuplicateTransferOutEvent {
        alias: String,
        first: String,
        second: String,
    },
    #[error(
        "FTA {diagram}:{line}: $TransferInGate {target:?} is ambiguous; it matches several \
         nodes ({candidates})"
    )]
    AmbiguousTransferTarget {
        diagram: String,
        line: usize,
        target: String,
        candidates: String,
    },
    #[error("FTA nodes form a cycle through connections and transfers ({cycle})")]
    Cycle { cycle: String },
}

/// A TRLC fully-qualified name: `Package.Record`, two identifier segments.
pub(crate) fn is_valid_trlc_fqn(name: &str) -> bool {
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() == 2 && parts.iter().all(|part| is_valid_identifier(part))
}

/// A single identifier: `[A-Za-z_][A-Za-z0-9_]*`.
pub(crate) fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    let first_ok = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_');
    first_ok && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// One to three dot-separated identifiers.
fn is_valid_transfer_target(target: &str) -> bool {
    let parts: Vec<&str> = target.split('.').collect();
    (1..=3).contains(&parts.len()) && parts.iter().all(|part| is_valid_identifier(part))
}

fn string_arg(call: &MacroCallDef, index: usize) -> Result<String, FtaError> {
    let line = call.line.unwrap_or(0);
    let arg = call.args.get(index).ok_or_else(|| FtaError::MissingArgs {
        macro_name: call.name.clone(),
        line,
        expected: index + 1,
    })?;
    match arg {
        Arg::String(s) => Ok(s.clone()),
        _ => Err(FtaError::NonStringArg {
            macro_name: call.name.clone(),
            line,
            index,
        }),
    }
}

fn identifier_arg(call: &MacroCallDef, index: usize) -> Result<String, FtaError> {
    let alias = string_arg(call, index)?;
    if is_valid_identifier(&alias) {
        Ok(alias)
    } else {
        Err(FtaError::InvalidAlias {
            macro_name: call.name.clone(),
            line: call.line.unwrap_or(0),
            alias,
        })
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
            let line = call.line.unwrap_or(0);

            let node = match call.name.as_str() {
                FAILURE_MODE => {
                    if call.args.len() < 2 {
                        return Err(FtaError::MissingArgs {
                            macro_name: call.name.clone(),
                            line,
                            expected: 2,
                        });
                    }
                    let name = string_arg(call, 0)?;
                    let mut failure_modes = Vec::with_capacity(call.args.len() - 1);
                    for i in 1..call.args.len() {
                        let fm_fqn = string_arg(call, i)?;
                        if !is_valid_trlc_fqn(&fm_fqn) {
                            return Err(FtaError::InvalidFailureModeFqn { line, fm_fqn });
                        }
                        if !failure_modes.contains(&fm_fqn) {
                            failure_modes.push(fm_fqn);
                        }
                    }
                    FtaNode {
                        alias: failure_modes[0].clone(),
                        kind: NodeKind::FailureMode { failure_modes },
                        name: Some(name),
                        connections: Vec::new(),
                        line,
                    }
                }
                INTERMEDIATE_EVENT | ROOT_CAUSE => FtaNode {
                    kind: if call.name == ROOT_CAUSE {
                        NodeKind::RootCause
                    } else {
                        NodeKind::IntermediateEvent
                    },
                    name: Some(string_arg(call, 0)?),
                    alias: identifier_arg(call, 1)?,
                    connections: vec![string_arg(call, 2)?],
                    line,
                },
                TRANSFER_OUT_EVENT => FtaNode {
                    kind: NodeKind::TransferOutEvent,
                    name: Some(string_arg(call, 0)?),
                    alias: identifier_arg(call, 1)?,
                    connections: Vec::new(),
                    line,
                },
                AND_GATE | OR_GATE => FtaNode {
                    kind: if call.name == AND_GATE {
                        NodeKind::AndGate
                    } else {
                        NodeKind::OrGate
                    },
                    name: None,
                    alias: identifier_arg(call, 0)?,
                    connections: vec![string_arg(call, 1)?],
                    line,
                },
                TRANSFER_IN_GATE => {
                    let target = string_arg(call, 1)?;
                    if !is_valid_transfer_target(&target) {
                        return Err(FtaError::InvalidTransferTarget { line, target });
                    }
                    FtaNode {
                        kind: NodeKind::TransferInGate { target },
                        name: Some(string_arg(call, 0)?),
                        alias: identifier_arg(call, 2)?,
                        connections: vec![string_arg(call, 3)?],
                        line,
                    }
                }
                // Unknown / cosmetic macros are not part of the topology.
                _ => continue,
            };
            nodes.push(node);
        }

        // PlantUML draws a repeated alias as one shape with an edge per call.
        let mut merged: Vec<FtaNode> = Vec::with_capacity(nodes.len());
        let mut index: HashMap<String, usize> = HashMap::new();
        for node in nodes {
            let Some(&i) = index.get(&node.alias) else {
                index.insert(node.alias.clone(), merged.len());
                merged.push(node);
                continue;
            };
            let first = &mut merged[i];
            if first.kind != node.kind || first.name != node.name || node.connections.is_empty() {
                return Err(FtaError::DuplicateAlias {
                    line: node.line,
                    first_line: first.line,
                    alias: node.alias,
                });
            }
            for connection in node.connections {
                if !first.connections.contains(&connection) {
                    first.connections.push(connection);
                }
            }
        }

        Ok(Self { nodes: merged })
    }

    /// Every node with a path of `connections` to a `$FailureMode` of this
    /// diagram, mapped from its alias to the `$FailureMode`s it reaches.
    /// Nodes whose chains dangle or loop are absent.
    pub fn root_failure_modes(&self) -> HashMap<&str, Vec<&FtaNode>> {
        let by_alias: HashMap<&str, &FtaNode> =
            self.nodes.iter().map(|n| (n.alias.as_str(), n)).collect();
        let mut roots = HashMap::new();
        for node in &self.nodes {
            let mut found = Vec::new();
            let mut seen = HashSet::new();
            let mut stack = vec![node];
            while let Some(current) = stack.pop() {
                if !seen.insert(current.alias.as_str()) {
                    continue;
                }
                if matches!(current.kind, NodeKind::FailureMode { .. }) {
                    found.push(current);
                    continue;
                }
                stack.extend(
                    current
                        .connections
                        .iter()
                        .filter_map(|alias| by_alias.get(alias.as_str()).copied()),
                );
            }
            if !found.is_empty() {
                roots.insert(node.alias.as_str(), found);
            }
        }
        roots
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;

    pub fn mk_call(name: &str, args: &[&str], line: usize) -> Statement {
        Statement::MacroCall(MacroCallDef {
            name: name.to_string(),
            args: args.iter().map(|a| Arg::String(a.to_string())).collect(),
            line: Some(line),
        })
    }

    pub fn model_from_stmts(stmts: Vec<Statement>) -> FtaModel {
        FtaModel::from_procedure_file(&ProcedureFile { stmts }).expect("fta model")
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::*;
    use super::*;
    use parser_core::DiagramParser;
    use procedure_preprocessor::ProcedureParserService;
    use puml_utils::LogLevel;
    use std::path::PathBuf;
    use std::rc::Rc;

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

    fn count(model: &FtaModel, is_kind: fn(&NodeKind) -> bool) -> usize {
        model.nodes.iter().filter(|n| is_kind(&n.kind)).count()
    }

    #[test]
    fn builds_all_topology_nodes() {
        let path = Rc::new(PathBuf::from("sample_fta.puml"));
        let parsed = ProcedureParserService
            .parse_file(&path, SAMPLE, LogLevel::Warn)
            .expect("procedure parse");
        let model = FtaModel::from_procedure_file(&parsed).expect("model");
        assert_eq!(
            count(&model, |k| matches!(k, NodeKind::FailureMode { .. })),
            1
        );
        assert_eq!(count(&model, |k| matches!(k, NodeKind::RootCause)), 3);
        assert_eq!(
            count(&model, |k| matches!(
                k,
                NodeKind::AndGate | NodeKind::OrGate
            )),
            2
        );
        assert_eq!(
            count(&model, |k| matches!(k, NodeKind::IntermediateEvent)),
            1
        );
    }

    #[test]
    fn macro_name_is_the_macro_that_builds_the_node() {
        for (name, args) in [
            (FAILURE_MODE, vec!["fm", "Lib.Fm"]),
            (INTERMEDIATE_EVENT, vec!["ie", "Ie", "Lib.Fm"]),
            (ROOT_CAUSE, vec!["cm", "Cm", "Lib.Fm"]),
            (TRANSFER_OUT_EVENT, vec!["out", "Out"]),
            (AND_GATE, vec!["AG", "Lib.Fm"]),
            (OR_GATE, vec!["OG", "Lib.Fm"]),
            (TRANSFER_IN_GATE, vec!["in", "Lib.Fm", "T1", "Lib.Fm"]),
        ] {
            let model = model_from_stmts(vec![mk_call(name, &args, 1)]);
            assert_eq!(model.nodes[0].kind.macro_name(), name);
        }
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
    fn missing_argument_is_an_error() {
        let file = ProcedureFile {
            stmts: vec![mk_call(FAILURE_MODE, &["only name"], 1)],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(err, FtaError::MissingArgs { line: 1, .. }));
    }

    #[test]
    fn transfer_in_gate_requires_four_arguments() {
        let file = ProcedureFile {
            stmts: vec![mk_call(TRANSFER_IN_GATE, &["name", "Lib.Fm", "T1"], 6)],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(
            err,
            FtaError::MissingArgs {
                line: 6,
                expected: 4,
                ..
            }
        ));
    }

    #[test]
    fn non_string_argument_is_an_error() {
        let file = ProcedureFile {
            stmts: vec![Statement::MacroCall(MacroCallDef {
                name: FAILURE_MODE.to_string(),
                args: vec![Arg::Number(1), Arg::String("Lib.Fm".to_string())],
                line: Some(3),
            })],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(
            err,
            FtaError::NonStringArg {
                line: 3,
                index: 0,
                ..
            }
        ));
    }

    #[test]
    fn argument_errors_name_the_line() {
        let file = ProcedureFile {
            stmts: vec![mk_call(FAILURE_MODE, &["only name"], 9)],
        };
        let message = FtaModel::from_procedure_file(&file)
            .unwrap_err()
            .to_string();
        assert!(message.contains("$FailureMode at line 9"), "{message}");
    }

    #[test]
    fn failure_mode_alias_equals_first_failure_mode() {
        let model = model_from_stmts(vec![mk_call(FAILURE_MODE, &["FM", "Lib.Fm"], 1)]);
        assert_eq!(model.nodes[0].alias, "Lib.Fm");
        assert_eq!(model.nodes[0].failure_modes(), ["Lib.Fm".to_string()]);
    }

    #[test]
    fn failure_mode_with_multiple_failure_modes_dedups_and_keeps_order() {
        let model = model_from_stmts(vec![mk_call(
            FAILURE_MODE,
            &["Top", "Lib.A", "Lib.B", "Lib.A"],
            1,
        )]);
        assert_eq!(model.nodes[0].alias, "Lib.A");
        assert_eq!(
            model.nodes[0].failure_modes(),
            ["Lib.A".to_string(), "Lib.B".to_string()]
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
    fn dotted_aliases_are_rejected_for_every_macro_with_an_alias() {
        for (name, args) in [
            (ROOT_CAUSE, vec!["cm", "Lib.Cm", "OG"]),
            (INTERMEDIATE_EVENT, vec!["ie", "Lib.Ie", "OG"]),
            (TRANSFER_OUT_EVENT, vec!["out", "Lib.Out"]),
            (AND_GATE, vec!["Lib.AG", "OG"]),
            (OR_GATE, vec!["Lib.OG", "OG"]),
            (TRANSFER_IN_GATE, vec!["in", "Lib.Fm", "Lib.T1", "OG"]),
        ] {
            let file = ProcedureFile {
                stmts: vec![mk_call(name, &args, 3)],
            };
            let err = FtaModel::from_procedure_file(&file).unwrap_err();
            assert!(
                matches!(&err, FtaError::InvalidAlias { macro_name, line: 3, .. } if macro_name == name),
                "{name}: {err:?}"
            );
        }
    }

    #[test]
    fn transfer_in_gate_target_forms_are_accepted() {
        for target in ["Alias", "Lib.Fm", "Lib.Fm.Alias"] {
            let model = model_from_stmts(vec![mk_call(
                TRANSFER_IN_GATE,
                &["name", target, "T1", "OG"],
                4,
            )]);
            assert_eq!(model.nodes[0].transfer_target(), Some(target));
            assert_eq!(model.nodes[0].alias, "T1");
            assert_eq!(model.nodes[0].name.as_deref(), Some("name"));
            assert_eq!(model.nodes[0].connections, vec!["OG"]);
        }
    }

    #[test]
    fn transfer_in_gate_target_with_bad_segments_is_rejected() {
        for target in ["", "A.B.C.D", "1Bad", "Lib..Fm", "Lib.Fm."] {
            let file = ProcedureFile {
                stmts: vec![mk_call(TRANSFER_IN_GATE, &["n", target, "T1", "OG"], 4)],
            };
            let err = FtaModel::from_procedure_file(&file).unwrap_err();
            assert!(
                matches!(err, FtaError::InvalidTransferTarget { line: 4, .. }),
                "{target:?}"
            );
        }
    }

    #[test]
    fn transfer_out_event_has_no_connection() {
        let model = model_from_stmts(vec![mk_call(TRANSFER_OUT_EVENT, &["Partial", "Out1"], 2)]);
        let node = &model.nodes[0];
        assert_eq!(node.kind, NodeKind::TransferOutEvent);
        assert_eq!(node.alias, "Out1");
        assert!(node.connections.is_empty());
        assert_eq!(node.transfer_target(), None);
    }

    #[test]
    fn repeated_alias_in_one_diagram_is_an_error() {
        let file = ProcedureFile {
            stmts: vec![
                mk_call(FAILURE_MODE, &["FM", "Lib.Fm"], 1),
                mk_call(OR_GATE, &["OG", "Lib.Fm"], 2),
                mk_call(ROOT_CAUSE, &["cm", "OG", "OG"], 3),
            ],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(
            &err,
            FtaError::DuplicateAlias { line: 3, first_line: 2, alias }
                if alias == "OG"
        ));
    }

    #[test]
    fn identical_repeated_node_gets_every_parent() {
        let model = model_from_stmts(vec![
            mk_call(FAILURE_MODE, &["FM", "Lib.Fm"], 1),
            mk_call(AND_GATE, &["AG1", "Lib.Fm"], 2),
            mk_call(AND_GATE, &["AG2", "Lib.Fm"], 3),
            mk_call(ROOT_CAUSE, &["shared", "Shared", "AG1"], 4),
            mk_call(ROOT_CAUSE, &["shared", "Shared", "AG2"], 5),
            mk_call(ROOT_CAUSE, &["shared", "Shared", "AG2"], 6),
        ]);
        let shared: Vec<_> = model.nodes.iter().filter(|n| n.alias == "Shared").collect();
        assert_eq!(shared.len(), 1);
        assert_eq!(shared[0].connections, vec!["AG1", "AG2"]);
        assert_eq!(shared[0].line, 4);
    }

    #[test]
    fn repeated_alias_with_other_text_is_an_error() {
        let file = ProcedureFile {
            stmts: vec![
                mk_call(ROOT_CAUSE, &["one", "Cm", "OG1"], 1),
                mk_call(ROOT_CAUSE, &["two", "Cm", "OG2"], 2),
            ],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(
            err,
            FtaError::DuplicateAlias {
                line: 2,
                first_line: 1,
                ..
            }
        ));
    }

    #[test]
    fn same_failure_mode_alias_twice_in_one_diagram_is_an_error() {
        let file = ProcedureFile {
            stmts: vec![
                mk_call(FAILURE_MODE, &["FM first", "Lib.Fm"], 1),
                mk_call(FAILURE_MODE, &["FM second", "Lib.Fm"], 2),
            ],
        };
        let err = FtaModel::from_procedure_file(&file).unwrap_err();
        assert!(matches!(err, FtaError::DuplicateAlias { line: 2, .. }));
    }

    #[test]
    fn root_failure_modes_follow_connections_and_skip_dangling_and_cyclic_chains() {
        let model = model_from_stmts(vec![
            mk_call(FAILURE_MODE, &["FM", "Lib.Fm"], 1),
            mk_call(OR_GATE, &["OG", "Lib.Fm"], 2),
            mk_call(INTERMEDIATE_EVENT, &["ie", "Ie", "OG"], 3),
            mk_call(ROOT_CAUSE, &["cm", "Cm", "Ie"], 4),
            mk_call(ROOT_CAUSE, &["lost", "Lost", "Missing"], 5),
            mk_call(OR_GATE, &["G1", "G2"], 6),
            mk_call(OR_GATE, &["G2", "G1"], 7),
        ]);
        let roots = model.root_failure_modes();
        assert_eq!(roots["Cm"][0].alias, "Lib.Fm");
        assert_eq!(roots["Ie"][0].alias, "Lib.Fm");
        assert_eq!(roots["Lib.Fm"][0].alias, "Lib.Fm");
        assert!(!roots.contains_key("Lost"));
        assert!(!roots.contains_key("G1"));
    }
}
