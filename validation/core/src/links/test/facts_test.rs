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
use class_diagram::{MemberVariable, Method, RelationType, Relationship, SimpleEntity};
use sequence_logic::{Block, ParticipantType, SequenceParticipant, SourceLocation};

fn ids(entries: &[FactEntry]) -> Vec<&str> {
    entries.iter().map(|e| e.id.as_str()).collect()
}

fn element(
    id: &str,
    alias: Option<&str>,
    name: Option<&str>,
    parent: Option<&str>,
) -> ComponentElement {
    ComponentElement {
        id: id.to_string(),
        name: name.map(str::to_string),
        alias: alias.map(str::to_string),
        parent_id: parent.map(str::to_string),
    }
}

fn entity(id: &str, name: &str) -> SimpleEntity {
    SimpleEntity {
        id: id.to_string(),
        name: name.to_string(),
        ..Default::default()
    }
}

fn with_members(entity: SimpleEntity) -> SimpleEntity {
    SimpleEntity {
        variables: vec![MemberVariable::default()],
        ..entity
    }
}

fn in_namespace(entity: SimpleEntity, namespace: &str) -> SimpleEntity {
    SimpleEntity {
        enclosing_namespace_id: Some(namespace.to_string()),
        ..entity
    }
}

fn related(entity: SimpleEntity, source: &str, target: &str) -> SimpleEntity {
    SimpleEntity {
        relationships: vec![Relationship {
            source: source.to_string(),
            target: target.to_string(),
            relation_type: RelationType::Association,
            ..Default::default()
        }],
        ..entity
    }
}

fn class_diagram(name: &str, entities: Vec<SimpleEntity>) -> ClassDiagram {
    ClassDiagram {
        source: Some("pkg/classes.puml".to_string()),
        name: name.to_string(),
        entities,
        free_functions: vec![],
    }
}

fn participant(uid: &str, alias: Option<&str>) -> SequenceParticipant {
    SequenceParticipant {
        display_name: uid.to_string(),
        alias: alias.map(str::to_string),
        uid: uid.to_string(),
        participant_type: ParticipantType::Participant,
        source_location: SourceLocation::new("pkg/seq.puml", 1),
        stereotype: None,
    }
}

#[test]
fn component_with_children_is_define_and_leaf_is_reference() {
    let facts = DiagramFacts::from_components(
        &[
            element("Proxy", Some("Proxy"), None, None),
            element("Proxy.Handler", Some("Handler"), None, Some("Proxy")),
        ],
        "pkg/proxy.puml",
        None,
    );

    assert_eq!(facts.source, "pkg/proxy.puml");
    assert_eq!(ids(&facts.defines), ["Proxy"]);
    assert_eq!(ids(&facts.references), ["Proxy.Handler"]);
}

#[test]
fn component_alias_falls_back_to_name_then_id() {
    let facts = DiagramFacts::from_components(
        &[
            element("id.only", None, None, None),
            element("id.named", None, Some("Named"), None),
            element("id.aliased", Some("Alias"), Some("Named"), None),
        ],
        "pkg/c.puml",
        None,
    );

    let aliases: Vec<&str> = facts.references.iter().map(|e| e.alias.as_str()).collect();
    assert_eq!(
        ids(&facts.references),
        ["id.aliased", "id.named", "id.only"]
    );
    assert_eq!(aliases, ["Alias", "Named", "id.only"]);
}

#[test]
fn component_named_by_the_diagram_is_define() {
    let facts = DiagramFacts::from_components(
        &[
            element("Proxy", Some("Proxy"), None, None),
            element("Other", Some("Other"), None, None),
        ],
        "pkg/proxy.puml",
        Some("Proxy"),
    );

    assert_eq!(ids(&facts.defines), ["Proxy"]);
    assert_eq!(ids(&facts.references), ["Other"]);
}

#[test]
fn class_with_members_is_define_and_without_is_reference() {
    let facts = DiagramFacts::from_class(
        &class_diagram(
            "d",
            vec![
                entity("pkg.Z", "Z"),
                with_members(entity("pkg.B", "B")),
                entity("pkg.A", "A"),
            ],
        ),
        "fallback.puml",
    );

    assert_eq!(facts.source, "pkg/classes.puml");
    assert_eq!(ids(&facts.defines), ["pkg.B"]);
    assert_eq!(ids(&facts.references), ["pkg.A", "pkg.Z"]);
}

#[test]
fn class_named_by_the_diagram_is_define_by_name_or_id_leaf() {
    let facts = DiagramFacts::from_class(
        &class_diagram(
            "Proxy",
            vec![
                entity("pkg.Proxy", "Display Label"),
                entity("pkg.Leaf", "Leaf"),
            ],
        ),
        "x.puml",
    );

    assert_eq!(ids(&facts.defines), ["pkg.Proxy"]);
    assert_eq!(ids(&facts.references), ["pkg.Leaf"]);
}

