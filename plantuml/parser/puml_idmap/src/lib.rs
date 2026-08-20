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
use component_diagram::LogicComponent;
use puml_fta::{FtaModel, GateKind, NodeKind};
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct IdMapEntry {
    /// PlantUML alias used in `url of <alias> is [[url]]` injection.
    pub alias: String,
    /// Fully-qualified identifier (FQN) for matching across diagrams.
    pub id: String,
    /// `true` when this define was synthesized for a namespace/package
    /// container rather than elaborated as an entity of its own (see
    /// `class_model_to_idmap`'s namespace synthesis). `clickable_plantuml`
    /// only trusts a synthesized define when no non-synthesized diagram
    /// elaborates the same element, so a namespace merely used for FQN
    /// containment in several class diagrams never outranks or ties with
    /// its real elaboration site (e.g. a `static` component diagram).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub synthesized: bool,
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
/// All remaining elements are **references** (top-level leaves that mention
/// something that may be detailed in another diagram). `package` elements
/// are not special-cased: an empty package (no children) is a reference like
/// any other leaf, while a package with children is a define, exactly like a
/// `component` with nested elements.
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
            ..Default::default()
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
    // This is a structural invariant of `ComponentResolver`'s output, not a
    // convention shared with `class_model_to_idmap` below — the class-diagram
    // resolver *can* leave relationship endpoints that reference no known
    // entity (see the dangling-endpoint handling and warning there), which is
    // why that converter, unlike this one, must scan `relationships` at all.
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
/// references so links can resolve from relationship mentions. Namespaces are
/// also synthesized as defines (see below) so a unit/component elaborated
/// only via `namespace <name> { class RealClass {...} }` remains linkable.
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

    // The resolver never materialises a namespace/package as an entity of its
    // own (only its FQN survives on each child's `enclosing_namespace_id`), so
    // no namespace can reach the entity loop below by itself. Collected here
    // so relationship endpoints that only name a container can be recognised
    // below.
    let namespace_ids: HashSet<&str> = model
        .entities
        .iter()
        .filter_map(|e| e.enclosing_namespace_id.as_deref())
        .collect();
    // Namespaces that own at least one child this diagram elaborates as a
    // define; populated by the entity loop below and consumed afterwards to
    // synthesize namespace-level defines (see there for the rationale).
    let mut namespaces_with_defined_child: HashSet<&str> = HashSet::new();

    for entity in &model.entities {
        let has_members = !entity.methods.is_empty() || !entity.variables.is_empty();
        let matches_diagram_name = diagram_name == Some(entity.name.as_str());
        let is_define = has_members || matches_diagram_name;
        if is_define {
            define_ids.insert(entity.id.clone());
            defines.push(IdMapEntry {
                alias: entity.name.clone(),
                id: entity.id.clone(),
                ..Default::default()
            });
            if let Some(ns) = entity.enclosing_namespace_id.as_deref() {
                namespaces_with_defined_child.insert(ns);
            }
            continue;
        }

        if reference_ids.insert(entity.id.clone()) {
            references.push(IdMapEntry {
                alias: entity.name.clone(),
                id: entity.id.clone(),
                ..Default::default()
            });
        }
    }

    // Synthesize a define for each namespace/package FQN that has at least one
    // directly-nested child this diagram itself elaborates (i.e. a define, not
    // a memberless stub shown only for collaborator context). A namespace is a
    // valid elaboration site even though the resolver never gives it its own
    // entity: e.g. a unit elaborated only via `namespace unit_1 { class Foo {
    // ... } }` (a real implementation class, not a same-named placeholder)
    // must still be linkable from a component/sequence diagram's bare
    // `unit_1` reference. Without this, such units could only be made linkable
    // by inventing a fake same-named class, which would fail the
    // class_design_implementation design-vs-implementation consistency check.
    // Requiring a defined child (rather than any child at all) matters when
    // the same namespace is *also* referenced as a memberless/collaborator
    // stub from an unrelated diagram elsewhere — that diagram must not become
    // a tied co-definer and turn every real link to this namespace ambiguous.
    // The alias is the namespace's own (last) path segment, matching how a
    // leaf component/participant's bare alias is derived.
    //
    // Marked `synthesized: true` so `clickable_plantuml` can tell this
    // incidental, per-file namespace define apart from a diagram whose actual
    // purpose is to elaborate that element (e.g. its `static` component
    // diagram). Several unrelated class diagrams routinely nest their
    // entities under the same shared namespace purely to reflect FQN
    // containment; without this flag every one of them would tie as a
    // co-definer of that namespace and the real elaboration site would never
    // win, silently breaking the link.
    for &ns in &namespaces_with_defined_child {
        if define_ids.contains(ns) || reference_ids.contains(ns) {
            // An actual entity already owns this id; don't shadow it with a
            // synthetic namespace define.
            continue;
        }
        let alias = ns.rsplit('.').next().unwrap_or(ns).to_string();
        define_ids.insert(ns.to_string());
        defines.push(IdMapEntry {
            alias,
            id: ns.to_string(),
            synthesized: true,
        });
    }

    // Relationships are now stored per-entity (`SimpleEntity::relationships`)
    // rather than on the diagram as a whole; flatten across all entities.
    for relationship in model.entities.iter().flat_map(|e| &e.relationships) {
        for endpoint in [&relationship.source, &relationship.target] {
            // A relation endpoint that is only a namespace/package container is
            // not a linkable element; never emit it as a reference.
            if namespace_ids.contains(endpoint.as_str()) {
                continue;
            }
            if define_ids.contains(endpoint) || !reference_ids.insert(endpoint.clone()) {
                continue;
            }

            // A relationship endpoint that matches neither a known entity nor a
            // known namespace is a dangling/orphan reference (e.g. a typo'd id,
            // or an id from a diagram the resolver didn't see). Emit it by its
            // raw id so the link still round-trips, but warn so a malformed
            // diagram is visible in the build log rather than silently guessed
            // at — mirroring how `puml_fta` warns on malformed aliases instead
            // of failing the build.
            let alias = match entity_name_by_id.get(endpoint.as_str()) {
                Some(&name) => name.to_string(),
                None => {
                    log::warn!(
                        "class diagram relationship endpoint {:?} does not match any \
                         known entity or namespace in {source}; emitting it as a \
                         reference by its raw id",
                        endpoint,
                    );
                    endpoint.clone()
                }
            };

            references.push(IdMapEntry {
                alias,
                id: endpoint.clone(),
                ..Default::default()
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
    tree.participant_reference_names().collect()
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
            ..Default::default()
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
/// A gate node is a **reference** only when [`FtaNode::gate_kind`] is
/// `GateKind::TransferIn` (a `$TransferInGate` pointing to another diagram's
/// top event); `GateKind::And`/`GateKind::Or` gates are internal and produce
/// no cross-diagram link. All other nodes (basic/intermediate events) are
/// likewise internal.
fn fta_model_to_idmap(model: &FtaModel, source: &str) -> IdMapFile {
    let mut defines = Vec::new();
    let mut references = Vec::new();

    for node in &model.nodes {
        match node.kind {
            NodeKind::TopEvent => {
                defines.push(IdMapEntry {
                    alias: node.alias.clone(),
                    id: node.alias.clone(),
                    ..Default::default()
                });
            }
            NodeKind::Gate if node.gate_kind == Some(GateKind::TransferIn) => {
                if !is_trlc_fqn(&node.alias) {
                    log::warn!(
                        "FTA $TransferInGate {:?} does not look like a TRLC \
                         fully-qualified name (expected 'Package.Record'); \
                         emitting it as a reference anyway",
                        node.alias,
                    );
                }
                references.push(IdMapEntry {
                    alias: node.alias.clone(),
                    id: node.alias.clone(),
                    ..Default::default()
                });
            }
            _ => {} // BasicEvent, IntermediateEvent, $AndGate/$OrGate — internal, no link.
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

/// Compute the `<stem>.idmap.json` output path for `input_path` in
/// `output_dir`, without writing anything.
///
/// Exposed so callers that process multiple input files in one run (e.g. the
/// CLI) can pre-check for filename collisions *before* writing: two `.puml`
/// files in different directories that share a file stem would otherwise
/// silently overwrite each other's `.idmap.json` output, since the filename
/// is derived from the stem alone.
pub fn idmap_output_path(input_path: &Path, output_dir: &Path) -> PathBuf {
    let file_stem = input_path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("output");
    output_dir.join(format!("{file_stem}.idmap.json"))
}

fn write_idmap_json(
    input_path: &Path,
    output_dir: &Path,
    idmap: &IdMapFile,
) -> io::Result<PathBuf> {
    let output_path = idmap_output_path(input_path, output_dir);

    let json = serde_json::to_string_pretty(idmap)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
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
    use class_diagram::{MemberVariable, Method, RelationType, Relationship, SimpleEntity};
    use component_diagram::{ComponentType, SourceLocation};
    use puml_fta::FtaNode;
    use sequence_logic::{Block, Interaction, Node, ParticipantType, SequenceParticipant};

    fn sequence_interaction(sender: &str, receiver: &str) -> Node {
        Node::Interaction(Interaction {
            sender: Some(sender.to_string().into()),
            receiver: Some(receiver.to_string().into()),
            message: Some("call".to_string()),
            source_location: SourceLocation::new("test.puml", 0),
        })
    }

    fn sequence_tree(participants: &[&str], items: Vec<Node>) -> SequenceTree {
        SequenceTree {
            name: None,
            participants: participants
                .iter()
                .map(|name| sequence_participant(name))
                .collect(),
            root: Block { items },
        }
    }

    fn sequence_participant(name: &str) -> SequenceParticipant {
        SequenceParticipant {
            display_name: name.to_string(),
            alias: None,
            participant_type: ParticipantType::Participant,
            source_location: SourceLocation::new("test.puml", 0),
            stereotype: None,
        }
    }

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
        };

        let idmap = class_model_to_idmap(&model, "pkg/class_sorted.puml");

        let define_ids: Vec<&str> = idmap.defines.iter().map(|e| e.id.as_str()).collect();
        let ref_ids: Vec<&str> = idmap.references.iter().map(|e| e.id.as_str()).collect();

        assert_eq!(define_ids, ["pkg.A", "pkg.Z"]);
        assert_eq!(ref_ids, ["pkg.B", "pkg.M"]);
    }

    #[test]
    fn sequence_participants_become_sorted_references() {
        let tree = sequence_tree(
            &["Zebra", "Alpha", "Mango"],
            vec![
                sequence_interaction("Zebra", "Alpha"),
                sequence_interaction("Alpha", "Mango"),
            ],
        );

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
    fn package_with_children_is_a_define() {
        let mut pkg = component("Pkg", Some("Pkg"), None, None);
        pkg.element_type = ComponentType::Package;
        let model = component_map(vec![
            pkg,
            component("A", Some("A"), None, Some("Pkg")),
            component("AA", Some("AA"), None, Some("A")),
        ]);

        let idmap = comp_model_to_idmap(&model, "pkg.puml", None);
        assert!(idmap.defines.iter().any(|e| e.id == "Pkg"));
        assert!(!idmap.references.iter().any(|e| e.id == "Pkg"));
    }

    #[test]
    fn empty_package_is_a_reference() {
        let mut pkg = component("Pkg", Some("Pkg"), None, None);
        pkg.element_type = ComponentType::Package;
        let model = component_map(vec![pkg]);

        let idmap = comp_model_to_idmap(&model, "pkg.puml", None);
        assert!(!idmap.defines.iter().any(|e| e.id == "Pkg"));
        assert!(idmap.references.iter().any(|e| e.id == "Pkg"));
    }

    #[test]
    fn class_relationship_endpoints_are_emitted_as_references() {
        let define = SimpleEntity {
            id: "pkg.Define".to_string(),
            name: "Define".to_string(),
            variables: vec![MemberVariable::default()],
            relationships: vec![Relationship {
                source: "pkg.Define".to_string(),
                target: "pkg.ExternalRef".to_string(),
                relation_type: RelationType::Association,
                source_multiplicity: None,
                target_multiplicity: None,
                ..Default::default()
            }],
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![define],
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
            relationships: vec![Relationship {
                source: "pkg.A".to_string(),
                target: "pkg.B".to_string(),
                relation_type: RelationType::Association,
                source_multiplicity: None,
                target_multiplicity: None,
                ..Default::default()
            }],
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
        // are not double-counted as references (they're already synthesized
        // as a define below), so the endpoint must be dropped from references.
        let child = SimpleEntity {
            id: "pkg.Container.Child".to_string(),
            name: "Child".to_string(),
            enclosing_namespace_id: Some("pkg.Container".to_string()),
            variables: vec![MemberVariable::default()],
            relationships: vec![Relationship {
                source: "pkg.Container.Child".to_string(),
                target: "pkg.Container".to_string(),
                relation_type: RelationType::Association,
                source_multiplicity: None,
                target_multiplicity: None,
                ..Default::default()
            }],
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![child],
        };

        let idmap = class_model_to_idmap(&model, "pkg/ns.puml");
        assert!(
            !idmap.references.iter().any(|e| e.id == "pkg.Container"),
            "namespace container must not leak as a reference, got: {:?}",
            idmap.references
        );
        assert!(
            idmap.defines.iter().any(|e| e.id == "pkg.Container"),
            "namespace container must be synthesized as a define so units \
             elaborated only via a namespace (not a same-named class) stay \
             linkable, got: {:?}",
            idmap.defines
        );
    }

    #[test]
    fn class_namespace_container_as_relationship_source_is_not_emitted_as_reference() {
        // Symmetric to the test above, but the container FQN is the
        // relationship's `source` rather than its `target` — the endpoint
        // filter must apply to both sides, not just `target`.
        // NOTE: a relationship whose `source` is the container FQN itself
        // couldn't be produced by the real resolver (relationships are always
        // pushed onto a concrete entity, and containers are never entities),
        // but the idmap converter must stay defensive either way — attach it
        // to the concrete child entity to exercise the `source` endpoint side
        // of the filter.
        let child = SimpleEntity {
            id: "pkg.Container.Child".to_string(),
            name: "Child".to_string(),
            enclosing_namespace_id: Some("pkg.Container".to_string()),
            variables: vec![MemberVariable::default()],
            relationships: vec![Relationship {
                source: "pkg.Container".to_string(),
                target: "pkg.Container.Child".to_string(),
                relation_type: RelationType::Association,
                source_multiplicity: None,
                target_multiplicity: None,
                ..Default::default()
            }],
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![child],
        };

        let idmap = class_model_to_idmap(&model, "pkg/ns.puml");
        assert!(
            !idmap.references.iter().any(|e| e.id == "pkg.Container"),
            "namespace container must not leak as a reference when it is the \
             relationship source, got: {:?}",
            idmap.references
        );
        assert!(idmap.defines.iter().any(|e| e.id == "pkg.Container"));
    }

    #[test]
    fn class_namespace_define_alias_is_last_path_segment_and_id_is_full_fqn() {
        // Real-world shape: a unit elaborated only via
        // `namespace unit_1 { class Foo { ... } }` — Foo is a real
        // implementation class (not a same-named placeholder for the unit),
        // so the only way a bare `unit_1` reference elsewhere can resolve to
        // this diagram is via a synthesized define for the namespace itself.
        let foo = SimpleEntity {
            id: "safety_software_seooc_example.component_example.unit_1.Foo".to_string(),
            name: "Foo".to_string(),
            enclosing_namespace_id: Some(
                "safety_software_seooc_example.component_example.unit_1".to_string(),
            ),
            methods: vec![Method::default()],
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "unit_1_class_diagram".to_string(),
            entities: vec![foo],
        };

        let idmap = class_model_to_idmap(&model, "unit_1/docs/unit_1_class_diagram.puml");

        let namespace_define = idmap
            .defines
            .iter()
            .find(|e| e.id == "safety_software_seooc_example.component_example.unit_1")
            .expect("namespace must be synthesized as a define");
        assert_eq!(namespace_define.alias, "unit_1");
    }

    #[test]
    fn class_namespace_define_does_not_shadow_a_real_entity_with_the_same_id() {
        // Defensive: if a namespace FQN happens to collide with a genuine
        // entity's id (e.g. a dangling relationship endpoint already resolved
        // as a reference, or - in principle - a define), the synthesized
        // namespace define must not duplicate or shadow it.
        let child = SimpleEntity {
            id: "pkg.Container.Child".to_string(),
            name: "Child".to_string(),
            enclosing_namespace_id: Some("pkg.Container".to_string()),
            variables: vec![MemberVariable::default()],
            ..Default::default()
        };
        let container_as_real_entity = SimpleEntity {
            id: "pkg.Container".to_string(),
            name: "Container".to_string(),
            methods: vec![Method::default()],
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![child, container_as_real_entity],
        };

        let idmap = class_model_to_idmap(&model, "pkg/ns.puml");

        assert_eq!(
            idmap
                .defines
                .iter()
                .filter(|e| e.id == "pkg.Container")
                .count(),
            1,
            "the real entity must not be duplicated by the namespace synthesis, got: {:?}",
            idmap.defines
        );
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
        };

        let idmap = class_model_to_idmap(&model, "pkg/proxy.puml");
        assert!(idmap.defines.iter().any(|e| e.id == "pkg.Proxy"));
        assert!(idmap.references.iter().any(|e| e.id == "pkg.Leaf"));
        assert!(!idmap.defines.iter().any(|e| e.id == "pkg.Leaf"));
    }

    // ── FTA converter ──────────────────────────────────────────────────────

    fn fta_node(kind: NodeKind, gate_kind: Option<GateKind>, alias: &str) -> FtaNode {
        FtaNode {
            kind,
            name: None,
            alias: alias.to_string(),
            connection: None,
            gate_kind,
            line: None,
        }
    }

    #[test]
    fn fta_top_event_is_define_and_transfer_gate_is_reference() {
        let model = FtaModel {
            nodes: vec![
                fta_node(NodeKind::TopEvent, None, "pkg.TopFailure"),
                // $TransferInGate: alias is a foreign top-event FQN → reference.
                fta_node(
                    NodeKind::Gate,
                    Some(GateKind::TransferIn),
                    "other.ForeignTop",
                ),
                // $AndGate/$OrGate are internal regardless of alias shape —
                // classification is driven by `gate_kind`, not the alias, so a
                // dotted alias on a non-transfer gate must NOT be misread as a
                // cross-diagram reference (regression test).
                fta_node(NodeKind::Gate, Some(GateKind::And), "And.Gate1"),
                fta_node(NodeKind::Gate, Some(GateKind::Or), "OG"),
                // Basic events are internal even when the alias looks like an FQN.
                fta_node(NodeKind::BasicEvent, None, "pkg.Cause"),
                fta_node(NodeKind::IntermediateEvent, None, "IE"),
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

    // ── Public write_idmap_to_file dispatch ─────────────────────────────────

    #[test]
    fn write_idmap_to_file_writes_component_dispatch_to_disk() {
        let dir = unique_tmp_dir("write_component");
        let model = component_map(vec![
            component("Proxy", Some("Proxy"), None, None),
            component("Handler", Some("Handler"), None, Some("Proxy")),
        ]);
        let input = Path::new("some/dir/proxy.puml");

        let output = write_idmap_to_file(
            IdMapModel::Component(&model),
            input,
            Some("pkg/proxy.puml"),
            None,
            &dir,
        )
        .expect("component idmap must be written");

        assert_eq!(
            output.file_name().and_then(OsStr::to_str),
            Some("proxy.idmap.json")
        );
        let parsed: IdMapFile =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(parsed.source, "pkg/proxy.puml");
        assert_eq!(
            parsed
                .defines
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["Proxy"]
        );
        assert_eq!(
            parsed
                .references
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["Handler"]
        );

        cleanup_tmp_dir(&dir);
    }

    #[test]
    fn write_idmap_to_file_writes_class_dispatch_to_disk() {
        let dir = unique_tmp_dir("write_class");
        let with_members = SimpleEntity {
            id: "pkg.WithMembers".to_string(),
            name: "WithMembers".to_string(),
            variables: vec![MemberVariable::default()],
            ..Default::default()
        };
        let model = ClassDiagram {
            name: "d".to_string(),
            entities: vec![with_members],
        };
        let input = Path::new("some/dir/classes.puml");

        let output = write_idmap_to_file(
            IdMapModel::Class(&model),
            input,
            Some("pkg/classes.puml"),
            None,
            &dir,
        )
        .expect("class idmap must be written");

        let parsed: IdMapFile =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(parsed.source, "pkg/classes.puml");
        assert_eq!(
            parsed
                .defines
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["pkg.WithMembers"]
        );

        cleanup_tmp_dir(&dir);
    }

    #[test]
    fn write_idmap_to_file_writes_sequence_dispatch_to_disk() {
        let dir = unique_tmp_dir("write_sequence");
        let tree = sequence_tree(
            &["Alpha", "Beta"],
            vec![sequence_interaction("Alpha", "Beta")],
        );
        let input = Path::new("some/dir/seq.puml");

        let output = write_idmap_to_file(
            IdMapModel::Sequence(&tree),
            input,
            Some("pkg/seq.puml"),
            None,
            &dir,
        )
        .expect("sequence idmap must be written");

        let parsed: IdMapFile =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(parsed.source, "pkg/seq.puml");
        assert!(parsed.defines.is_empty());
        assert_eq!(
            parsed
                .references
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["Alpha", "Beta"]
        );

        cleanup_tmp_dir(&dir);
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

    /// Remove a test's temp dir, surfacing (rather than silently swallowing)
    /// any cleanup failure so a locked/undeletable dir doesn't go unnoticed.
    fn cleanup_tmp_dir(dir: &Path) {
        if let Err(e) = fs::remove_dir_all(dir) {
            eprintln!(
                "warning: failed to remove temp dir {}: {}",
                dir.display(),
                e
            );
        }
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

        cleanup_tmp_dir(&dir);
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

        cleanup_tmp_dir(&dir);
    }

    // ── Sequence participant traversal ─────────────────────────────────────

    #[test]
    fn sequence_collect_participants_from_sequence_tree() {
        let tree = SequenceTree {
            name: None,
            participants: vec![
                sequence_participant("B"),
                sequence_participant("A"),
                sequence_participant("Deep"),
                sequence_participant("Nested"),
            ],
            root: Block::default(),
        };

        let idmap = sequence_model_to_idmap(&tree, "pkg/seq.puml");

        assert!(idmap.defines.is_empty());
        let ids: Vec<&str> = idmap.references.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["A", "B", "Deep", "Nested"]);
    }

    #[test]
    fn sequence_declared_but_unused_participant_is_still_a_reference() {
        let tree = SequenceTree {
            name: None,
            participants: vec![
                sequence_participant("Alice"),
                sequence_participant("Bob"),
                sequence_participant("Idle"),
            ],
            root: Block {
                items: vec![sequence_interaction("Alice", "Bob")],
            },
        };

        let idmap = sequence_model_to_idmap(&tree, "pkg/seq.puml");

        let ids: Vec<&str> = idmap.references.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["Alice", "Bob", "Idle"]);
    }
}
