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

use log::error;
use std::collections::HashMap;

use component_diagram::{
    ComponentRelationType, ComponentType, EndpointRole, LogicComponent, LogicRelation,
    SourceLocation,
};
use component_parser::{Arrow, CompPumlDocument, Element, Port, PortType, Relation, Statement};
use resolver_traits::DiagramResolver;
use uid_normalization::{
    identity_name, leaf_key, resolve_reference, strip_root_marker, DeclarationScope, IdentityKind,
    Resolution,
};

#[derive(Debug, thiserror::Error)]
pub enum ComponentResolverError {
    #[error("Element Resolver: UnresolvedReference: {reference}")]
    UnresolvedReference { reference: String },

    #[error(
        "Element Resolver: {reference} is the name of an element declared with alias {alias}; \
         refer to it by the alias"
    )]
    NameOfAliasedElement { reference: String, alias: String },

    #[error("Element Resolver: AmbiguousReference: {reference} -> {candidates:?}")]
    AmbiguousReference {
        reference: String,
        candidates: Vec<String>,
    },

    #[error("Duplicate element id: {element_id} (line {line})", line = source_location.line)]
    DuplicateElement {
        element_id: String,
        source_location: SourceLocation,
    },

    #[error("Duplicate alias: {alias} (line {line})", line = source_location.line)]
    DuplicateAlias {
        alias: String,
        source_location: SourceLocation,
    },

    #[error("Unknown element type: {element_type}")]
    UnknownElementType { element_type: String },

    #[error("Element at {source_location:?} has no name")]
    MissingElementIdentity { source_location: SourceLocation },

    #[error(
        "Invalid identifier: {name}{location}: {reason}",
        location = source_location
            .as_ref()
            .map(|location| format!(" (line {})", location.line))
            .unwrap_or_default()
    )]
    InvalidIdentifier {
        name: String,
        reason: String,
        source_location: Option<SourceLocation>,
    },

    #[error("Invalid relationship: {from} -> {to}: {reason}")]
    InvalidRelationship {
        from: String,
        to: String,
        reason: String,
    },
}

#[derive(Clone)]
struct PendingRelation {
    scope: DeclarationScope,
    relation: Relation,
}

#[derive(Clone)]
struct ArrowAnalysis {
    has_provided_token: bool,
    has_required_token: bool,
    has_direction: bool,
    reverse_direction: bool,
    decor_role: Option<EndpointRole>,
}

struct RelationValidationInput<'a> {
    relation: &'a Relation,
    has_interface_tokens: bool,
    src_is_interface: bool,
    tgt_is_interface: bool,
    src_is_component_role: bool,
    decor_role: Option<EndpointRole>,
    src_port_role: Option<EndpointRole>,
}

