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

use crate::component_logic::{ElementResolverError, ElementType, LogicElement, LogicRelation};
use component_parser::{CompPumlDocument, Element, Port, Statement};
use resolver_traits::DiagramResolver;

#[derive(Default)]
pub struct ElementResolver {
    pub scope: Vec<String>,                      // element id stack
    pub elements: HashMap<String, LogicElement>, // FQN -> LogicElement
    /// Maps port FQN → parent element FQN (for relation lifting)
    pub port_parents: HashMap<String, String>,
}

impl ElementResolver {
    pub fn new() -> Self {
        Self {
            scope: Vec::new(),
            elements: HashMap::new(),
            port_parents: HashMap::new(),
        }
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
    fn resolve_ref(&self, raw: &str) -> Result<String, ElementResolverError> {
        let parts: Vec<&str> = raw.split('.').collect();

        // Helper: recursively search for an element FQN within the given scope and its children
        fn find_in_scope_or_children(
            scope: &[String],
            parts: &[&str],
            elements: &HashMap<String, LogicElement>,
        ) -> Option<String> {
            let mut candidate = scope.to_vec();
            candidate.extend(parts.iter().map(|s| s.to_string()));
            let fqn = candidate.join(".");
            if elements.contains_key(&fqn) {
                return Some(fqn);
            }

            for element in elements.values() {
                if let Some(parent) = &element.parent_id {
                    if parent == &scope.join(".") {
                        let mut child_scope = scope.to_vec();
                        child_scope.push(
                            element
                                .alias
                                .clone()
                                .unwrap_or(element.name.clone().unwrap()),
                        );
                        if let Some(f) = find_in_scope_or_children(&child_scope, parts, elements) {
                            return Some(f);
                        }
                    }
                }
            }

            None
        }

        // Helper: search for a port by local name within the given scope and any of its
        // descendants, returning the port's parent element FQN when found.
        fn find_port_in_scope_or_children(
            scope: &[String],
            port_local: &str,
            port_parents: &HashMap<String, String>,
        ) -> Option<String> {
            // Direct candidate: scope + port_local
            let mut candidate = scope.to_vec();
            candidate.push(port_local.to_string());
            let port_fqn = candidate.join(".");
            if let Some(parent_fqn) = port_parents.get(&port_fqn) {
                return Some(parent_fqn.clone());
            }

            // Search at any depth below the current scope: a port whose simple alias matches
            // and whose parent element is a descendant of (or equal to) the current scope.
            let scope_prefix = scope.join(".");
            for (pfqn, parent_comp) in port_parents {
                let parts: Vec<&str> = pfqn.split('.').collect();
                if parts.last() != Some(&port_local) {
                    continue;
                }
                let is_in_scope = scope.is_empty()
                    || parent_comp == &scope_prefix
                    || parent_comp.starts_with(&format!("{scope_prefix}."));
                if is_in_scope {
                    return Some(parent_comp.clone());
                }
            }

            None
        }

        // 1) Simple name: search upward from current scope
        if parts.len() == 1 {
            for i in (0..=self.scope.len()).rev() {
                let outer_scope = &self.scope[..i];
                if let Some(fqn) = find_in_scope_or_children(outer_scope, &parts, &self.elements) {
                    return Ok(fqn);
                }
            }
            for element in self.elements.values() {
                if element.alias.as_deref() == Some(parts[0])
                    || element.name.as_deref() == Some(parts[0])
                {
                    return Ok(element.id.clone());
                }
            }
            // Fallback: check if it's a port name and lift to the parent element.
            // Search upward through scope levels — the innermost scope that contains a
            // port with this alias wins (nearest-scope-first).
            for i in (0..=self.scope.len()).rev() {
                let outer_scope = &self.scope[..i];
                if let Some(parent_fqn) =
                    find_port_in_scope_or_children(outer_scope, raw, &self.port_parents)
                {
                    return Ok(parent_fqn);
                }
            }
        }

        // 2) Relative qualified name + recurse into children
        if let Some(fqn) = find_in_scope_or_children(&self.scope, &parts, &self.elements) {
            return Ok(fqn);
        }

        // 3) Absolute FQN
        let fqn = parts.join(".");
        if self.elements.contains_key(&fqn) {
            return Ok(fqn);
        }

        error!("Unresolved reference: {}", raw);
        Err(ElementResolverError::UnresolvedReference {
            reference: raw.to_string(),
        })
    }
}

impl DiagramResolver for ElementResolver {
    type Document = CompPumlDocument;
    type Output = HashMap<String, LogicElement>;
    type Error = ElementResolverError;

