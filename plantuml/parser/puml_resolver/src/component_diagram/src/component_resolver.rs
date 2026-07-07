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
use std::collections::{BTreeSet, HashMap};

use component_diagram::{
    ComponentRelationType, ComponentType, EndpointRole, LogicComponent, LogicRelation,
};
use component_parser::{Arrow, CompPumlDocument, Element, Port, PortType, Relation, Statement};
use resolver_traits::DiagramResolver;

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

    #[error("Invalid relationship: {from} -> {to}: {reason}")]
    InvalidRelationship {
        from: String,
        to: String,
        reason: String,
    },
}

#[derive(Clone)]
struct PendingRelation {
    scope: Vec<String>,
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
    pub scope: Vec<String>,                        // element id stack
    pub elements: HashMap<String, LogicComponent>, // FQN -> LogicComponent
    /// Maps parent FQN -> direct child element FQNs
    pub child_elements_by_parent: HashMap<Option<String>, Vec<String>>,
    /// Maps port FQN → parent element FQN (for relation lifting)
    pub port_parents: HashMap<String, String>,
    /// Maps port FQN -> parser port type (`port` / `portin` / `portout`)
    pub port_types: HashMap<String, PortType>,
    pending_relations: Vec<PendingRelation>,
}

impl ComponentResolver {
    pub fn new() -> Self {
        Self {
            scope: Vec::new(),
            elements: HashMap::new(),
            child_elements_by_parent: HashMap::new(),
            port_parents: HashMap::new(),
            port_types: HashMap::new(),
            pending_relations: Vec::new(),
        }
    }

    fn port_type_to_role(port_type: PortType) -> EndpointRole {
        match port_type {
            PortType::Port => EndpointRole::None,
            PortType::PortIn => EndpointRole::Required,
            PortType::PortOut => EndpointRole::Provided,
        }
    }

    /// Collect all matching port FQNs by local/simple port name within `scope`
    /// and all descendant scopes.
    ///
    /// Notes:
    /// - `port_local` is treated as a single-segment name (no dot path parsing here).
    /// - Direct candidate `<scope>.<port_local>` is checked first.
    /// - Descendant matches are included when their parent component is inside scope.
    /// - Return value is deterministic and deduplicated (sorted via `BTreeSet`).
    fn collect_matching_port_fqns_in_scope_or_children(
        &self,
        scope: &[String],
        port_local: &str,
    ) -> Vec<String> {
        let mut matches = BTreeSet::new();

        // 1. Direct candidate: scope + port_local
        let mut candidate = scope.to_vec();
        candidate.push(port_local.to_string());

        let direct_fqn = candidate.join(".");
        if self.port_parents.contains_key(&direct_fqn) {
            return vec![direct_fqn];
        }

        // 2. scope prefix
        // Search at any depth below the current scope: a port whose simple alias matches
        // and whose parent element is a descendant of (or equal to) the current scope.
        let scope_prefix = scope.join(".");
        for (pfqn, parent_comp) in &self.port_parents {
            let pfqn_last = match pfqn.rfind('.') {
                Some(i) => &pfqn[i + 1..],
                None => pfqn,
            };
            if pfqn_last != port_local {
                continue;
            }

            if scope.is_empty()
                || parent_comp == &scope_prefix
                || parent_comp.starts_with(&format!("{scope_prefix}."))
            {
                matches.insert(pfqn.clone());
            }
        }

        matches.into_iter().collect()
    }

    /// Collect all element FQN candidates for `parts` (single or multi segment path)
    /// under `scope` and all descendant scopes.
    ///
    /// Behavior:
    /// - Builds candidate `<scope>.<parts...>` and checks exact existence.
    /// - Recursively descends into child elements and repeats the same lookup.
    /// - Returns all matches (not first-hit), deduplicated and deterministically ordered.
    fn collect_element_fqns_in_scope_or_children(
        &self,
        scope: &[String],
        parts: &[&str],
    ) -> Vec<String> {
        let mut found = BTreeSet::new();

        self.collect_element_fqns_rec(scope, parts, &mut found);

        found.into_iter().collect()
    }

    fn collect_element_fqns_rec(
        &self,
        scope: &[String],
        parts: &[&str],
        found: &mut BTreeSet<String>,
    ) {
        // 1. direct FQN check
        let fqn = if scope.is_empty() {
            parts.join(".")
        } else {
            format!("{}.{}", scope.join("."), parts.join("."))
        };

        if self.elements.contains_key(&fqn) {
            found.insert(fqn);
        }

        // 2. find children
        let scope_key = if scope.is_empty() {
            None
        } else {
            Some(scope.join("."))
        };

        let children: &[String] = self
            .child_elements_by_parent
            .get(&scope_key)
            .map(Vec::as_slice)
            .unwrap_or(&[]);

        for child_id in children {
            let Some(element) = self.elements.get(child_id) else {
                continue;
            };

            let Some(name) = element.alias.as_deref().or(element.name.as_deref()) else {
                continue;
            };

            let mut child_scope = scope.to_vec();
            child_scope.push(name.to_string());

            self.collect_element_fqns_rec(&child_scope, parts, found);
        }
    }

