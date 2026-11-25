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

use super::*;
use crate::links::facts::FactEntry;

fn entries(pairs: &[(&str, &str)]) -> Vec<FactEntry> {
    pairs
        .iter()
        .map(|(alias, id)| FactEntry {
            alias: alias.to_string(),
            id: id.to_string(),
        })
        .collect()
}

fn diagram(source: &str, defines: &[(&str, &str)], references: &[(&str, &str)]) -> DiagramFacts {
    DiagramFacts {
        source: source.to_string(),
        defines: entries(defines),
        references: entries(references),
        namespace_defines: Default::default(),
    }
}

fn namespace_wrapper(source: &str, defines: &[(&str, &str)], namespace: &str) -> DiagramFacts {
    DiagramFacts {
        namespace_defines: [namespace.to_string()].into(),
        ..diagram(source, defines, &[])
    }
}

fn definer(diagrams: &[DiagramFacts], id: &str, source: &str) -> Option<String> {
    DefinerIndex::new(diagrams)
        .definer(id, source)
        .map(str::to_string)
}

#[test]
fn exact_id_wins_over_leaf() {
    let diagrams = [
        diagram("pkg/id_hit.puml", &[("Proxy", "pkg.Proxy")], &[]),
        diagram("pkg/leaf_hit.puml", &[("Proxy", "other.Proxy")], &[]),
    ];

    assert_eq!(
        definer(&diagrams, "pkg.Proxy", "pkg/src.puml").as_deref(),
        Some("pkg/id_hit.puml")
    );
}

#[test]
fn leaf_is_used_without_an_id_definer() {
    let diagrams = [diagram("pkg/leaf_hit.puml", &[("Proxy", "pkg.Proxy")], &[])];

    assert_eq!(
        definer(&diagrams, "Proxy", "pkg/src.puml").as_deref(),
        Some("pkg/leaf_hit.puml")
    );
    assert_eq!(
        definer(&diagrams, "other.Proxy", "pkg/src.puml").as_deref(),
        Some("pkg/leaf_hit.puml")
    );
}

#[test]
fn leaf_is_used_when_only_the_diagram_itself_defines_the_id() {
    let diagrams = [
        diagram("pkg/src.puml", &[("Proxy", "pkg.Proxy")], &[]),
        diagram("pkg/leaf_hit.puml", &[("Proxy", "x.Proxy")], &[]),
    ];

    assert_eq!(
        definer(&diagrams, "pkg.Proxy", "pkg/src.puml").as_deref(),
        Some("pkg/leaf_hit.puml")
    );
}

#[test]
fn no_self_link() {
    let diagrams = [diagram("pkg/src.puml", &[("Proxy", "Proxy")], &[])];

    assert_eq!(definer(&diagrams, "Proxy", "pkg/src.puml"), None);
}

#[test]
fn closest_path_wins() {
    let diagrams = [
        diagram("a/one.puml", &[("Proxy", "Proxy")], &[]),
        diagram("pkg/two.puml", &[("Proxy", "Proxy")], &[]),
    ];

    assert_eq!(
        definer(&diagrams, "Proxy", "pkg/src.puml").as_deref(),
        Some("pkg/two.puml")
    );
}

#[test]
fn tie_gives_no_link() {
    let diagrams = [
        diagram("a/one.puml", &[("Proxy", "Proxy")], &[]),
        diagram("b/two.puml", &[("Proxy", "Proxy")], &[]),
    ];

    assert_eq!(definer(&diagrams, "Proxy", "pkg/src.puml"), None);
}

#[test]
fn same_directory_tie_goes_to_most_descendants() {
    let diagrams = [
        diagram(
            "design/static_design.puml",
            &[("component_example", "pkg.component_example")],
            &[
                ("unit_1", "pkg.component_example.unit_1"),
                ("unit_2", "pkg.component_example.unit_2"),
                ("Internal", "pkg.component_example.Internal"),
            ],
        ),
        diagram(
            "design/internal_api.puml",
            &[
                ("component_example", "pkg.component_example"),
                ("Internal", "pkg.component_example.Internal"),
            ],
            &[],
        ),
    ];

    assert_eq!(
        definer(&diagrams, "pkg.component_example", "design/overview.puml").as_deref(),
        Some("design/static_design.puml")
    );
}

#[test]
fn equal_descendant_counts_give_no_link() {
    let diagrams = [
        diagram("design/one.puml", &[("Proxy", "pkg.Proxy")], &[]),
        diagram("design/two.puml", &[("Proxy", "pkg.Proxy")], &[]),
    ];

    assert_eq!(
        definer(&diagrams, "pkg.Proxy", "design/overview.puml"),
        None
    );
}

#[test]
fn drawn_element_wins_over_namespace_wrapper() {
    let diagrams = [
        diagram(
            "design/overview.puml",
            &[("Seooc", "pkg.Seooc")],
            &[("CE", "pkg.Seooc.CE")],
        ),
        namespace_wrapper(
            "design/public_api.puml",
            &[("Seooc", "pkg.Seooc"), ("Api", "pkg.Seooc.Api")],
            "pkg.Seooc",
        ),
    ];

    assert_eq!(
        definer(&diagrams, "pkg.Seooc", "design/seq.puml").as_deref(),
        Some("design/overview.puml")
    );
    assert_eq!(
        definer(&diagrams, "Seooc", "design/seq.puml").as_deref(),
        Some("design/overview.puml")
    );
}

#[test]
fn namespace_wrappers_only_still_tie() {
    let diagrams = [
        namespace_wrapper("design/one.puml", &[("Ns", "pkg.Ns")], "pkg.Ns"),
        namespace_wrapper("design/two.puml", &[("Ns", "pkg.Ns")], "pkg.Ns"),
    ];

    assert_eq!(definer(&diagrams, "pkg.Ns", "design/seq.puml"), None);
}

#[test]
fn element_links_take_the_first_linkable_reference_per_alias() {
    let diagrams = [
        diagram(
            "d/overview.puml",
            &[],
            &[
                ("unit", "pkg.unknown"),
                ("unit", "pkg.unit"),
                ("", "pkg.unit"),
            ],
        ),
        diagram("d/unit.puml", &[("unit", "pkg.unit")], &[]),
    ];

    assert_eq!(
        element_links(&diagrams),
        [ElementLink {
            source: "d/overview.puml".to_string(),
            alias: "unit".to_string(),
            target: "d/unit.puml".to_string(),
        }]
    );
}