type RelationValidationRule = fn(&RelationValidationInput<'_>) -> Option<ComponentResolverError>;

#[derive(Default)]
pub struct ComponentResolver {
    scope: DeclarationScope,
    pub elements: HashMap<String, LogicComponent>, // id -> LogicComponent
    /// Maps port id → parent element id (for relation lifting)
    pub port_parents: HashMap<String, String>,
    /// Maps port id -> parser port type (`port` / `portin` / `portout`)
    pub port_types: HashMap<String, PortType>,
    // reference path -> element id
    element_refs: HashMap<String, String>,
    // leaf (last reference segment) -> every element reference path under it
    element_leaves: HashMap<String, Vec<String>>,
    // same for ports
    port_refs: HashMap<String, String>,
    port_leaves: HashMap<String, Vec<String>>,
    // id of every aliased element or port -> its alias
    aliased: HashMap<String, String>,
    pending_relations: Vec<PendingRelation>,
}

impl ComponentResolver {
    pub fn new() -> Self {
        Self::default()
    }

    fn port_type_to_role(port_type: PortType) -> EndpointRole {
        match port_type {
            PortType::Port => EndpointRole::None,
            PortType::PortIn => EndpointRole::Required,
            PortType::PortOut => EndpointRole::Provided,
        }
    }

    /// Rule C lookup over reference paths.
    fn lookup(
        &self,
        raw: &str,
        refs: &HashMap<String, String>,
        leaves: &HashMap<String, Vec<String>>,
    ) -> Resolution {
        let exists = |path: &str| refs.contains_key(path);
        resolve_reference(&self.scope.reference, raw, exists, exists, |leaf| {
            leaves.get(leaf).cloned().unwrap_or_default()
        })
    }

    /// Rule C lookup over the whole diagram. Elements first; when none
    /// matches, ports, which resolve to their owning element. Returns the
    /// element id and the role of the port the reference named, if any.
    fn resolve_endpoint(
        &self,
        raw: &str,
    ) -> Result<(String, Option<EndpointRole>), ComponentResolverError> {
        let by_element = self.lookup(raw, &self.element_refs, &self.element_leaves);
        let (resolution, via_port) = match by_element {
            Resolution::Unresolved => (self.lookup(raw, &self.port_refs, &self.port_leaves), true),
            found => (found, false),
        };
        let refs = if via_port {
            &self.port_refs
        } else {
            &self.element_refs
        };

        match resolution {
            Resolution::Resolved(path) if via_port => {
                let port = &refs[&path];
                let role = self
                    .port_types
                    .get(port)
                    .copied()
                    .map(Self::port_type_to_role);
                Ok((self.port_parents[port].clone(), role))
            }
            Resolution::Resolved(path) => Ok((refs[&path].clone(), None)),
            Resolution::Ambiguous(paths) => {
                let mut candidates: Vec<String> =
                    paths.iter().map(|path| refs[path].clone()).collect();
                candidates.sort();
                Err(ComponentResolverError::AmbiguousReference {
                    reference: raw.to_string(),
                    candidates,
                })
            }
            Resolution::Unresolved => Err(self.unresolved(raw)),
        }
    }

    /// `raw` names an aliased element by its name instead of its alias, or
    /// nothing at all.
    fn unresolved(&self, raw: &str) -> ComponentResolverError {
        let exists = |id: &str| self.aliased.contains_key(id);
        let by_name = resolve_reference(&self.scope.id, raw, exists, exists, |leaf| {
            self.aliased
                .keys()
                .filter(|id| leaf_key(id) == leaf)
                .cloned()
                .collect()
        });

        match by_name {
            Resolution::Resolved(id) => ComponentResolverError::NameOfAliasedElement {
                reference: raw.to_string(),
                alias: self.aliased[&id].clone(),
            },
            _ => {
                error!("Unresolved reference: {}", raw);
                ComponentResolverError::UnresolvedReference {
                    reference: raw.to_string(),
                }
            }
        }
    }
}

// Resolve Relationship
impl ComponentResolver {
    fn arrow_parts(arrow: &Arrow) -> (&str, &str, &str) {
        let left = arrow.left.as_ref().map(|d| d.raw.as_str()).unwrap_or("");
        let right = arrow.right.as_ref().map(|d| d.raw.as_str()).unwrap_or("");
        let middle = arrow
            .middle
            .as_ref()
            .and_then(|m| m.decorator.as_deref())
            .unwrap_or("");
        (left, right, middle)
    }

    // Supported relation syntaxes:
    // - Interface binding: `)-`, `-(`, `--()`
    // - Directed: `-->`, `<--`, `..>`, `<..`
    // - Undirected: `--`, `..`
    fn parse_arrow(relation: &Relation) -> Result<ArrowAnalysis, ComponentResolverError> {
        let (left, right, middle) = Self::arrow_parts(&relation.arrow);
        let line = relation.arrow.line.raw.as_str();

        let has_provided_token = left == ")" || left == "()" || middle == "()";
        let has_required_token = middle == "(" || right == "(";

        if has_provided_token && has_required_token {
            return Err(ComponentResolverError::InvalidRelationship {
                from: relation.lhs.clone(),
                to: relation.rhs.clone(),
                reason: "Mixed interface decorators are not allowed: cannot combine provided ')' with required '(' in one relation"
                    .to_string(),
            });
        }

        // A lollipop line may carry a direction hint, which adds a second dash
        // segment: `)-u-` or `-u-(`.  The line field then contains `"--"` instead
        // of `"-"`.  Direction is visual-only and does not affect semantics.
        let is_lollipop_line = line.chars().all(|c| c == '-') && !line.is_empty();

        let is_canonical_provided =
            is_lollipop_line && left == ")" && middle.is_empty() && right.is_empty();
        let is_generic_lollipop_provided =
            is_lollipop_line && left.is_empty() && middle == "()" && right.is_empty();
        let is_required = is_lollipop_line
            && left.is_empty()
            && ((middle == "(" && right.is_empty()) || (middle.is_empty() && right == "("));

        let decor_role = if is_canonical_provided || is_generic_lollipop_provided {
            Some(EndpointRole::Provided)
        } else if is_required {
            Some(EndpointRole::Required)
        } else {
            None
        };

        let has_direction = left.contains('<') || right.contains('>');
        let reverse_direction = left.contains('<') && !right.contains('>');

        Ok(ArrowAnalysis {
            has_provided_token,
            has_required_token,
            has_direction,
            reverse_direction,
            decor_role,
        })
    }

    fn infer_relation_type(parsed_arrow: &ArrowAnalysis) -> ComponentRelationType {
        if parsed_arrow.decor_role.is_some() {
            ComponentRelationType::InterfaceBinding
        } else if parsed_arrow.has_direction {
            ComponentRelationType::Dependency
        } else {
            ComponentRelationType::Association
        }
    }

    fn resolve_ref_with_metadata(
        &self,
        raw: &str,
    ) -> Result<(String, Option<EndpointRole>, Option<ComponentType>), ComponentResolverError> {
        let (resolved, port_role_hint) = self.resolve_endpoint(raw)?;

        let element_type = self.elements.get(&resolved).map(|e| e.element_type);

        Ok((resolved, port_role_hint, element_type))
    }

    fn validate_relation_constraints(
        &self,
        input: &RelationValidationInput<'_>,
    ) -> Result<(), ComponentResolverError> {
        let rules: [RelationValidationRule; 5] = [
            Self::rule_require_exactly_one_interface_endpoint,
            Self::rule_disallow_interface_to_interface,
            Self::rule_require_component_endpoint_for_binding,
            Self::rule_disallow_generic_decor_with_direction,
            Self::rule_port_role_consistency,
        ];

        for rule in rules {
            if let Some(err) = rule(input) {
                return Err(err);
            }
        }

        Ok(())
    }

    fn rule_require_exactly_one_interface_endpoint(
        input: &RelationValidationInput<'_>,
    ) -> Option<ComponentResolverError> {
        if input.has_interface_tokens && !input.src_is_interface && !input.tgt_is_interface {
            return Some(ComponentResolverError::InvalidRelationship {
                from: input.relation.lhs.clone(),
                to: input.relation.rhs.clone(),
                reason: "Interface decorators '-(', ')-', and '--()' require exactly one Interface endpoint"
                    .to_string(),
            });
        }
        None
    }

    fn rule_disallow_interface_to_interface(
        input: &RelationValidationInput<'_>,
    ) -> Option<ComponentResolverError> {
        if input.has_interface_tokens && input.src_is_interface && input.tgt_is_interface {
            return Some(ComponentResolverError::InvalidRelationship {
                from: input.relation.lhs.clone(),
                to: input.relation.rhs.clone(),
                reason: "Interface decorators '-(', ')-', and '--()' are not allowed between two interfaces"
                    .to_string(),
            });
        }
        None
    }

    fn rule_require_component_endpoint_for_binding(
        input: &RelationValidationInput<'_>,
    ) -> Option<ComponentResolverError> {
        if input.has_interface_tokens
            && input.decor_role.is_some()
            && (!input.src_is_component_role || !input.tgt_is_interface)
        {
            return Some(ComponentResolverError::InvalidRelationship {
                from: input.relation.lhs.clone(),
                to: input.relation.rhs.clone(),
                reason:
                    "Decorator binding requires a Component or component-stereotyped element on the left and Interface on the right"
                        .to_string(),
            });
        }
        None
    }

    fn rule_disallow_generic_decor_with_direction(
        input: &RelationValidationInput<'_>,
    ) -> Option<ComponentResolverError> {
        if input.has_interface_tokens
            && input.decor_role.is_none()
            && (input.src_is_interface || input.tgt_is_interface)
        {
            return Some(ComponentResolverError::InvalidRelationship {
                from: input.relation.lhs.clone(),
                to: input.relation.rhs.clone(),
                reason: "Unsupported interface decorator syntax: only ')-' and '--()' (Provided) plus '-(' (Required) are supported; '()--' is rejected"
                    .to_string(),
            });
        }
        None
    }
    fn rule_port_role_consistency(
        input: &RelationValidationInput<'_>,
    ) -> Option<ComponentResolverError> {
        if let (Some(port_role), Some(decor_role)) = (input.src_port_role, input.decor_role) {
            if port_role != decor_role {
                return Some(ComponentResolverError::InvalidRelationship {
                    from: input.relation.lhs.clone(),
                    to: input.relation.rhs.clone(),
                    reason: format!(
                        "Source endpoint role mismatch: port role {:?} conflicts with decorator role {:?}",
                        port_role, decor_role
                    ),
                });
            }
        }

        None
    }

    fn resolve_one_relation(&mut self, relation: &Relation) -> Result<(), ComponentResolverError> {
        let (mut src_fqn, mut src_port_role, mut src_type) =
            self.resolve_ref_with_metadata(&relation.lhs)?;

        let (mut tgt_fqn, mut tgt_port_role, mut tgt_type) =
            self.resolve_ref_with_metadata(&relation.rhs)?;

        let parsed_arrow = Self::parse_arrow(relation)?;
        if parsed_arrow.reverse_direction {
            std::mem::swap(&mut src_fqn, &mut tgt_fqn);
            std::mem::swap(&mut src_port_role, &mut tgt_port_role);
            std::mem::swap(&mut src_type, &mut tgt_type);
        }

        let src_is_interface = matches!(src_type, Some(ComponentType::Interface));
        let tgt_is_interface = matches!(tgt_type, Some(ComponentType::Interface));
        let src_is_component = matches!(src_type, Some(ComponentType::Component));
        let src_is_package = matches!(src_type, Some(ComponentType::Package));
        let src_stereotype = self
            .elements
            .get(&src_fqn)
            .and_then(|e| e.stereotype.as_deref());
        let src_is_component_role = src_is_component
            || (src_is_package && matches!(src_stereotype, Some("SEooC") | Some("component")));

        let validation_input = RelationValidationInput {
            relation,
            has_interface_tokens: parsed_arrow.has_provided_token
                || parsed_arrow.has_required_token,
            src_is_interface,
            tgt_is_interface,
            src_is_component_role,
            decor_role: parsed_arrow.decor_role,
            src_port_role,
        };

        self.validate_relation_constraints(&validation_input)?;

        let relation_type = Self::infer_relation_type(&parsed_arrow);

        let source_role = if relation_type == ComponentRelationType::InterfaceBinding {
            // Guard-only invariant check: InterfaceBinding should always carry a decorator role.
            // If this panics, resolver invariants have been broken by upstream logic changes.
            parsed_arrow
                .decor_role
                .expect("Invariant: InterfaceBinding requires decorator role")
        } else {
            EndpointRole::None
        };

        let source_element = self.elements.get_mut(&src_fqn).ok_or_else(|| {
            ComponentResolverError::UnresolvedReference {
                reference: src_fqn.clone(),
            }
        })?;

        let duplicate = source_element.relations.iter().any(|existing| {
            existing.target == tgt_fqn
                && existing.relation_type == relation_type
                && existing.source_role == source_role
        });

        if duplicate {
            return Ok(());
        }

        source_element.relations.push(LogicRelation {
            target: tgt_fqn,
            annotation: relation.description.clone(),
            relation_type,
            source_role,
            source_location: relation.source_location.clone(),
        });

        Ok(())
    }

    fn resolve_pending_relations(&mut self) -> Result<(), ComponentResolverError> {
        let pending_relations = std::mem::take(&mut self.pending_relations);

        for relation in pending_relations {
            let saved_scope = std::mem::replace(&mut self.scope, relation.scope);
            let res = self.resolve_one_relation(&relation.relation);
            self.scope = saved_scope;
            res?;
        }

        Ok(())
    }
}

impl DiagramResolver for ComponentResolver {
    type Document = CompPumlDocument;
    type Output = HashMap<String, LogicComponent>;
    type Error = ComponentResolverError;

    fn resolve(&mut self, document: &CompPumlDocument) -> Result<Self::Output, Self::Error> {
        self.scope = DeclarationScope::default();
        self.elements.clear();
        self.port_parents.clear();
        self.port_types.clear();
        self.element_refs.clear();
        self.element_leaves.clear();
        self.port_refs.clear();
        self.port_leaves.clear();
        self.aliased.clear();
        self.pending_relations.clear();

        for stmt in &document.statements {
            self.visit_statement(stmt)?;
        }

        self.resolve_pending_relations()?;

        Ok(self.elements.clone())
    }
}

impl ComponentResolver {
    fn visit_statement(&mut self, statement: &Statement) -> Result<(), ComponentResolverError> {
        match statement {
            Statement::Element(element) => {
                self.visit_element(element)?;
                Ok(())
            }
            Statement::Port(port) => self.visit_port(port),
            Statement::Relation(relation) => {
                self.pending_relations.push(PendingRelation {
                    scope: self.scope.clone(),
                    relation: relation.clone(),
                });
                Ok(())
            }
        }
    }
}

impl ComponentResolver {
    fn visit_port(&mut self, port: &Port) -> Result<(), ComponentResolverError> {
        let text = match Self::identity(&port.name, Some(&port.source_location)) {
            Ok(text) => text,
            // a top-level port is no entity, its name is never an identity
            Err(_) if self.scope.id.is_empty() => return Ok(()),
            Err(error) => return Err(error),
        };
        let declared = self.scope.declare(&text, port.alias.as_deref());
        let fqn = declared.id.id();

        let parent = declared.id.parent();
        if parent.is_empty() {
            // Top-level ports are pure connectors/aliases, not entities — ignore them.
            // Use `interface` to declare a top-level interface as a first-class entity.
            return Ok(());
        }

        // Nested port: record port id -> parent id for relation lifting.
        let reference = declared.reference.id();
        self.check_unique(&fqn, &reference, &port.source_location)?;
        self.port_types.insert(fqn.clone(), port.port_type);
        self.port_leaves
            .entry(leaf_key(&reference))
            .or_default()
            .push(reference.clone());
        self.port_refs.insert(reference, fqn.clone());
        if let Some(alias) = &port.alias {
            self.aliased.insert(fqn.clone(), alias.clone());
        }
        self.port_parents.insert(fqn, parent.id());
        Ok(())
    }

    /// An id or reference path may belong to one element or port only.
    fn check_unique(
        &self,
        id: &str,
        reference: &str,
        source_location: &SourceLocation,
    ) -> Result<(), ComponentResolverError> {
        if self.elements.contains_key(id) || self.port_parents.contains_key(id) {
            return Err(ComponentResolverError::DuplicateElement {
                element_id: id.to_string(),
                source_location: source_location.clone(),
            });
        }
        if self.element_refs.contains_key(reference) || self.port_refs.contains_key(reference) {
            return Err(ComponentResolverError::DuplicateAlias {
                alias: leaf_key(reference),
                source_location: source_location.clone(),
            });
        }
        Ok(())
    }

    fn identity(
        name: &str,
        source_location: Option<&SourceLocation>,
    ) -> Result<String, ComponentResolverError> {
        identity_name(name, IdentityKind::Other).map_err(|error| {
            ComponentResolverError::InvalidIdentifier {
                name: name.to_string(),
                reason: error.reason().to_string(),
                source_location: source_location.cloned(),
            }
        })
    }

    fn visit_element(&mut self, element: &Element) -> Result<(), ComponentResolverError> {
        let name = element.identity.name.as_deref().ok_or_else(|| {
            ComponentResolverError::MissingElementIdentity {
                source_location: element.identity.source_location.clone(),
            }
        })?;
        let text = Self::identity(name, Some(&element.identity.source_location))?;

        let declared = self.scope.declare(&text, element.identity.alias.as_deref());
        let fqn = declared.id.id();
        let reference = declared.reference.id();
        self.check_unique(&fqn, &reference, &element.identity.source_location)?;

        let parent = declared.id.parent();
        let parent_id = (!parent.is_empty()).then(|| parent.id());

        let logic = LogicComponent {
            id: fqn.clone(),
            name: Some(strip_root_marker(&text).to_string()),
            alias: element.identity.alias.clone(),
            source_location: element.identity.source_location.clone(),
            parent_id,
            element_type: parse_kind(&element.identity.element_kind)?,
            stereotype: element.identity.stereotype.clone(),
            relations: Vec::new(),
        };

        self.elements.insert(fqn.clone(), logic);
        self.element_leaves
            .entry(leaf_key(&reference))
            .or_default()
            .push(reference.clone());
        self.element_refs.insert(reference, fqn.clone());
        if let Some(alias) = &element.identity.alias {
            self.aliased.insert(fqn, alias.clone());
        }

        let outer = std::mem::replace(&mut self.scope, declared);

        for stmt in &element.statements {
            self.visit_statement(stmt)?;
        }

        self.scope = outer;

        Ok(())
    }
}

const ELEMENT_TYPE_TABLE: &[(&str, ComponentType)] = &[
    ("artifact", ComponentType::Artifact),
    ("actor", ComponentType::Actor),
    ("agent", ComponentType::Agent),
    ("boundary", ComponentType::Boundary),
    ("card", ComponentType::Card),
    ("cloud", ComponentType::Cloud),
    ("component", ComponentType::Component),
    ("control", ComponentType::Control),
    ("database", ComponentType::Database),
    ("entity", ComponentType::Entity),
    ("file", ComponentType::File),
    ("folder", ComponentType::Folder),
    ("frame", ComponentType::Frame),
    ("hexagon", ComponentType::Hexagon),
    ("interface", ComponentType::Interface),
    ("node", ComponentType::Node),
    ("package", ComponentType::Package),
    ("queue", ComponentType::Queue),
    ("rectangle", ComponentType::Rectangle),
    ("stack", ComponentType::Stack),
    ("storage", ComponentType::Storage),
    ("usecase", ComponentType::Usecase),
];

pub fn parse_kind(raw: &str) -> Result<ComponentType, ComponentResolverError> {
    ELEMENT_TYPE_TABLE
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(raw))
        .map(|(_, v)| *v)
        .ok_or_else(|| ComponentResolverError::UnknownElementType {
            element_type: raw.into(),
        })
}