    /// Collect possible port FQN candidates from a raw reference token.
    /// Returns deduplicated, deterministic candidates (sorted by `BTreeSet`).
    fn collect_port_fqn_candidates_for_raw(&self, raw: &str) -> Vec<String> {
        let parts: Vec<&str> = raw.split('.').collect();
        let mut candidates = std::collections::BTreeSet::new();

        // 1. scope-based lookup (only for simple name)
        if parts.len() == 1 {
            for i in (0..=self.scope.len()).rev() {
                let outer_scope = &self.scope[..i];
                let matches =
                    self.collect_matching_port_fqns_in_scope_or_children(outer_scope, raw);
                if !matches.is_empty() {
                    // Keep nearest-scope matches only for simple names.
                    candidates.extend(matches);
                    break;
                }
            }
        }

        // 2. relative FQN (scope + parts)
        let mut relative = self.scope.clone();
        relative.extend(parts.iter().map(|p| p.to_string()));

        let relative_port_fqn = relative.join(".");
        if self.port_types.contains_key(&relative_port_fqn) {
            candidates.insert(relative_port_fqn);
        }

        // 3. direct match
        if self.port_types.contains_key(raw) {
            candidates.insert(raw.to_string());
        }

        candidates.into_iter().collect()
    }

    /// Resolve a port role hint for `raw` reference, but only when port candidates are
    /// consistent with the already resolved endpoint identity (`resolved`).
    ///
    /// This prevents using an unrelated same-name port as role hint.
    ///
    /// Returns:
    /// - `Ok(None)`: no usable/aligned port hint.
    /// - `Ok(Some(role))`: exactly one aligned port candidate.
    /// - `Err(AmbiguousReference)`: multiple aligned port candidates.
    fn resolve_port_role_hint_for_ref(
        &self,
        raw: &str,
        resolved: &str,
    ) -> Result<Option<EndpointRole>, ComponentResolverError> {
        let candidates = self.collect_port_fqn_candidates_for_raw(raw);

        let mut matched: Option<&String> = None;

        for pfqn in &candidates {
            let ok = pfqn == resolved
                || self
                    .port_parents
                    .get(pfqn)
                    .map(|p| p == resolved)
                    .unwrap_or(false);

            if ok {
                // Invariant: after aligning candidates to resolved endpoint identity, there should be at most one match.
                // If multiple matches remain, this indicates inconsistent resolver state or unexpected duplicate-alignment; fail fast with AmbiguousReference instead of silently picking one.
                if matched.is_some() {
                    return Err(ComponentResolverError::AmbiguousReference {
                        reference: raw.to_string(),
                        candidates,
                    });
                }
                matched = Some(pfqn);
            }
        }

        let pfqn = match matched {
            Some(v) => v,
            None => return Ok(None),
        };

        Ok(self
            .port_types
            .get(pfqn)
            .copied()
            .map(Self::port_type_to_role))
    }

    fn make_fqn(&self, local: &str) -> String {
        if self.scope.is_empty() {
            local.to_string()
        } else {
            format!("{}.{}", self.scope.join("."), local)
        }
    }

    /// Resolve relation references, supporting:
    /// 1) Simple name: search upward from current scope + recurse into children
    /// 2) Relative qualified name: path starting from current scope
    /// 3) Absolute FQN: full path
    ///
    /// Ambiguity handling:
    /// - If any stage returns multiple valid candidates, return
    ///   `ComponentResolverError::AmbiguousReference` immediately.
    pub fn resolve_ref(&self, raw: &str) -> Result<String, ComponentResolverError> {
        let parts: Vec<&str> = raw.split('.').collect();

        // 1. simple name
        if parts.len() == 1 {
            if let Some(res) = self.resolve_simple_name(parts[0], raw)? {
                return Ok(res);
            }
        }

        // 2. relative qualified name
        if let Some(res) = self.resolve_relative(&parts)? {
            return Ok(res);
        }

        // 3. absolute FQN
        let fqn = parts.join(".");
        if self.elements.contains_key(&fqn) {
            return Ok(fqn);
        }

        error!("Unresolved reference: {}", raw);
        Err(ComponentResolverError::UnresolvedReference {
            reference: raw.to_string(),
        })
    }