    fn resolve(&mut self, document: &CompPumlDocument) -> Result<Self::Output, Self::Error> {
        self.scope.clear();

        for stmt in &document.statements {
            self.visit_statement(stmt)?;
        }

        // Post-pass: lift port references to their parent element
        self.lift_port_relations();

        Ok(self.elements.clone())
    }
}

impl ElementResolver {
    fn visit_statement(&mut self, statement: &Statement) -> Result<(), ElementResolverError> {
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
                let src_fqn = self.resolve_ref(&relation.lhs)?;
                let tgt_fqn = self.resolve_ref(&relation.rhs)?;

                if let Some(source_element) = self.elements.get_mut(&src_fqn) {
                    source_element.relations.push(LogicRelation {
                        target: tgt_fqn,
                        annotation: relation.description.clone(),
                        relation_type: "None".to_string(), // Placeholder, can be enhanced to capture relation type from arrow
                    });
                    Ok(())
                } else {
                    Err(ElementResolverError::UnresolvedReference { reference: src_fqn })
                }
            }
        }
    }
}

impl ElementResolver {
    fn visit_port(&mut self, port: &Port) {
        let local_id = port.alias.as_deref().unwrap_or(&port.name);
        let fqn = self.make_fqn(local_id);

        if self.scope.is_empty() {
            // Top-level ports are pure connectors/aliases, not entities — ignore them.
            // Use `interface` to declare a top-level interface as a first-class entity.
        } else {
            // Nested port: record port_fqn -> parent_fqn for relation lifting.
            self.port_parents.insert(fqn, self.scope.join("."));
        }
    }

    /// After all statements are visited, replace any relation endpoint that is a
    /// port FQN with the port's parent element FQN.
    fn lift_port_relations(&mut self) {
        let port_parents = self.port_parents.clone();

        for element in self.elements.values_mut() {
            for rel in element.relations.iter_mut() {
                if let Some(parent) = port_parents.get(&rel.target) {
                    rel.target = parent.clone();
                }
            }
        }
    }

    fn visit_element(&mut self, element: &Element) -> Result<(), ElementResolverError> {
        let local_id = element
            .alias
            .as_deref()
            .or(element.name.as_deref())
            .expect("Element must have name or alias (guaranteed by grammar)");

        let fqn = self.make_fqn(local_id);
        if self.elements.contains_key(&fqn) {
            return Err(ElementResolverError::DuplicateElement { element_id: fqn });
        }

        let parent_id = if self.scope.is_empty() {
            None
        } else {
            Some(self.scope.join("."))
        };

        let logic = LogicElement {
            id: fqn.clone(),
            name: element.name.clone(),
            alias: element.alias.clone(),
            parent_id,
            element_type: parse_kind(&element.kind)?,
            stereotype: element.stereotype.clone(),
            relations: Vec::new(),
        };

        self.elements.insert(fqn.clone(), logic);

        self.scope.push(local_id.to_string());

        for stmt in &element.statements {
            self.visit_statement(stmt)?;
        }

        self.scope.pop();

        Ok(())
    }
}

const ELEMENT_TYPE_TABLE: &[(&str, ElementType)] = &[
    ("artifact", ElementType::Artifact),
    ("actor", ElementType::Actor),
    ("agent", ElementType::Agent),
    ("boundary", ElementType::Boundary),
    ("card", ElementType::Card),
    ("cloud", ElementType::Cloud),
    ("component", ElementType::Component),
    ("control", ElementType::Control),
    ("database", ElementType::Database),
    ("entity", ElementType::Entity),
    ("file", ElementType::File),
    ("folder", ElementType::Folder),
    ("frame", ElementType::Frame),
    ("hexagon", ElementType::Hexagon),
    ("interface", ElementType::Interface),
    ("node", ElementType::Node),
    ("package", ElementType::Package),
    ("queue", ElementType::Queue),
    ("rectangle", ElementType::Rectangle),
    ("stack", ElementType::Stack),
    ("storage", ElementType::Storage),
    ("usecase", ElementType::Usecase),
];

pub fn parse_kind(raw: &str) -> Result<ElementType, ElementResolverError> {
    ELEMENT_TYPE_TABLE
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(raw))
        .map(|(_, v)| *v)
        .ok_or_else(|| ElementResolverError::UnknownElementType {
            element_type: raw.into(),
        })
}

pub type ComponentResolver = ElementResolver;
