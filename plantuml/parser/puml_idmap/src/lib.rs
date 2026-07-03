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

//! Converts the resolved PlantUML logical model into an `.idmap.json` file
//! consumed by the `clickable_plantuml` Sphinx extension.
//!
//! The idmap separates each diagram's elements into two roles:
//!
//! * **defines** – elements that are *elaborated* in this diagram (they have
//!   child elements, class members, or this diagram is the detail view).
//! * **references** – leaf mentions and relation endpoints (elements that
//!   should link *away* to wherever they are elaborated).
//!
//! This mirrors the structure of `puml_lobster` but produces idmap JSON
//! rather than LOBSTER trace JSON.

use class_diagram::ClassDiagram;
use component_diagram::{ComponentType, LogicComponent};
use puml_fta::{FtaModel, NodeKind};
use sequence_logic::SequenceTree;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// A single element entry in the idmap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdMapEntry {
    /// PlantUML alias used in `url of <alias> is [[url]]` injection.
    pub alias: String,
    /// Fully-qualified identifier (FQN) for matching across diagrams.
    pub id: String,
}

/// Root structure of an `.idmap.json` file.
#[derive(Debug, Serialize, Deserialize)]
pub struct IdMapFile {
    /// Workspace-relative source path, e.g. `score/mw/com/proxy_detail.puml`.
    pub source: String,
    /// Elements elaborated (defined) in this diagram.
    pub defines: Vec<IdMapEntry>,
    /// Elements referenced (leaf/relation endpoint) in this diagram.
    pub references: Vec<IdMapEntry>,
}

// ---------------------------------------------------------------------------
// Model wrapper
// ---------------------------------------------------------------------------

