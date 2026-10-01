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
use uid_normalization::{leaf_key, resolve_reference, InternalScope, Resolution, RootAnchor};

#[derive(Debug, thiserror::Error)]
pub enum ComponentResolverError {
    #[error("Element Resolver: UnresolvedReference: {reference}")]
    UnresolvedReference { reference: String },

    #[error("Element Resolver: AmbiguousReference: {reference} -> {candidates:?}")]
    AmbiguousReference {
        reference: String,
        candidates: Vec<String>,
    },

    #[error("Duplicate element id: {element_id}")]
    DuplicateElement { element_id: String },

    #[error("Unknown element type: {element_type}")]
    UnknownElementType { element_type: String },

    #[error("Element at {source_location:?} has neither a name nor an alias")]
    MissingElementIdentity { source_location: SourceLocation },

    #[error("Invalid relationship: {from} -> {to}: {reason}")]
    InvalidRelationship {
        from: String,
        to: String,
        reason: String,
    },
}

#[derive(Clone)]
struct PendingRelation {
    scope: InternalScope,
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
    scope: InternalScope,
    root_anchor: RootAnchor,
    pub elements: HashMap<String, LogicComponent>, // FQN -> LogicComponent
    /// Maps port FQN → parent element FQN (for relation lifting)
    pub port_parents: HashMap<String, String>,
    /// Maps port FQN -> parser port type (`port` / `portin` / `portout`)
    pub port_types: HashMap<String, PortType>,
    // leaf (Rule A, last segment) -> every element id registered under it
    element_leaves: HashMap<String, Vec<String>>,
    // same for ports
    port_leaves: HashMap<String, Vec<String>>,
    pending_relations: Vec<PendingRelation>,
}

impl ComponentResolver {
    pub fn new() -> Self {
        Self::with_root_anchor(None)
    }

    /// Resolver whose ids are prefixed with `root_anchor`.
    pub fn with_root_anchor(root_anchor: Option<&str>) -> Self {
        Self {
            root_anchor: RootAnchor::new(root_anchor),
            ..Self::default()
        }
    }

    fn port_type_to_role(port_type: PortType) -> EndpointRole {
        match port_type {
            PortType::Port => EndpointRole::None,
            PortType::PortIn => EndpointRole::Required,
            PortType::PortOut => EndpointRole::Provided,
        }
    }

    fn lookup(
        &self,
        raw: &str,
        exists: impl Fn(&str) -> bool,
        leaves: &HashMap<String, Vec<String>>,
    ) -> Resolution {
        resolve_reference(
            &self.scope,
            &self.root_anchor,
            raw,
            &exists,
            &exists,
            |leaf| leaves.get(leaf).cloned().unwrap_or_default(),
        )
    }

    /// Rule C lookup over the whole diagram. Elements first; when none
    /// matches, ports, which resolve to their owning element. Returns the
    /// element id and the role of the port the reference named, if any.
    fn resolve_endpoint(
        &self,
        raw: &str,
    ) -> Result<(String, Option<EndpointRole>), ComponentResolverError> {
        let by_element = self.lookup(
            raw,
            |id| self.elements.contains_key(id),
            &self.element_leaves,
        );
        let (resolution, via_port) = match by_element {
            Resolution::Unresolved => (
                self.lookup(
                    raw,
                    |id| self.port_parents.contains_key(id),
                    &self.port_leaves,
                ),
                true,
            ),
            found => (found, false),
        };

        match resolution {
            Resolution::Resolved(port) if via_port => {
                let role = self
                    .port_types
                    .get(&port)
                    .copied()
                    .map(Self::port_type_to_role);
                Ok((self.port_parents[&port].clone(), role))
            }
            Resolution::Resolved(id) => Ok((id, None)),
            Resolution::Ambiguous(candidates) => Err(ComponentResolverError::AmbiguousReference {
                reference: raw.to_string(),
                candidates,
            }),
            Resolution::Unresolved => {
                error!("Unresolved reference: {}", raw);
                Err(ComponentResolverError::UnresolvedReference {
                    reference: raw.to_string(),
                })
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
        self.scope = InternalScope::default();
        self.elements.clear();
        self.port_parents.clear();
        self.port_types.clear();
        self.element_leaves.clear();
        self.port_leaves.clear();
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
            Statement::Port(port) => {
                self.visit_port(port);
                Ok(())
            }
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
    fn visit_port(&mut self, port: &Port) {
        let local_id = port.alias.as_deref().unwrap_or(&port.name);
        let fqn = self.scope.resolve_with_leaf(&self.root_anchor, local_id);

        if self.scope.is_empty() {
            // Top-level ports are pure connectors/aliases, not entities — ignore them.
            // Use `interface` to declare a top-level interface as a first-class entity.
        } else {
            // Nested port: record port_fqn -> parent_fqn for relation lifting.
            self.port_types.insert(fqn.clone(), port.port_type);
            self.port_leaves
                .entry(leaf_key(local_id))
                .or_default()
                .push(fqn.clone());
            self.port_parents
                .insert(fqn, self.scope.resolve(&self.root_anchor));
        }
    }

    fn visit_element(&mut self, element: &Element) -> Result<(), ComponentResolverError> {
        let local_id = element
            .identity
            .alias
            .as_deref()
            .or(element.identity.name.as_deref())
            .ok_or_else(|| ComponentResolverError::MissingElementIdentity {
                source_location: element.identity.source_location.clone(),
            })?;

        let fqn = self.scope.resolve_with_leaf(&self.root_anchor, local_id);
        if self.elements.contains_key(&fqn) {
            return Err(ComponentResolverError::DuplicateElement { element_id: fqn });
        }

        let parent_id = (!self.scope.is_empty()).then(|| self.scope.resolve(&self.root_anchor));

        let logic = LogicComponent {
            id: fqn.clone(),
            name: element.identity.name.clone(),
            alias: element.identity.alias.clone(),
            source_location: element.identity.source_location.clone(),
            parent_id,
            element_type: parse_kind(&element.identity.element_kind)?,
            stereotype: element.identity.stereotype.clone(),
            relations: Vec::new(),
        };

        self.elements.insert(fqn.clone(), logic);
        self.element_leaves
            .entry(leaf_key(local_id))
            .or_default()
            .push(fqn);

        let nested = self.scope.child(local_id);
        let outer = std::mem::replace(&mut self.scope, nested);

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
