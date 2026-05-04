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

use crate::component_logic::{
    ComponentResolverError, ComponentType, LogicComponent, LogicRelation,
};
use component_parser::{CompPumlDocument, Component, Port, Statement};
use resolver_traits::DiagramResolver;

#[derive(Default)]
pub struct ComponentResolver {
    pub scope: Vec<String>,                          // component id stack
    pub components: HashMap<String, LogicComponent>, // FQN -> LogicComponent
    /// Maps port FQN → parent component FQN (for relation lifting)
    pub port_parents: HashMap<String, String>,
}

impl ComponentResolver {
    pub fn new() -> Self {
        Self {
            scope: Vec::new(),
            components: HashMap::new(),
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
    fn resolve_ref(&self, raw: &str) -> Result<String, ComponentResolverError> {
        let parts: Vec<&str> = raw.split('.').collect();

        // Helper: recursively search for a component FQN within the given scope and its children
        fn find_in_scope_or_children(
            scope: &[String],
            parts: &[&str],
            components: &HashMap<String, LogicComponent>,
        ) -> Option<String> {
            let mut candidate = scope.to_vec();
            candidate.extend(parts.iter().map(|s| s.to_string()));
            let fqn = candidate.join(".");
            if components.contains_key(&fqn) {
                return Some(fqn);
            }

            for comp in components.values() {
                if let Some(parent) = &comp.parent_id {
                    if parent == &scope.join(".") {
                        let mut child_scope = scope.to_vec();
                        child_scope.push(comp.alias.clone().unwrap_or(comp.name.clone().unwrap()));
                        if let Some(f) = find_in_scope_or_children(&child_scope, parts, components)
                        {
                            return Some(f);
                        }
                    }
                }
            }

            None
        }

        // Helper: search for a port by local name within the given scope and any of its
        // descendants, returning the port's parent component FQN when found.
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
            // and whose parent component is a descendant of (or equal to) the current scope.
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
                if let Some(fqn) = find_in_scope_or_children(outer_scope, &parts, &self.components)
                {
                    return Ok(fqn);
                }
            }
            for comp in self.components.values() {
                if comp.alias.as_deref() == Some(parts[0]) || comp.name.as_deref() == Some(parts[0])
                {
                    return Ok(comp.id.clone());
                }
            }
            // Fallback: check if it's a port name and lift to parent component.
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
        if let Some(fqn) = find_in_scope_or_children(&self.scope, &parts, &self.components) {
            return Ok(fqn);
        }

        // 3) Absolute FQN
        let fqn = parts.join(".");
        if self.components.contains_key(&fqn) {
            return Ok(fqn);
        }

        error!("Unresolved reference: {}", raw);
        Err(ComponentResolverError::UnresolvedReference {
            reference: raw.to_string(),
        })
    }
}

impl DiagramResolver for ComponentResolver {
    type Document = CompPumlDocument;
    type Output = HashMap<String, LogicComponent>;
    type Error = ComponentResolverError;

    fn resolve(&mut self, document: &CompPumlDocument) -> Result<Self::Output, Self::Error> {
        self.scope.clear();

        for stmt in &document.statements {
            self.visit_statement(stmt)?;
        }

        // Post-pass: lift port references to parent component
        self.lift_port_relations();

        Ok(self.components.clone())
    }
}

impl ComponentResolver {
    fn visit_statement(&mut self, statement: &Statement) -> Result<(), ComponentResolverError> {
        match statement {
            Statement::Component(component) => {
                self.visit_component(component)?;
                Ok(())
            }
            Statement::Port(port) => {
                self.visit_port(port);
                Ok(())
            }
            Statement::Relation(relation) => {
                let src_fqn = self.resolve_ref(&relation.lhs)?;
                let tgt_fqn = self.resolve_ref(&relation.rhs)?;

                if let Some(source_component) = self.components.get_mut(&src_fqn) {
                    source_component.relations.push(LogicRelation {
                        target: tgt_fqn,
                        annotation: relation.description.clone(),
                        relation_type: "None".to_string(), // Placeholder, can be enhanced to capture relation type from arrow
                    });
                    Ok(())
                } else {
                    Err(ComponentResolverError::UnresolvedReference { reference: src_fqn })
                }
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
            self.port_parents.insert(fqn, self.scope.join("."));
        }
    }

    /// After all statements are visited, replace any relation endpoint that is a
    /// port FQN with the port's parent component FQN.
    fn lift_port_relations(&mut self) {
        let port_parents = self.port_parents.clone();

        for comp in self.components.values_mut() {
            for rel in comp.relations.iter_mut() {
                if let Some(parent) = port_parents.get(&rel.target) {
                    rel.target = parent.clone();
                }
            }
        }
    }

    fn visit_component(&mut self, component: &Component) -> Result<(), ComponentResolverError> {
        let local_id = component
            .alias
            .as_deref()
            .or(component.name.as_deref())
            .expect("Component must have name or alias (guaranteed by grammar)");

        let fqn = self.make_fqn(local_id);
        if self.components.contains_key(&fqn) {
            return Err(ComponentResolverError::DuplicateComponent { component_id: fqn });
        }

        let parent_id = if self.scope.is_empty() {
            None
        } else {
            Some(self.scope.join("."))
        };

        let logic = LogicComponent {
            id: fqn.clone(),
            name: component.name.clone(),
            alias: component.alias.clone(),
            parent_id,
            comp_type: parse_component_type(&component.component_type)?,
            stereotype: component.stereotype.clone(),
            relations: Vec::new(),
        };

        self.components.insert(fqn.clone(), logic);

        self.scope.push(local_id.to_string());

        for stmt in &component.statements {
            self.visit_statement(stmt)?;
        }

        self.scope.pop();

        Ok(())
    }
}

const COMPONENT_TYPE_TABLE: &[(&str, ComponentType)] = &[
    ("artifact", ComponentType::Artifact),
    ("card", ComponentType::Card),
    ("cloud", ComponentType::Cloud),
    ("component", ComponentType::Component),
    ("database", ComponentType::Database),
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
];

pub fn parse_component_type(raw: &str) -> Result<ComponentType, ComponentResolverError> {
    COMPONENT_TYPE_TABLE
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(raw))
        .map(|(_, v)| *v)
        .ok_or_else(|| ComponentResolverError::UnknownComponentType {
            component_type: raw.into(),
        })
}
