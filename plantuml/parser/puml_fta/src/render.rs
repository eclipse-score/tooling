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

//! Emitters for the resolved fault trees: the `RootCause` TRLC stub and the
//! review index.

use std::collections::{BTreeMap, BTreeSet};

use puml_utils::{render_trlc_file, TrlcRecord, TrlcValue};
use serde::Serialize;

use crate::resolve::{Resolution, RootCauseStub, SourceRef};

/// Render [`RootCauseStub`]s as one TRLC file: a `RootCause` record per entry,
/// importing `ScoreReq` plus every package referenced by a `failure_modes`
/// fully-qualified name (no imports without records). Output order is
/// deterministic and every record carries a `<diagram>:<line>` comment pointing
/// back at its source macro call.
pub fn render_trlc_stub(package: &str, root_causes: &[RootCauseStub]) -> String {
    let records: Vec<TrlcRecord> = root_causes
        .iter()
        .map(|rc| TrlcRecord {
            record_type: "ScoreReq.RootCause".to_string(),
            name: rc.name.clone(),
            origin: format!("{}:{}", rc.diagram, rc.line),
            fields: vec![
                ("title", TrlcValue::Text(rc.title.clone())),
                ("diagram", TrlcValue::Text(rc.diagram.clone())),
                ("line", TrlcValue::Integer(rc.line as u64)),
                (
                    "failure_modes",
                    TrlcValue::List(
                        rc.failure_modes
                            .iter()
                            .map(|fm| TrlcValue::Reference(fm.clone()))
                            .collect(),
                    ),
                ),
            ],
        })
        .collect();
    render_trlc_file("FTA diagrams", package, &records)
}

/// Version of the JSON that [`render_index`] writes.
pub const INDEX_VERSION: u32 = 1;

#[derive(Serialize)]
struct IndexJson<'a> {
    version: u32,
    /// Diagram basename to the path of its source file.
    diagrams: &'a BTreeMap<String, String>,
    failure_mode_trees: BTreeMap<&'a str, Vec<SourceRefJson<'a>>>,
    via: BTreeMap<&'a str, BTreeMap<&'a str, Vec<SourceRefJson<'a>>>>,
    findings: Vec<FindingJson<'a>>,
}

#[derive(Serialize)]
struct SourceRefJson<'a> {
    diagram: &'a str,
    line: usize,
    alias: &'a str,
}

#[derive(Serialize)]
struct FindingJson<'a> {
    kind: &'static str,
    diagram: &'a str,
    line: usize,
    message: &'a str,
}

fn source_refs(refs: &BTreeSet<SourceRef>) -> Vec<SourceRefJson<'_>> {
    refs.iter()
        .map(|r| SourceRefJson {
            diagram: &r.diagram,
            line: r.line,
            alias: &r.alias,
        })
        .collect()
}

