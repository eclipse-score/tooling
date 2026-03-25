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
use component_parser::{CompPumlDocument, Component, Statement};
use resolver_traits::DiagramResolver;

pub struct ComponentResolver {
    pub scope: Vec<String>,                          // component id stack
    pub components: HashMap<String, LogicComponent>, // FQN -> LogicComponent
}

impl Default for ComponentResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentResolver {
    pub fn new() -> Self {
        Self {
            scope: Vec::new(),
            components: HashMap::new(),
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
    type Statement = Statement;
    type Output = HashMap<String, LogicComponent>;
    type Error = ComponentResolverError;

    fn visit_document(&mut self, document: &CompPumlDocument) -> Result<Self::Output, Self::Error> {
        self.scope.clear();

        for stmt in &document.statements {
            self.visit_statement(stmt)?;
        }

        Ok(self.components.clone())
    }

    fn visit_statement(&mut self, statement: &Statement) -> Result<(), Self::Error> {
        match statement {
            Statement::Component(component) => {
                self.visit_component(component)?;
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