    fn resolve_simple_name(
        &self,
        name: &str,
        raw: &str,
    ) -> Result<Option<String>, ComponentResolverError> {
        // 1) lexical element lookup
        if let Some(res) = self.walk_scopes_nearest_first(|scope| {
            let matches = self.collect_element_fqns_in_scope_or_children(scope, &[name]);
            Self::pick_unique(matches, raw)
        })? {
            return Ok(Some(res));
        }

        // 2) lexical port lookup (collapsed to parent component)
        if let Some(res) = self.walk_scopes_nearest_first(|scope| {
            let ports = self.collect_matching_port_fqns_in_scope_or_children(scope, name);

            if ports.is_empty() {
                return Ok(None);
            }

            let parents: Vec<String> = ports
                .iter()
                .filter_map(|p| self.port_parents.get(p))
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();

            Self::pick_unique(parents, raw)
        })? {
            return Ok(Some(res));
        }

        // 3) global alias fallback
        let global: Vec<String> = self
            .elements
            .values()
            .filter(|e| e.alias.as_deref() == Some(name) || e.name.as_deref() == Some(name))
            .map(|e| e.id.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();

        Self::pick_unique(global, raw)
    }

    fn resolve_relative(&self, parts: &[&str]) -> Result<Option<String>, ComponentResolverError> {
        let matches = self.collect_element_fqns_in_scope_or_children(&self.scope, parts);

        Self::pick_unique(matches, &parts.join("."))
    }

    fn walk_scopes_nearest_first<F>(
        &self,
        mut f: F,
    ) -> Result<Option<String>, ComponentResolverError>
    where
        F: FnMut(&[String]) -> Result<Option<String>, ComponentResolverError>,
    {
        for i in (0..=self.scope.len()).rev() {
            let scope = &self.scope[..i];
            if let Some(res) = f(scope)? {
                return Ok(Some(res));
            }
        }
        Ok(None)
    }

    fn pick_unique(
        mut matches: Vec<String>,
        raw: &str,
    ) -> Result<Option<String>, ComponentResolverError> {
        if matches.is_empty() {
            return Ok(None);
        }

        if matches.len() == 1 {
            return Ok(Some(matches.remove(0)));
        }

        matches.sort();
        Err(ComponentResolverError::AmbiguousReference {
            reference: raw.to_string(),
            candidates: matches,
        })
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
    // - Interface binding: `)-`, `-(`
    // - Directed: `-->`, `<--`, `..>`, `<..`
    // - Undirected: `--`, `..`
    fn parse_arrow(relation: &Relation) -> Result<ArrowAnalysis, ComponentResolverError> {
        let (left, right, middle) = Self::arrow_parts(&relation.arrow);
        let line = relation.arrow.line.raw.as_str();

        let has_provided_token = left == ")";
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

        let decor_role = if is_lollipop_line && left == ")" && middle.is_empty() && right.is_empty()
        {
            Some(EndpointRole::Provided)
        } else if is_lollipop_line
            && left.is_empty()
            && ((middle == "(" && right.is_empty()) || (middle.is_empty() && right == "("))
        {
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
        let resolved = self.resolve_ref(raw)?;
        let port_role_hint = self.resolve_port_role_hint_for_ref(raw, &resolved)?;

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
                reason: "Interface decorators '-(' and ')-' require exactly one Interface endpoint"
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
                reason: "Interface decorators '-(' and ')-' are not allowed between two interfaces"
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
                reason: "Unsupported interface decorator syntax: only ')-' (Provided) and '-(' (Required) are supported"
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
        self.scope.clear();
        self.elements.clear();
        self.child_elements_by_parent.clear();
        self.port_parents.clear();
        self.port_types.clear();
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
        let fqn = self.make_fqn(local_id);

        if self.scope.is_empty() {
            // Top-level ports are pure connectors/aliases, not entities — ignore them.
            // Use `interface` to declare a top-level interface as a first-class entity.
        } else {
            // Nested port: record port_fqn -> parent_fqn for relation lifting.
            self.port_types.insert(fqn.clone(), port.port_type);
            self.port_parents.insert(fqn, self.scope.join("."));
        }
    }

    fn visit_element(&mut self, element: &Element) -> Result<(), ComponentResolverError> {
        let local_id = element
            .alias
            .as_deref()
            .or(element.name.as_deref())
            .expect("Element must have name or alias (guaranteed by grammar)");

        let fqn = self.make_fqn(local_id);
        if self.elements.contains_key(&fqn) {
            return Err(ComponentResolverError::DuplicateElement { element_id: fqn });
        }

        let parent_id = if self.scope.is_empty() {
            None
        } else {
            Some(self.scope.join("."))
        };

        let logic = LogicComponent {
            id: fqn.clone(),
            name: element.name.clone(),
            alias: element.alias.clone(),
            source_location: element.source_location.clone(),
            parent_id: parent_id.clone(),
            element_type: parse_kind(&element.kind)?,
            stereotype: element.stereotype.clone(),
            relations: Vec::new(),
        };

        self.elements.insert(fqn.clone(), logic);

        self.child_elements_by_parent
            .entry(parent_id.clone())
            .or_default()
            .push(fqn.clone());

        self.scope.push(local_id.to_string());

        for stmt in &element.statements {
            self.visit_statement(stmt)?;
        }

        self.scope.pop();

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