/// Render the review evidence of a [`Resolution`] as JSON: the `$FailureMode`
/// calls of each failure mode, the transfer gates each root cause reaches a
/// failure mode through, and the findings.  `diagrams` maps each diagram's
/// basename to the path of its source.  The safety-analysis page shows the
/// index and the findings are turned into traceability items from it.
pub fn render_index(resolution: &Resolution, diagrams: &BTreeMap<String, String>) -> String {
    let index = IndexJson {
        version: INDEX_VERSION,
        diagrams,
        failure_mode_trees: resolution
            .failure_mode_trees
            .iter()
            .map(|(fqn, nodes)| (fqn.as_str(), source_refs(nodes)))
            .collect(),
        via: resolution
            .root_causes
            .iter()
            .filter(|rc| !rc.via.is_empty())
            .map(|rc| {
                let per_failure_mode = rc
                    .via
                    .iter()
                    .map(|(fqn, gates)| (fqn.as_str(), source_refs(gates)))
                    .collect();
                (rc.name.as_str(), per_failure_mode)
            })
            .collect(),
        findings: resolution
            .findings
            .iter()
            .map(|f| FindingJson {
                kind: f.kind.as_str(),
                diagram: &f.diagram,
                line: f.line,
                message: &f.message,
            })
            .collect(),
    };
    let mut out = serde_json::to_string_pretty(&index).expect("index serialises");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::{FindingKind, FtaFinding};

    fn diagrams() -> BTreeMap<String, String> {
        [("b.puml".to_string(), "pkg/b.puml".to_string())].into()
    }

    #[test]
    fn render_index_lists_trees_gates_and_findings() {
        let gate = SourceRef {
            diagram: "a.puml".into(),
            line: 2,
            alias: "T1".into(),
        };
        let tree = SourceRef {
            diagram: "b.puml".into(),
            line: 1,
            alias: "fm".into(),
        };
        let mut resolution = Resolution::default();
        resolution.root_causes.push(RootCauseStub {
            name: "Cm".into(),
            title: "t".into(),
            diagram: "b.puml".into(),
            line: 3,
            failure_modes: vec!["Lib.A".into()],
            via: [("Lib.A".to_string(), [gate].into())].into(),
        });
        resolution.root_causes.push(RootCauseStub {
            name: "Direct".into(),
            title: "t".into(),
            diagram: "b.puml".into(),
            line: 4,
            failure_modes: vec!["Lib.A".into()],
            via: Default::default(),
        });
        resolution
            .failure_mode_trees
            .insert("Lib.A".into(), [tree].into());
        resolution.findings.push(FtaFinding {
            kind: FindingKind::UndevelopedEvent,
            diagram: "b.puml".into(),
            line: 5,
            message: "msg".into(),
        });
        let index: serde_json::Value =
            serde_json::from_str(&render_index(&resolution, &diagrams())).unwrap();
        assert_eq!(index["version"], 1);
        assert_eq!(index["diagrams"]["b.puml"], "pkg/b.puml");
        assert_eq!(index["failure_mode_trees"]["Lib.A"][0]["diagram"], "b.puml");
        assert_eq!(index["via"]["Cm"]["Lib.A"][0]["alias"], "T1");
        assert_eq!(index["via"]["Cm"]["Lib.A"][0]["line"], 2);
        assert!(index["via"].get("Direct").is_none());
        assert_eq!(index["findings"][0]["kind"], "UndevelopedEvent");
        assert_eq!(index["findings"][0]["message"], "msg");
    }

    #[test]
    fn render_index_of_an_empty_resolution_has_empty_sections() {
        let index: serde_json::Value =
            serde_json::from_str(&render_index(&Resolution::default(), &BTreeMap::new())).unwrap();
        assert_eq!(index["version"], 1);
        assert!(index["diagrams"].as_object().unwrap().is_empty());
        assert!(index["failure_mode_trees"].as_object().unwrap().is_empty());
        assert!(index["via"].as_object().unwrap().is_empty());
        assert!(index["findings"].as_array().unwrap().is_empty());
    }

    #[test]
    fn render_trlc_stub_emits_package_imports_and_records() {
        let root_causes = vec![RootCauseStub {
            name: "BeOne".into(),
            title: "root \"quoted\"".into(),
            diagram: "d.puml".into(),
            line: 8,
            failure_modes: vec!["Lib.A".into(), "Other.B".into()],
            via: Default::default(),
        }];
        let out = render_trlc_stub("MyFta", &root_causes);
        assert!(out.contains("package MyFta"));
        assert!(out.contains("import ScoreReq"));
        assert!(out.contains("import Lib"));
        assert!(out.contains("import Other"));
        assert!(out.contains("ScoreReq.RootCause BeOne {"));
        assert!(out.contains("failure_modes = [Lib.A, Other.B]"));
        assert!(out.contains("root \\\"quoted\\\""));
        assert!(out.contains("// d.puml:8"));
    }

    #[test]
    fn render_trlc_stub_without_root_causes_has_no_imports() {
        let out = render_trlc_stub("MyFta", &[]);
        assert!(out.contains("package MyFta"));
        assert!(!out.contains("import"));
    }

    #[test]
    fn render_trlc_stub_renders_a_root_cause_without_failure_modes() {
        let root_causes = vec![RootCauseStub {
            name: "Lonely".into(),
            title: "t".into(),
            diagram: "d.puml".into(),
            line: 2,
            failure_modes: Vec::new(),
            via: Default::default(),
        }];
        let out = render_trlc_stub("MyFta", &root_causes);
        assert!(out.contains("failure_modes = []"), "{out}");
    }
}