/// Union of the resolved diagram models accepted by the idmap writer.
pub enum IdMapModel<'a> {
    Component(&'a HashMap<String, LogicComponent>),
    Class(&'a ClassDiagram),
    Sequence(&'a SequenceTree),
    Fta(&'a FtaModel),
}

// ---------------------------------------------------------------------------
// Model converters
// ---------------------------------------------------------------------------

/// Produce an [`IdMapFile`] from a resolved component diagram.
///
/// An element is a **define** when at least one other element lists it as its
/// `parent_id` (i.e. it has children and is therefore elaborated here).
/// All remaining non-package elements are **references** (top-level leaves
/// that mention something that may be detailed in another diagram).
/// Packages are treated as containers/namespaces and are not emitted.
fn comp_model_to_idmap(
    model: &HashMap<String, LogicComponent>,
    source: &str,
    diagram_name: Option<&str>,
) -> IdMapFile {
    // Collect the set of IDs that are listed as parent by at least one child.
    let has_children: HashSet<&str> = model
        .values()
        .filter_map(|c| c.parent_id.as_deref())
        .collect();

    let mut defines = Vec::new();
    let mut references = Vec::new();

    for comp in model.values() {
        if comp.element_type == ComponentType::Package {
            continue;
        }

        let alias = comp
            .alias
            .as_deref()
            .or(comp.name.as_deref())
            .unwrap_or(&comp.id)
            .to_string();
        // An element is a define when it has children OR when the diagram's
        // @startuml <name> matches its alias/name (the diagram elaborates it).
        let matches_diagram_name = diagram_name
            .map(|dn| comp.alias.as_deref() == Some(dn) || comp.name.as_deref() == Some(dn))
            .unwrap_or(false);
        let is_define = has_children.contains(comp.id.as_str()) || matches_diagram_name;
        let entry = IdMapEntry {
            alias,
            id: comp.id.clone(),
        };
        if is_define {
            defines.push(entry);
        } else {
            references.push(entry);
        }
    }

    // NOTE: `LogicComponent.relations` endpoints are intentionally *not* scanned
    // here (unlike class-diagram relationships). Every component relation
    // endpoint is itself a declared component in this same `model` map, so it is
    // already emitted above as either a define or a reference. Scanning
    // relations would only re-derive those identical ids and produce duplicates.
    defines.sort_by(|a, b| a.id.cmp(&b.id));
    references.sort_by(|a, b| a.id.cmp(&b.id));

    IdMapFile {
        source: source.to_string(),
        defines,
        references,
    }
}

/// Produce an [`IdMapFile`] from a resolved class diagram.
///
/// A class entity is a **define** when it has any members (methods or
/// variables), making this diagram the elaboration site.  Entities without
/// members are **references**. Relation endpoints are also emitted as
/// references so links can resolve from relationship mentions.
fn class_model_to_idmap(model: &ClassDiagram, source: &str) -> IdMapFile {
    // The @startuml <name> value is preserved in ClassDiagram::name by the resolver.
    let diagram_name = if model.name.is_empty() {
        None
    } else {
        Some(model.name.as_str())
    };

    let mut defines = Vec::new();
    let mut references = Vec::new();
    let mut define_ids: HashSet<String> = HashSet::new();
    let mut reference_ids: HashSet<String> = HashSet::new();
    let entity_name_by_id: HashMap<&str, &str> = model
        .entities
        .iter()
        .map(|e| (e.id.as_str(), e.name.as_str()))
        .collect();

    // Namespaces/packages are pure containers, not linkable elements — mirror
    // the component-diagram handling where `ComponentType::Package` is skipped.
    // The resolver never materialises a namespace as an entity (only its FQN
    // survives on each child's `enclosing_namespace_id`), so no namespace can
    // reach the entity loop below as a define. A relationship may still point at
    // a container FQN, though, so collect them here to filter such endpoints.
    let namespace_ids: HashSet<&str> = model
        .entities
        .iter()
        .filter_map(|e| e.enclosing_namespace_id.as_deref())
        .collect();

    for entity in &model.entities {
        let has_members = !entity.methods.is_empty() || !entity.variables.is_empty();
        let matches_diagram_name = diagram_name == Some(entity.name.as_str());
        let is_define = has_members || matches_diagram_name;
        if is_define {
            define_ids.insert(entity.id.clone());
            defines.push(IdMapEntry {
                alias: entity.name.clone(),
                id: entity.id.clone(),
            });
            continue;
        }

        if reference_ids.insert(entity.id.clone()) {
            references.push(IdMapEntry {
                alias: entity.name.clone(),
                id: entity.id.clone(),
            });
        }
    }

    for relationship in &model.relationships {
        for endpoint in [&relationship.source, &relationship.target] {
            // A relation endpoint that is only a namespace/package container is
            // not a linkable element; never emit it as a reference.
            if namespace_ids.contains(endpoint.as_str()) {
                continue;
            }
            if define_ids.contains(endpoint) || !reference_ids.insert(endpoint.clone()) {
                continue;
            }

            let alias = entity_name_by_id
                .get(endpoint.as_str())
                .map(|name| (*name).to_string())
                .unwrap_or_else(|| endpoint.clone());

            references.push(IdMapEntry {
                alias,
                id: endpoint.clone(),
            });
        }
    }

    defines.sort_by(|a, b| a.id.cmp(&b.id));
    references.sort_by(|a, b| a.id.cmp(&b.id));

    IdMapFile {
        source: source.to_string(),
        defines,
        references,
    }
}

/// Collect the unique participant names from a sequence tree.
fn collect_participants(tree: &SequenceTree) -> HashSet<String> {
    use sequence_logic::{Event, SequenceNode};

    fn walk_nodes(nodes: &[SequenceNode], out: &mut HashSet<String>) {
        for node in nodes {
            match &node.event {
                Event::Interaction(i) => {
                    out.insert(i.caller.clone());
                    out.insert(i.callee.clone());
                }
                Event::Return(r) => {
                    out.insert(r.caller.clone());
                    out.insert(r.callee.clone());
                }
                Event::Condition(_) => {}
            }
            walk_nodes(&node.branches_node, out);
        }
    }

    // TODO: switch to explicit participant declarations once the sequence
    // resolver persists them in SequenceTree.
    let mut participants = HashSet::new();
    walk_nodes(&tree.root_interactions, &mut participants);
    participants
}

/// Produce an [`IdMapFile`] from a resolved sequence diagram.
///
/// Sequence diagrams have no "definition" elements — all participants are
/// references (each participant links away to the component diagram that
/// elaborates it).
fn sequence_model_to_idmap(model: &SequenceTree, source: &str) -> IdMapFile {
    let participants = collect_participants(model);
    let mut references: Vec<IdMapEntry> = participants
        .into_iter()
        .map(|name| IdMapEntry {
            alias: name.clone(),
            id: name,
        })
        .collect();
    references.sort_by(|a, b| a.id.cmp(&b.id));

    IdMapFile {
        source: source.to_string(),
        defines: Vec::new(),
        references,
    }
}

/// Produce an empty [`IdMapFile`] for diagrams without cross-linkable elements.
fn empty_idmap(source: &str) -> IdMapFile {
    IdMapFile {
        source: source.to_string(),
        defines: Vec::new(),
        references: Vec::new(),
    }
}

/// Return `true` when `alias` is a 2-part TRLC fully-qualified name of the form
/// `Package.Record`, where each part is a valid identifier (leading ASCII
/// letter or `_`, followed by ASCII alphanumerics or `_`).
fn is_trlc_fqn(alias: &str) -> bool {
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

/// Produce an [`IdMapFile`] from a resolved FTA model.
///
/// A node is a **define** when it is the tree's top event (`NodeKind::TopEvent`,
/// `connection` is `None` — a relation sink never used as a source).
/// Gate nodes whose alias is a valid TRLC fully-qualified name (`Package.Record`)
/// are `$TransferInGate` references pointing to another diagram's top event;
/// these are emitted as **references**.  All other nodes (basic/intermediate
/// events, `$AndGate`, `$OrGate`) are internal and produce no cross-diagram link.
fn fta_model_to_idmap(model: &FtaModel, source: &str) -> IdMapFile {
    let mut defines = Vec::new();
    let mut references = Vec::new();

    for node in &model.nodes {
        match node.kind {
            NodeKind::TopEvent => {
                defines.push(IdMapEntry {
                    alias: node.alias.clone(),
                    id: node.alias.clone(),
                });
            }
            NodeKind::Gate if is_trlc_fqn(&node.alias) => {
                // $TransferInGate: alias is the foreign top-event's TRLC FQN.
                references.push(IdMapEntry {
                    alias: node.alias.clone(),
                    id: node.alias.clone(),
                });
            }
            _ => {} // BasicEvent, IntermediateEvent, And/OrGate — internal, no link.
        }
    }

    defines.sort_by(|a, b| a.id.cmp(&b.id));
    references.sort_by(|a, b| a.id.cmp(&b.id));

    IdMapFile {
        source: source.to_string(),
        defines,
        references,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Write an `.idmap.json` file for *model* into *output_dir*.
///
/// The output filename is `<stem>.idmap.json` where `<stem>` is the file
/// stem of *input_path* (the original `.puml` source file).
///
/// The `source` field embedded in the JSON is set to *source_name* when
/// provided (preferred: a stable workspace-relative path such as
/// `score/mw/com/proxy_detail.puml`), otherwise falls back to
/// `input_path.to_string_lossy()`.
pub fn write_idmap_to_file(
    model: IdMapModel<'_>,
    input_path: &Path,
    source_name: Option<&str>,
    diagram_name: Option<&str>,
    output_dir: &Path,
) -> io::Result<PathBuf> {
    let source = source_name
        .map(|s| s.to_string())
        .unwrap_or_else(|| input_path.to_string_lossy().into_owned());

    let idmap = match model {
        IdMapModel::Component(m) => comp_model_to_idmap(m, &source, diagram_name),
        IdMapModel::Class(m) => class_model_to_idmap(m, &source),
        IdMapModel::Sequence(m) => sequence_model_to_idmap(m, &source),
        IdMapModel::Fta(m) => fta_model_to_idmap(m, &source),
    };

    write_idmap_json(input_path, output_dir, &idmap)
}

/// Write an empty `.idmap.json` for diagrams that intentionally have no
/// cross-linkable elements (for example, activity diagrams).
pub fn write_empty_idmap_to_file(
    input_path: &Path,
    source_name: Option<&str>,
    output_dir: &Path,
) -> io::Result<PathBuf> {
    let source = source_name
        .map(|s| s.to_string())
        .unwrap_or_else(|| input_path.to_string_lossy().into_owned());
    let idmap = empty_idmap(&source);

    write_idmap_json(input_path, output_dir, &idmap)
}

fn write_idmap_json(
    input_path: &Path,
    output_dir: &Path,
    idmap: &IdMapFile,
) -> io::Result<PathBuf> {
    let file_stem = input_path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("output");
    let output_path = output_dir.join(format!("{file_stem}.idmap.json"));

    let json = serde_json::to_string_pretty(idmap).map_err(io::Error::other)?;
    fs::write(&output_path, json)?;

    log::debug!("idmap written to {}", output_path.display());
    Ok(output_path)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use class_diagram::{MemberVariable, RelationType, Relationship, SimpleEntity};
    use component_diagram::{ComponentType, SourceLocation};
    use puml_fta::FtaNode;
    use sequence_logic::{Event, Interaction, SequenceNode};

    fn component(
        id: &str,
        alias: Option<&str>,
        name: Option<&str>,
        parent: Option<&str>,
    ) -> LogicComponent {
        LogicComponent {
            id: id.to_string(),
            name: name.map(str::to_string),
            alias: alias.map(str::to_string),
            parent_id: parent.map(str::to_string),
            element_type: ComponentType::Component,
            stereotype: None,
            relations: Vec::new(),
            source_location: SourceLocation::new("test.puml", 0),
        }
    }

    fn component_map(components: Vec<LogicComponent>) -> HashMap<String, LogicComponent> {
        components.into_iter().map(|c| (c.id.clone(), c)).collect()
    }

    #[test]
    fn component_children_make_define_leaves_make_reference() {
        // `Proxy` has a child `Handler` → Proxy is a define, Handler a reference.
        let model = component_map(vec![
            component("Proxy", Some("Proxy"), None, None),
            component("Handler", Some("Handler"), None, Some("Proxy")),
        ]);

        let idmap = comp_model_to_idmap(&model, "pkg/proxy.puml", None);

        assert_eq!(idmap.source, "pkg/proxy.puml");
        assert_eq!(
            idmap
                .defines
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["Proxy"]
        );
        assert_eq!(
            idmap
                .references
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["Handler"]
        );
    }

    #[test]
    fn component_with_no_children_is_all_references() {
        let model = component_map(vec![
            component("A", Some("A"), None, None),
            component("B", Some("B"), None, None),
        ]);

        let idmap = comp_model_to_idmap(&model, "pkg/overview.puml", None);

        assert!(idmap.defines.is_empty());
        assert_eq!(idmap.references.len(), 2);
    }

    #[test]
    fn component_alias_falls_back_to_name_then_id() {
        let model = component_map(vec![
            component("id.only", None, None, None),
            component("id.named", None, Some("DisplayName"), None),
            component("id.aliased", Some("AliasName"), Some("DisplayName"), None),
        ]);

        let idmap = comp_model_to_idmap(&model, "pkg/aliases.puml", None);

        let alias_of = |id: &str| -> String {
            idmap
                .references
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.alias.clone())
                .unwrap()
        };
        assert_eq!(alias_of("id.only"), "id.only");
        assert_eq!(alias_of("id.named"), "DisplayName");
        assert_eq!(alias_of("id.aliased"), "AliasName");
    }

    #[test]
    fn component_output_is_sorted_by_id() {
        let model = component_map(vec![
            component("zeta", Some("zeta"), None, None),
            component("alpha", Some("alpha"), None, None),
            component("mu", Some("mu"), None, None),
        ]);

        let idmap = comp_model_to_idmap(&model, "pkg/sorted.puml", None);

        let ids: Vec<&str> = idmap.references.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["alpha", "mu", "zeta"]);
    }

    #[test]
    fn class_entities_with_members_are_defines() {
        let with_members = SimpleEntity {
            id: "pkg.WithMembers".to_string(),
            name: "WithMembers".to_string(),
            variables: vec![MemberVariable::default()],
            ..Default::default()
        };
        let without_members = SimpleEntity {
            id: "pkg.Empty".to_string(),
            name: "Empty".to_string(),
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![with_members, without_members],
            relationships: Vec::new(),
            source_files: Vec::new(),
            version: None,
        };

        let idmap = class_model_to_idmap(&model, "pkg/classes.puml");

        assert_eq!(
            idmap
                .defines
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["pkg.WithMembers"]
        );
        assert_eq!(
            idmap
                .references
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["pkg.Empty"]
        );
    }

    #[test]
    fn class_output_is_sorted_by_id_for_defines_and_references() {
        let with_members_z = SimpleEntity {
            id: "pkg.Z".to_string(),
            name: "Z".to_string(),
            variables: vec![MemberVariable::default()],
            ..Default::default()
        };
        let with_members_a = SimpleEntity {
            id: "pkg.A".to_string(),
            name: "A".to_string(),
            variables: vec![MemberVariable::default()],
            ..Default::default()
        };
        let ref_m = SimpleEntity {
            id: "pkg.M".to_string(),
            name: "M".to_string(),
            ..Default::default()
        };
        let ref_b = SimpleEntity {
            id: "pkg.B".to_string(),
            name: "B".to_string(),
            ..Default::default()
        };

        let model = ClassDiagram {
            name: "sorted".to_string(),
            entities: vec![with_members_z, ref_m, with_members_a, ref_b],
            relationships: Vec::new(),
            source_files: Vec::new(),
            version: None,
        };

        let idmap = class_model_to_idmap(&model, "pkg/class_sorted.puml");

        let define_ids: Vec<&str> = idmap.defines.iter().map(|e| e.id.as_str()).collect();
        let ref_ids: Vec<&str> = idmap.references.iter().map(|e| e.id.as_str()).collect();

        assert_eq!(define_ids, ["pkg.A", "pkg.Z"]);
        assert_eq!(ref_ids, ["pkg.B", "pkg.M"]);
    }

    #[test]
    fn sequence_participants_become_sorted_references() {
        let interaction = |caller: &str, callee: &str| SequenceNode {
            event: Event::Interaction(Interaction {
                caller: caller.to_string(),
                callee: callee.to_string(),
                method: "call".to_string(),
            }),
            source_location: SourceLocation::new("test.puml", 0),
            branches_node: Vec::new(),
        };
        let tree = SequenceTree {
            name: None,
            root_interactions: vec![interaction("Zebra", "Alpha"), interaction("Alpha", "Mango")],
        };

        let idmap = sequence_model_to_idmap(&tree, "pkg/seq.puml");

        assert!(idmap.defines.is_empty());
        let ids: Vec<&str> = idmap.references.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["Alpha", "Mango", "Zebra"]);
    }

    #[test]
    fn empty_model_yields_empty_idmap() {
        let idmap = empty_idmap("pkg/activity.puml");

        assert_eq!(idmap.source, "pkg/activity.puml");
        assert!(idmap.defines.is_empty());
        assert!(idmap.references.is_empty());
    }

    #[test]
    fn overview_top_level_leaves_are_references_not_defines() {
        // [Gateway] --> [Proxy] — no children on either
        let model = component_map(vec![
            component("Gateway", Some("Gateway"), None, None),
            component("Proxy", Some("Proxy"), None, None),
        ]);
        let idmap = comp_model_to_idmap(&model, "overview.puml", None);
        assert!(idmap.defines.is_empty());
        assert_eq!(idmap.references.len(), 2);
    }

    #[test]
    fn detail_diagram_name_promotes_to_define() {
        // @startuml Proxy — diagram_name matches element alias
        let model = component_map(vec![
            component("Proxy", Some("Proxy"), None, None),
            component("Proxy.RequestHandler", Some("RequestHandler"), None, None),
        ]);
        let idmap = comp_model_to_idmap(&model, "proxy_detail.puml", Some("Proxy"));
        assert!(idmap.defines.iter().any(|e| e.alias == "Proxy"));
    }

    #[test]
    fn packages_are_not_emitted_as_defines_or_references() {
        let mut pkg = component("Pkg", Some("Pkg"), None, None);
        pkg.element_type = ComponentType::Package;
        let model = component_map(vec![
            pkg,
            component("A", Some("A"), None, Some("Pkg")),
            component("AA", Some("AA"), None, Some("A")),
        ]);

        let idmap = comp_model_to_idmap(&model, "pkg.puml", None);
        assert!(!idmap.defines.iter().any(|e| e.id == "Pkg"));
        assert!(!idmap.references.iter().any(|e| e.id == "Pkg"));
    }

    #[test]
    fn class_relationship_endpoints_are_emitted_as_references() {
        let define = SimpleEntity {
            id: "pkg.Define".to_string(),
            name: "Define".to_string(),
            variables: vec![MemberVariable::default()],
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![define],
            relationships: vec![Relationship {
                source: "pkg.Define".to_string(),
                target: "pkg.ExternalRef".to_string(),
                relation_type: RelationType::Association,
                source_multiplicity: None,
                target_multiplicity: None,
            }],
            source_files: Vec::new(),
            version: None,
        };

        let idmap = class_model_to_idmap(&model, "pkg/classes.puml");
        assert!(idmap.references.iter().any(|e| e.id == "pkg.ExternalRef"));
    }

    #[test]
    fn class_relationship_endpoint_that_is_a_define_is_not_duplicated_as_reference() {
        // A --> B where BOTH endpoints have members (are defines). The
        // `define_ids.contains(endpoint)` guard must keep them out of references.
        let a = SimpleEntity {
            id: "pkg.A".to_string(),
            name: "A".to_string(),
            variables: vec![MemberVariable::default()],
            ..Default::default()
        };
        let b = SimpleEntity {
            id: "pkg.B".to_string(),
            name: "B".to_string(),
            variables: vec![MemberVariable::default()],
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![a, b],
            relationships: vec![Relationship {
                source: "pkg.A".to_string(),
                target: "pkg.B".to_string(),
                relation_type: RelationType::Association,
                source_multiplicity: None,
                target_multiplicity: None,
            }],
            source_files: Vec::new(),
            version: None,
        };

        let idmap = class_model_to_idmap(&model, "pkg/classes.puml");

        let define_ids: Vec<&str> = idmap.defines.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(define_ids, ["pkg.A", "pkg.B"]);
        assert!(
            idmap.references.is_empty(),
            "endpoints already present as defines must not be duplicated into references, got: {:?}",
            idmap.references
        );
    }

    #[test]
    fn class_namespace_container_endpoint_is_not_emitted_as_reference() {
        // A relation points at the namespace/package container FQN. Containers
        // are not linkable elements, so the endpoint must be dropped.
        let child = SimpleEntity {
            id: "pkg.Container.Child".to_string(),
            name: "Child".to_string(),
            enclosing_namespace_id: Some("pkg.Container".to_string()),
            variables: vec![MemberVariable::default()],
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![child],
            relationships: vec![Relationship {
                source: "pkg.Container.Child".to_string(),
                target: "pkg.Container".to_string(),
                relation_type: RelationType::Association,
                source_multiplicity: None,
                target_multiplicity: None,
            }],
            source_files: Vec::new(),
            version: None,
        };

        let idmap = class_model_to_idmap(&model, "pkg/ns.puml");
        assert!(
            !idmap.references.iter().any(|e| e.id == "pkg.Container"),
            "namespace container must not leak as a reference, got: {:?}",
            idmap.references
        );
        assert!(!idmap.defines.iter().any(|e| e.id == "pkg.Container"));
    }

    #[test]
    fn class_matches_diagram_name_promotes_memberless_entity_to_define() {
        // @startuml Proxy — an entity without members whose name equals the
        // diagram name is still elaborated here, so it is a define.
        let proxy = SimpleEntity {
            id: "pkg.Proxy".to_string(),
            name: "Proxy".to_string(),
            ..Default::default()
        };
        let leaf = SimpleEntity {
            id: "pkg.Leaf".to_string(),
            name: "Leaf".to_string(),
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "Proxy".to_string(),
            entities: vec![proxy, leaf],
            relationships: Vec::new(),
            source_files: Vec::new(),
            version: None,
        };

        let idmap = class_model_to_idmap(&model, "pkg/proxy.puml");
        assert!(idmap.defines.iter().any(|e| e.id == "pkg.Proxy"));
        assert!(idmap.references.iter().any(|e| e.id == "pkg.Leaf"));
        assert!(!idmap.defines.iter().any(|e| e.id == "pkg.Leaf"));
    }

    // ── FTA converter ──────────────────────────────────────────────────────

    fn fta_node(kind: NodeKind, alias: &str) -> FtaNode {
        FtaNode {
            kind,
            name: None,
            alias: alias.to_string(),
            connection: None,
            line: None,
        }
    }

    #[test]
    fn fta_top_event_is_define_and_transfer_gate_is_reference() {
        let model = FtaModel {
            nodes: vec![
                fta_node(NodeKind::TopEvent, "pkg.TopFailure"),
                // $TransferInGate: alias is a foreign top-event FQN → reference.
                fta_node(NodeKind::Gate, "other.ForeignTop"),
                // $AndGate/$OrGate: alias is not an FQN → internal, no link.
                fta_node(NodeKind::Gate, "OG"),
                // Basic events are internal even when the alias looks like an FQN.
                fta_node(NodeKind::BasicEvent, "pkg.Cause"),
                fta_node(NodeKind::IntermediateEvent, "IE"),
            ],
        };

        let idmap = fta_model_to_idmap(&model, "pkg/fta.puml");

        let define_ids: Vec<&str> = idmap.defines.iter().map(|e| e.id.as_str()).collect();
        let reference_ids: Vec<&str> = idmap.references.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(define_ids, ["pkg.TopFailure"]);
        assert_eq!(reference_ids, ["other.ForeignTop"]);
    }

    #[test]
    fn is_trlc_fqn_accepts_two_part_identifiers_and_rejects_others() {
        // Valid: exactly two identifier parts.
        assert!(is_trlc_fqn("Package.Record"));
        assert!(is_trlc_fqn("_priv.R1"));
        assert!(is_trlc_fqn("a1.b2"));

        // Invalid: wrong number of parts.
        assert!(!is_trlc_fqn("NoDot"));
        assert!(!is_trlc_fqn("a.b.c"));
        assert!(!is_trlc_fqn(""));

        // Invalid: empty parts.
        assert!(!is_trlc_fqn(".Record"));
        assert!(!is_trlc_fqn("Package."));

        // Invalid: bad leading character or illegal characters.
        assert!(!is_trlc_fqn("1bad.Name"));
        assert!(!is_trlc_fqn("good.2bad"));
        assert!(!is_trlc_fqn("has space.Name"));
        assert!(!is_trlc_fqn("dash-ed.Name"));
    }

    // ── Public empty writer API ────────────────────────────────────────────

    fn unique_tmp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "puml_idmap_{}_{}_{}",
            tag,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn write_empty_idmap_to_file_emits_empty_arrays_to_disk() {
        let dir = unique_tmp_dir("empty_writer");
        let input = Path::new("some/dir/activity.puml");

        let output = write_empty_idmap_to_file(input, Some("score/activity.puml"), &dir)
            .expect("empty idmap must be written");

        assert_eq!(
            output.file_name().and_then(OsStr::to_str),
            Some("activity.idmap.json"),
            "output filename must be derived from the input stem"
        );

        let content = fs::read_to_string(&output).unwrap();
        let parsed: IdMapFile = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.source, "score/activity.puml");
        assert!(parsed.defines.is_empty());
        assert!(parsed.references.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_empty_idmap_to_file_falls_back_to_input_path_when_source_is_none() {
        let dir = unique_tmp_dir("empty_writer_fallback");
        let input = Path::new("rel/dir/diagram.puml");

        let output =
            write_empty_idmap_to_file(input, None, &dir).expect("empty idmap must be written");

        let content = fs::read_to_string(&output).unwrap();
        let parsed: IdMapFile = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.source, "rel/dir/diagram.puml");

        fs::remove_dir_all(&dir).ok();
    }

    // ── Sequence participant traversal ─────────────────────────────────────

    #[test]
    fn sequence_collect_participants_traverses_returns_and_nested_branches() {
        use sequence_logic::{Condition, ConditionType, Return};

        // Deeply nested interaction inside a control block (Condition), which
        // itself sits inside a Return node's branches. `collect_participants`
        // must recurse through both and pick up Deep/Nested, plus the Return's
        // own caller/callee (A/B). Condition contributes no participant itself.
        let deep = SequenceNode {
            event: Event::Interaction(Interaction {
                caller: "Deep".to_string(),
                callee: "Nested".to_string(),
                method: "call".to_string(),
            }),
            source_location: SourceLocation::new("test.puml", 0),
            branches_node: Vec::new(),
        };
        let control_block = SequenceNode {
            event: Event::Condition(Condition {
                condition_type: ConditionType::Alt,
                condition_value: "ok?".to_string(),
            }),
            source_location: SourceLocation::new("test.puml", 0),
            branches_node: vec![deep],
        };
        let ret = SequenceNode {
            event: Event::Return(Return {
                caller: "B".to_string(),
                callee: "A".to_string(),
                return_content: "ok".to_string(),
            }),
            source_location: SourceLocation::new("test.puml", 0),
            branches_node: vec![control_block],
        };
        let tree = SequenceTree {
            name: None,
            root_interactions: vec![ret],
        };

        let idmap = sequence_model_to_idmap(&tree, "pkg/seq.puml");

        assert!(idmap.defines.is_empty());
        let ids: Vec<&str> = idmap.references.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["A", "B", "Deep", "Nested"]);
    }
}