#[test]
fn class_relation_endpoints_are_references_unless_defined() {
    let facts = DiagramFacts::from_class(
        &class_diagram(
            "d",
            vec![
                related(with_members(entity("pkg.A", "A")), "pkg.A", "pkg.External"),
                related(with_members(entity("pkg.B", "B")), "pkg.B", "pkg.A"),
            ],
        ),
        "x.puml",
    );

    assert_eq!(ids(&facts.defines), ["pkg.A", "pkg.B"]);
    assert_eq!(ids(&facts.references), ["pkg.External"]);
    assert_eq!(facts.references[0].alias, "pkg.External");
}

#[test]
fn class_namespace_with_defined_child_is_namespace_define() {
    let facts = DiagramFacts::from_class(
        &class_diagram(
            "unit_1_class_diagram",
            vec![in_namespace(
                SimpleEntity {
                    methods: vec![Method::default()],
                    ..entity("seooc.comp.unit_1.Foo", "Foo")
                },
                "seooc.comp.unit_1",
            )],
        ),
        "x.puml",
    );

    let namespace = facts
        .defines
        .iter()
        .find(|e| e.id == "seooc.comp.unit_1")
        .expect("namespace define");
    assert_eq!(namespace.alias, "unit_1");
    assert_eq!(
        facts.namespace_defines,
        BTreeSet::from(["seooc.comp.unit_1".to_string()])
    );
}

#[test]
fn class_namespace_without_defined_child_is_no_define() {
    let facts = DiagramFacts::from_class(
        &class_diagram("d", vec![in_namespace(entity("pkg.Stub", "Stub"), "pkg")]),
        "x.puml",
    );

    assert!(facts.defines.is_empty());
    assert!(facts.namespace_defines.is_empty());
}

#[test]
fn class_namespace_endpoint_is_no_reference() {
    let facts = DiagramFacts::from_class(
        &class_diagram(
            "d",
            vec![related(
                in_namespace(with_members(entity("pkg.Ns.Child", "Child")), "pkg.Ns"),
                "pkg.Ns",
                "pkg.Ns.Child",
            )],
        ),
        "x.puml",
    );

    assert!(facts.references.is_empty());
    assert_eq!(ids(&facts.defines), ["pkg.Ns", "pkg.Ns.Child"]);
}

#[test]
fn class_namespace_define_does_not_shadow_an_entity() {
    let facts = DiagramFacts::from_class(
        &class_diagram(
            "d",
            vec![
                in_namespace(with_members(entity("pkg.Ns.Child", "Child")), "pkg.Ns"),
                with_members(entity("pkg.Ns", "Ns")),
            ],
        ),
        "x.puml",
    );

    assert_eq!(ids(&facts.defines), ["pkg.Ns", "pkg.Ns.Child"]);
    assert!(facts.namespace_defines.is_empty());
}

#[test]
fn sequence_participants_are_sorted_references_by_uid() {
    let tree = SequenceTree {
        source: None,
        name: None,
        participants: vec![
            participant("seooc.Zebra", Some("Z")),
            participant("Alpha", None),
        ],
        root: Block::default(),
    };

    let facts = DiagramFacts::from_sequence(&tree, "pkg/seq.puml");

    assert_eq!(facts.source, "pkg/seq.puml");
    assert!(facts.defines.is_empty());
    assert_eq!(ids(&facts.references), ["Alpha", "seooc.Zebra"]);
    assert_eq!(facts.references[1].alias, "Z");
}

fn facts_of(source: &str, id: &str) -> DiagramFacts {
    DiagramFacts {
        source: source.to_string(),
        references: vec![FactEntry {
            alias: id.to_string(),
            id: id.to_string(),
        }],
        ..Default::default()
    }
}

#[test]
fn identical_facts_of_a_source_are_kept_once() {
    let facts = vec![
        facts_of("a.puml", "x"),
        facts_of("b.puml", "y"),
        facts_of("a.puml", "x"),
    ];

    let unique = unique_by_source(facts).unwrap();

    let sources: Vec<&str> = unique.iter().map(|f| f.source.as_str()).collect();
    assert_eq!(sources, ["a.puml", "b.puml"]);
}

#[test]
fn differing_facts_of_a_source_are_an_error() {
    let facts = vec![facts_of("a.puml", "x"), facts_of("a.puml", "y")];

    let error = unique_by_source(facts).unwrap_err();

    assert!(error.contains("a.puml"), "{error}");
}
