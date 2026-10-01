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
use std::collections::HashMap;

use class_diagram::Visibility as ResolverVisibility;
use class_diagram::*;
use class_parser::Visibility as ParserVisibility;
use class_parser::{
    Attribute, ClassUmlFile, ClassUmlTopLevel, Element, EnumDef, EnumValue, Method as ParserMethod,
    Name, Namespace, Package, Param as ParserParam, Relationship as ParserRelationship,
    TypeAlias as ParserTypeAlias,
};
use parser_core::common_ast::Arrow;
use resolver_traits::DiagramResolver;
use thiserror::Error;
use uid_normalization::{leaf_key, InternalScope, Resolution, RootAnchor};

#[derive(Debug, Error)]
pub enum ClassPumlResolverError {
    #[error("Class Resolver: Unresolved reference: {reference}")]
    UnresolvedReference { reference: String },

    #[error("Duplicate entity id: {entity_id} (line {line})", line = source_location.line)]
    DuplicateEntity {
        entity_id: String,
        source_location: SourceLocation,
    },

    #[error("Ambiguous reference: {reference} -> {candidates}", candidates = candidates.join(", "))]
    AmbiguousReference {
        reference: String,
        candidates: Vec<String>,
    },

    #[error("Unknown entity type: {entity_type}")]
    UnknownEntityType { entity_type: String },

    #[error("Invalid relationship: {from} -> {to}: {reason}")]
    InvalidRelationship {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Circular inheritance detected: {cycle}")]
    CircularInheritance { cycle: String },

    #[error("Invalid visibility modifier: {modifier}")]
    InvalidVisibility { modifier: String },

    #[error("Parse error: {message}")]
    ParseError { message: String },
}

pub struct ClassResolver {
    pub logic: ClassDiagram,
    root_anchor: RootAnchor,
    // entity leaf (Rule A, last segment) -> every id registered under that leaf
    name_map: HashMap<String, Vec<String>>,
    // entity id -> its declaration line; also the existence check
    declared_at: HashMap<String, u32>,
}

impl Default for ClassResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassResolver {
    pub fn new() -> Self {
        Self::with_root_anchor(None)
    }

    /// Resolver whose ids are prefixed with `root_anchor`.
    pub fn with_root_anchor(root_anchor: Option<&str>) -> Self {
        Self {
            logic: ClassDiagram {
                name: String::new(),
                entities: Vec::new(),
                free_functions: Vec::new(),
            },
            root_anchor: RootAnchor::new(root_anchor),
            name_map: HashMap::new(),
            declared_at: HashMap::new(),
        }
    }

    fn analyze(&mut self, file: &ClassUmlFile) -> Result<(), ClassPumlResolverError> {
        let root = InternalScope::default();

        for elem in &file.elements {
            self.process_top_level(elem, &root)?;
        }

        for rel in &file.relationships {
            self.process_relationship(rel, &root)?;
        }

        for elem in &file.elements {
            self.process_declared_relations_top_level(elem, &root)?;
        }

        Ok(())
    }

    pub fn result(self) -> ClassDiagram {
        self.logic
    }

    fn map_visibility(v: ParserVisibility) -> ResolverVisibility {
        match v {
            ParserVisibility::Public => ResolverVisibility::Public,
            ParserVisibility::Private => ResolverVisibility::Private,
            ParserVisibility::Protected => ResolverVisibility::Protected,
            ParserVisibility::Package => ResolverVisibility::Private,
        }
    }

    /// Rule A: the id leaf is the alias when present, else the name.
    fn id_leaf(name: &Name) -> &str {
        name.display.as_deref().unwrap_or(&name.internal)
    }

    /// Nesting scope path, without root anchor.
    fn enclosing_namespace_id(scope: &InternalScope) -> Option<String> {
        (!scope.is_empty()).then(|| scope.resolve(&RootAnchor::default()))
    }

    fn resolve_entity_id(&self, leaf: &str, scope: &InternalScope) -> String {
        scope.resolve_with_leaf(&self.root_anchor, leaf)
    }

    fn declared_before(&self, id: &str, reference_line: u32) -> bool {
        self.declared_at
            .get(id)
            .is_some_and(|&line| line < reference_line)
    }

    fn leaf_candidates(&self, leaf: &str, reference_line: u32) -> Vec<String> {
        self.name_map
            .get(leaf)
            .into_iter()
            .flatten()
            .filter(|id| self.declared_before(id, reference_line))
            .cloned()
            .collect()
    }

    fn push_entity(
        &mut self,
        entity: SimpleEntity,
        leaf: &str,
    ) -> Result<(), ClassPumlResolverError> {
        if self.declared_at.contains_key(&entity.id) {
            return Err(ClassPumlResolverError::DuplicateEntity {
                entity_id: entity.id,
                source_location: entity.source_location,
            });
        }

        self.declared_at
            .insert(entity.id.clone(), entity.source_location.line);

        self.name_map
            .entry(leaf_key(leaf))
            .or_default()
            .push(entity.id.clone());
        self.logic.entities.push(entity);
        Ok(())
    }

    /// Rule C lookup; sees entities declared earlier, plus the scope-local hit.
    fn resolve_reference(
        &self,
        raw: &str,
        scope: &InternalScope,
        reference_line: u32,
    ) -> Result<String, ClassPumlResolverError> {
        let resolution = uid_normalization::resolve_reference(
            scope,
            &self.root_anchor,
            raw,
            |id| self.declared_at.contains_key(id),
            |id| self.declared_before(id, reference_line),
            |leaf| self.leaf_candidates(leaf, reference_line),
        );

        match resolution {
            Resolution::Resolved(id) => Ok(id),
            Resolution::Unresolved => Err(ClassPumlResolverError::UnresolvedReference {
                reference: raw.to_string(),
            }),
            Resolution::Ambiguous(candidates) => Err(ClassPumlResolverError::AmbiguousReference {
                reference: raw.to_string(),
                candidates,
            }),
        }
    }

    fn process_top_level(
        &mut self,
        elem: &ClassUmlTopLevel,
        scope: &InternalScope,
    ) -> Result<(), ClassPumlResolverError> {
        match elem {
            ClassUmlTopLevel::Types(element) => {
                self.process_element(element, scope)?;
            }

            ClassUmlTopLevel::Enum(enum_def) => {
                self.process_enum(enum_def, scope)?;
            }

            ClassUmlTopLevel::Namespace(ns) => {
                self.process_namespace(ns, scope)?;
            }

            ClassUmlTopLevel::Package(pkg) => {
                self.process_package(pkg, scope)?;
            }
        }
        Ok(())
    }

    fn process_declared_relations_top_level(
        &mut self,
        elem: &ClassUmlTopLevel,
        scope: &InternalScope,
    ) -> Result<(), ClassPumlResolverError> {
        match elem {
            ClassUmlTopLevel::Types(element) => {
                self.process_declared_relations_element(element, scope)?;
            }
            ClassUmlTopLevel::Enum(_) => {}
            ClassUmlTopLevel::Namespace(ns) => {
                self.process_namespace_declared_relations(ns, scope)?;
            }
            ClassUmlTopLevel::Package(pkg) => {
                self.process_package_declared_relations(pkg, scope)?;
            }
        }

        Ok(())
    }

    fn process_package(
        &mut self,
        pkg: &Package,
        scope: &InternalScope,
    ) -> Result<(), ClassPumlResolverError> {
        let nested = scope.child(Self::id_leaf(&pkg.name));

        for t in &pkg.types {
            self.process_element(t, &nested)?;
        }

        for sub in &pkg.packages {
            self.process_package(sub, &nested)?;
        }

        Ok(())
    }

    fn process_namespace(
        &mut self,
        ns: &Namespace,
        scope: &InternalScope,
    ) -> Result<(), ClassPumlResolverError> {
        let nested = scope.child(Self::id_leaf(&ns.name));

        for t in &ns.types {
            self.process_element(t, &nested)?;
        }

        for sub in &ns.namespaces {
            self.process_namespace(sub, &nested)?;
        }

        Ok(())
    }

    /// Resolves nested relationships once all entities are registered.
    fn process_package_declared_relations(
        &mut self,
        pkg: &Package,
        scope: &InternalScope,
    ) -> Result<(), ClassPumlResolverError> {
        let nested = scope.child(Self::id_leaf(&pkg.name));

        for t in &pkg.types {
            self.process_declared_relations_element(t, &nested)?;
        }

        for rel in &pkg.relationships {
            self.process_relationship(rel, &nested)?;
        }

        for sub in &pkg.packages {
            self.process_package_declared_relations(sub, &nested)?;
        }

        Ok(())
    }

    fn process_namespace_declared_relations(
        &mut self,
        ns: &Namespace,
        scope: &InternalScope,
    ) -> Result<(), ClassPumlResolverError> {
        let nested = scope.child(Self::id_leaf(&ns.name));

        for t in &ns.types {
            self.process_declared_relations_element(t, &nested)?;
        }

        for rel in &ns.relationships {
            self.process_relationship(rel, &nested)?;
        }

        for sub in &ns.namespaces {
            self.process_namespace_declared_relations(sub, &nested)?;
        }

        Ok(())
    }

    fn process_declared_relations_element(
        &mut self,
        element: &Element,
        scope: &InternalScope,
    ) -> Result<(), ClassPumlResolverError> {
        match element {
            Element::ClassDef(def) => {
                self.process_declared_relationships(
                    Self::id_leaf(&def.name),
                    &def.extends,
                    scope,
                    RelationType::Inheritance,
                    &def.source_location,
                )?;
                self.process_declared_relationships(
                    Self::id_leaf(&def.name),
                    &def.implements,
                    scope,
                    RelationType::Implementation,
                    &def.source_location,
                )?;
            }
            Element::InterfaceDef(def) => {
                self.process_declared_relationships(
                    Self::id_leaf(&def.name),
                    &def.extends,
                    scope,
                    RelationType::Inheritance,
                    &def.source_location,
                )?;
            }
            _ => {}
        }

        Ok(())
    }

    fn process_declared_relationships(
        &mut self,
        source_leaf: &str,
        targets: &[String],
        scope: &InternalScope,
        relation_type: RelationType,
        source_location: &SourceLocation,
    ) -> Result<(), ClassPumlResolverError> {
        if targets.is_empty() {
            return Ok(());
        }

        let source = self.resolve_entity_id(source_leaf, scope);

        for declared_target in targets {
            let target = self.resolve_reference(declared_target, scope, source_location.line)?;

            self.add_relationship(Relationship {
                source: source.clone(),
                target,
                relation_type,
                source_multiplicity: None,
                target_multiplicity: None,
                source_location: source_location.clone(),
            })?;
        }

        Ok(())
    }

    fn process_element(
        &mut self,
        element: &Element,
        scope: &InternalScope,
    ) -> Result<(), ClassPumlResolverError> {
        match element {
            Element::EnumDef(def) => self.process_enum(def, scope),
            _ => {
                let entity_type = match element {
                    Element::ClassDef(def) if def.is_abstract => EntityType::AbstractClass,
                    Element::ClassDef(_) => EntityType::Class,
                    Element::StructDef(_) => EntityType::Struct,
                    Element::InterfaceDef(_) => EntityType::Interface,
                    _ => unreachable!(),
                };
                self.process_class(element, scope, entity_type)
            }
        }
    }

    fn process_class(
        &mut self,
        def: &Element,
        scope: &InternalScope,
        entity_type: EntityType,
    ) -> Result<(), ClassPumlResolverError> {
        let (
            name,
            stereotypes,
            attributes,
            type_aliases,
            methods,
            template_parameters,
            source_location,
        ) = match def {
            Element::ClassDef(c) => (
                &c.name,
                &c.stereotypes,
                &c.attributes,
                &c.type_aliases,
                &c.methods,
                &c.template_parameters,
                &c.source_location,
            ),
            Element::StructDef(s) => (
                &s.name,
                &s.stereotypes,
                &s.attributes,
                &s.type_aliases,
                &s.methods,
                &s.template_parameters,
                &s.source_location,
            ),
            Element::InterfaceDef(i) => (
                &i.name,
                &i.stereotypes,
                &i.attributes,
                &i.type_aliases,
                &i.methods,
                &i.template_parameters,
                &i.source_location,
            ),
            Element::EnumDef(_) => {
                unreachable!("EnumDef should not be passed to process_class")
            }
        };

        let leaf = Self::id_leaf(name);
        let id = self.resolve_entity_id(leaf, scope);
        let owner_name = leaf_key(leaf);

        let template_parameters =
            Self::convert_class_template_parameters(template_parameters, methods);

        let entity = SimpleEntity {
            id: id.clone(),
            name: leaf.to_string(),
            enclosing_namespace_id: Self::enclosing_namespace_id(scope),
            stereotypes: stereotypes.clone(),
            entity_type,
            type_aliases: type_aliases.iter().map(Self::convert_type_alias).collect(),
            variables: attributes.iter().map(Self::convert_variable).collect(),
            methods: methods
                .iter()
                .map(|method| Self::convert_method(method, &owner_name))
                .collect(),
            template_parameters,
            enum_literals: vec![],
            relationships: vec![],
            source_location: source_location.clone(),
        };

        self.push_entity(entity, leaf)
    }

    fn convert_type_alias(type_alias: &ParserTypeAlias) -> TypeAlias {
        TypeAlias {
            alias: type_alias.alias.clone(),
            original_type: type_alias.original_type.clone(),
            source_location: type_alias.source_location.clone(),
        }
    }

    fn convert_variable(attr: &Attribute) -> MemberVariable {
        fn has_modifier(modifiers: &[String], expected: &str) -> bool {
            modifiers
                .iter()
                .any(|modifier| ClassResolver::normalize_modifier(modifier) == expected)
        }

        MemberVariable {
            name: attr.name.clone(),
            data_type: attr.r#type.clone(),
            visibility: Self::map_visibility(attr.visibility.clone()),
            is_static: has_modifier(&attr.modifiers, "static"),
            source_location: attr.source_location.clone(),
        }
    }

    fn convert_method(m: &ParserMethod, owner_name: &str) -> Method {
        fn has_modifier(modifiers: &[String], expected: &str) -> bool {
            modifiers
                .iter()
                .any(|modifier| ClassResolver::normalize_modifier(modifier) == expected)
        }

        let is_constructor = m.name == owner_name;
        let is_destructor = m.name == format!("~{}", owner_name);

        let is_abstract = has_modifier(&m.modifiers, "abstract");
        let is_override = has_modifier(&m.modifiers, "override");
        let is_final = has_modifier(&m.modifiers, "final");
        // A method that is abstract or overriding a base method is implicitly virtual in C++.
        let is_virtual = has_modifier(&m.modifiers, "virtual") || is_abstract || is_override;

        Method {
            name: m.name.clone(),
            return_type: m.r#type.clone(),
            visibility: Self::map_visibility(m.visibility.clone()),
            parameters: m.params.iter().map(Self::convert_param).collect(),
            template_parameters: Self::convert_template_parameters_with_pack_expansions(
                &m.template_parameters,
                &m.params,
            ),
            modifiers: MethodModifier::from_conditions([
                (has_modifier(&m.modifiers, "static"), MethodModifier::Static),
                (is_virtual, MethodModifier::Virtual),
                (is_abstract, MethodModifier::Abstract),
                (is_override, MethodModifier::Override),
                (
                    has_modifier(&m.modifiers, "noexcept"),
                    MethodModifier::Noexcept,
                ),
                (is_constructor, MethodModifier::Constructor),
                (is_destructor, MethodModifier::Destructor),
                (is_final, MethodModifier::Final),
            ]),
            source_location: m.source_location.clone(),
        }
    }

    fn normalize_modifier(raw: &str) -> &str {
        raw.trim()
            .trim_start_matches("<<")
            .trim_end_matches(">>")
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim()
    }

    fn convert_template_parameters(
        parameters: &Option<Vec<String>>,
    ) -> Option<Vec<TemplateParameter>> {
        parameters.as_ref().map(|values| {
            values
                .iter()
                .map(|value| {
                    let trimmed = value.trim();
                    let (name, is_pack) = trimmed
                        .strip_suffix("...")
                        .map(|name| (name.trim(), true))
                        .unwrap_or((trimmed, false));

                    TemplateParameter::Type {
                        name: name.to_string(),
                        is_pack,
                    }
                })
                .collect()
        })
    }

    fn convert_class_template_parameters(
        parameters: &Option<Vec<String>>,
        methods: &[ParserMethod],
    ) -> Option<Vec<TemplateParameter>> {
        parameters.as_ref()?;

        let mut converted = Self::convert_template_parameters(parameters).unwrap_or_default();

        for method in methods {
            Self::append_pack_expansion_parameters(&mut converted, &method.params);
        }

        (!converted.is_empty()).then_some(converted)
    }

    fn convert_template_parameters_with_pack_expansions(
        parameters: &Option<Vec<String>>,
        params: &[ParserParam],
    ) -> Option<Vec<TemplateParameter>> {
        parameters.as_ref()?;

        let mut converted = Self::convert_template_parameters(parameters).unwrap_or_default();
        Self::append_pack_expansion_parameters(&mut converted, params);

        (!converted.is_empty()).then_some(converted)
    }

    fn append_pack_expansion_parameters(
        template_parameters: &mut Vec<TemplateParameter>,
        params: &[ParserParam],
    ) {
        for param in params.iter().filter(|param| param.is_pack_expansion) {
            let Some(pack_name) = param
                .param_type
                .as_deref()
                .and_then(Self::infer_pack_name_from_param_type)
            else {
                continue;
            };

            if !template_parameters
                .iter()
                .any(|parameter| parameter.name() == pack_name)
            {
                template_parameters.push(TemplateParameter::Type {
                    name: pack_name.to_string(),
                    is_pack: true,
                });
            }
        }
    }

    fn infer_pack_name_from_param_type(param_type: &str) -> Option<&str> {
        let trimmed = param_type.trim();
        let candidate = trimmed.strip_suffix("&&").unwrap_or(trimmed).trim();

        (!candidate.is_empty()).then_some(candidate)
    }

    fn convert_param(param: &ParserParam) -> FunctionArgument {
        FunctionArgument {
            name: param.name.clone().unwrap_or_default(),
            param_type: param.param_type.clone(),
            is_variadic: param.is_c_variadic,
            is_pack_expansion: param.is_pack_expansion,
        }
    }

    fn process_enum(
        &mut self,
        def: &EnumDef,
        scope: &InternalScope,
    ) -> Result<(), ClassPumlResolverError> {
        let leaf = Self::id_leaf(&def.name);
        let id = self.resolve_entity_id(leaf, scope);

        let mut last_value: Option<i128> = None;
        let literals = def
            .items
            .iter()
            .map(|item| {
                let value = match &item.value {
                    Some(EnumValue::Literal(v)) => v
                        .parse::<i128>()
                        .ok()
                        .or_else(|| last_value.map(|lv| lv + 1))
                        .or(Some(0)),
                    Some(EnumValue::Description(_)) | None => {
                        last_value.map(|lv| lv + 1).or(Some(0))
                    }
                };

                last_value = value;

                EnumLiteral {
                    name: item.name.clone(),
                    value,
                    source_location: item.source_location.clone(),
                }
            })
            .collect();

        let entity = SimpleEntity {
            id: id.clone(),
            name: leaf.to_string(),
            enclosing_namespace_id: Self::enclosing_namespace_id(scope),
            stereotypes: def.stereotypes.clone(),
            entity_type: EntityType::Enum,
            type_aliases: vec![],
            variables: vec![],
            methods: vec![],
            template_parameters: None,
            enum_literals: literals,
            relationships: vec![],
            source_location: def.source_location.clone(),
        };

        self.push_entity(entity, leaf)
    }

    fn convert_arrow(&self, arrow: &Arrow) -> Result<(RelationType, bool), ClassPumlResolverError> {
        let left = arrow.left.as_ref().map(|d| d.raw.as_str()).unwrap_or("");
        let line = arrow.line.raw.as_str();
        let right = arrow.right.as_ref().map(|d| d.raw.as_str()).unwrap_or("");

        // ---------------- Inheritance ----------------
        // A <|-- B   => B extends A  (reversed)
        if left == "<|" && line == "--" {
            return Ok((RelationType::Inheritance, true));
        }
        // A --|> B   => A extends B  (normal)
        if line == "--" && right == "|>" {
            return Ok((RelationType::Inheritance, false));
        }

        // ---------------- Implementation ----------------
        // A <|.. B   => B implements A (reversed)
        if left == "<|" && line == ".." {
            return Ok((RelationType::Implementation, true));
        }
        // A ..|> B   => A implements B (normal)
        if line == ".." && right == "|>" {
            return Ok((RelationType::Implementation, false));
        }

        // ---------------- Composition ----------------
        // *--   or   --*
        if left == "*" {
            return Ok((RelationType::Composition, true));
        }
        if right == "*" {
            return Ok((RelationType::Composition, false));
        }

        // ---------------- Aggregation ----------------
        if left == "o" {
            return Ok((RelationType::Aggregation, true));
        }
        if right == "o" {
            return Ok((RelationType::Aggregation, false));
        }

        // ---------------- Association ----------------
        if line == "-" && right == ">" {
            return Ok((RelationType::Association, false));
        }
        if left == "<" && line == "-" {
            return Ok((RelationType::Association, true));
        }
        if line == "--" && right == ">" {
            return Ok((RelationType::Association, false));
        }
        if left == "<" && line == "--" {
            return Ok((RelationType::Association, true));
        }

        // ---------------- Dependency ----------------
        if line == ".." && right == ">" {
            return Ok((RelationType::Dependency, false));
        }
        if left == "<" && line == ".." {
            return Ok((RelationType::Dependency, true));
        }

        Err(ClassPumlResolverError::InvalidRelationship {
            from: left.to_string(),
            to: right.to_string(),
            reason: format!("Unsupported arrow pattern: {}{}{}", left, line, right),
        })
    }

    fn process_relationship(
        &mut self,
        rel: &ParserRelationship,
        scope: &InternalScope,
    ) -> Result<(), ClassPumlResolverError> {
        let left = self.resolve_reference(&rel.left, scope, rel.source_location.line)?;
        let right = self.resolve_reference(&rel.right, scope, rel.source_location.line)?;

        let (relation_type, reversed) = self.convert_arrow(&rel.arrow)?;

        let (source_id, target_id) = if reversed {
            (right, left)
        } else {
            (left, right)
        };

        let (source_multiplicity, target_multiplicity) = if reversed {
            (
                rel.right_multiplicity.clone(),
                rel.left_multiplicity.clone(),
            )
        } else {
            (
                rel.left_multiplicity.clone(),
                rel.right_multiplicity.clone(),
            )
        };

        self.add_relationship(Relationship {
            source: source_id,
            target: target_id,
            relation_type,
            source_multiplicity,
            target_multiplicity,
            source_location: rel.source_location.clone(),
        })?;

        Ok(())
    }

    fn add_relationship(
        &mut self,
        relationship: Relationship,
    ) -> Result<(), ClassPumlResolverError> {
        let source_id = &relationship.source;
        let source_entity = self
            .logic
            .entities
            .iter_mut()
            .find(|entity| entity.id == *source_id)
            .ok_or_else(|| ClassPumlResolverError::UnresolvedReference {
                reference: source_id.clone(),
            })?;

        source_entity.relationships.push(relationship);
        Ok(())
    }
}

impl DiagramResolver for ClassResolver {
    type Document = ClassUmlFile;
    type Output = ClassDiagram;
    type Error = ClassPumlResolverError;

    fn resolve(&mut self, document: &Self::Document) -> Result<Self::Output, Self::Error> {
        self.name_map.clear();
        self.declared_at.clear();

        self.logic.name = document.name.clone();

        self.analyze(document)?;

        let logic_class = std::mem::replace(
            &mut self.logic,
            ClassDiagram {
                name: String::new(),
                entities: Vec::new(),
                free_functions: Vec::new(),
            },
        );

        Ok(logic_class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use class_parser::{ClassDef, EnumItem, Name};
    use parser_core::common_ast::{ArrowDecor, ArrowLine};

    // ----------------------------
    // whl Name / Class / Arrow
    // ----------------------------
    fn make_name(name: &str) -> Name {
        Name {
            internal: name.to_string(),
            display: None,
        }
    }

    fn make_class_at(name: &str, line: u32) -> Element {
        Element::ClassDef(ClassDef {
            name: make_name(name),
            namespace: "".to_string(),
            package: "".to_string(),
            stereotypes: Vec::new(),
            source_location: SourceLocation::new("test.puml", line),
            is_abstract: false,
            template_parameters: None,
            extends: vec![],
            implements: vec![],
            attributes: vec![],
            type_aliases: vec![],
            methods: vec![],
        })
    }

    fn make_class(name: &str) -> Element {
        make_class_at(name, 1)
    }

    fn make_aliased_class(internal: &str, alias: &str) -> Element {
        let mut element = make_class(internal);
        if let Element::ClassDef(def) = &mut element {
            def.name.display = Some(alias.to_string());
        }
        element
    }

    fn make_enum(name: &str, items: Vec<&str>) -> Element {
        Element::EnumDef(EnumDef {
            name: make_name(name),
            namespace: "".to_string(),
            package: "".to_string(),
            source_location: SourceLocation::new("test.puml", 1),
            stereotypes: Vec::new(),
            items: items
                .into_iter()
                .map(|n| EnumItem {
                    name: n.to_string(),
                    value: None,
                    source_location: SourceLocation::new("test.puml", 0),
                })
                .collect(),
        })
    }

    fn make_arrow(left: Option<&str>, line: &str, right: Option<&str>) -> Arrow {
        Arrow {
            left: left.map(|v| ArrowDecor { raw: v.to_string() }),
            line: ArrowLine {
                raw: line.to_string(),
            },
            middle: None,
            right: right.map(|v| ArrowDecor { raw: v.to_string() }),
        }
    }

    // ----------------------------
    // resolve_entity_id
    // ----------------------------
    #[test]
    fn test_resolve_entity_id_root() {
        let resolver = ClassResolver::new();
        let id = resolver.resolve_entity_id("User", &InternalScope::default());
        assert_eq!(id, "User");
    }

    #[test]
    fn test_resolve_entity_id_nested() {
        let resolver = ClassResolver::new();
        let id = resolver.resolve_entity_id("User", &InternalScope::from_path("core"));
        assert_eq!(id, "core.User");
    }

    #[test]
    fn test_resolve_entity_id_normalizes_namespace_separator() {
        let resolver = ClassResolver::new();

        let root_id = resolver.resolve_entity_id("core::geometry", &InternalScope::default());
        let nested_id =
            resolver.resolve_entity_id("User", &InternalScope::from_path("core::geometry"));

        assert_eq!(root_id, "core.geometry");
        assert_eq!(nested_id, "core.geometry.User");
    }

    // ----------------------------
    // process_class
    // ----------------------------
    #[test]
    fn test_process_class() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&make_class("User"), &InternalScope::default())
            .unwrap();
        assert_eq!(resolver.logic.entities.len(), 1);

        let entity = &resolver.logic.entities[0];
        assert_eq!(entity.id, "User");
        assert_eq!(entity.name, "User");
        assert_eq!(entity.entity_type, EntityType::Class);
    }

    #[test]
    fn test_process_class_prefers_alias_for_id_leaf() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(
                &make_aliased_class("Foo", "F"),
                &InternalScope::from_path("pkg"),
            )
            .unwrap();

        let entity = &resolver.logic.entities[0];
        assert_eq!(entity.id, "pkg.F");
        assert_eq!(entity.name, "F");
    }

    #[test]
    fn test_duplicate_entity_ids_are_rejected() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&make_class("User"), &InternalScope::default())
            .unwrap();

        let result = resolver.process_element(&make_class("User"), &InternalScope::default());

        assert!(matches!(
            result,
            Err(ClassPumlResolverError::DuplicateEntity { ref entity_id, .. }) if entity_id == "User"
        ));
    }

    // ----------------------------
    // process_enum
    // ----------------------------
    #[test]
    fn test_process_enum() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(
                &make_enum("Color", vec!["Red", "Green", "Blue"]),
                &InternalScope::default(),
            )
            .unwrap();

        assert_eq!(resolver.logic.entities.len(), 1);

        let entity = &resolver.logic.entities[0];
        assert_eq!(entity.id, "Color");
        assert_eq!(entity.entity_type, EntityType::Enum);
        assert_eq!(entity.enum_literals.len(), 3);
    }

    // ----------------------------
    // resolve_reference (Rule C)
    // ----------------------------
    #[test]
    fn test_resolve_reference_direct_scope_hit() {
        let mut resolver = ClassResolver::new();
        let scope = InternalScope::from_path("core");
        resolver
            .process_element(&make_class("User"), &scope)
            .unwrap();

        let resolved = resolver.resolve_reference("User", &scope, 100).unwrap();
        assert_eq!(resolved, "core.User");
    }

    #[test]
    fn test_resolve_reference_qualified_path_from_root() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(
                &make_class("User"),
                &InternalScope::from_path("core::geometry"),
            )
            .unwrap();

        let resolved = resolver
            .resolve_reference("core::geometry::User", &InternalScope::default(), 100)
            .unwrap();

        assert_eq!(resolved, "core.geometry.User");
    }

    #[test]
    fn test_resolve_reference_falls_back_to_unique_leaf() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&make_class("User"), &InternalScope::from_path("core"))
            .unwrap();

        let resolved = resolver
            .resolve_reference("User", &InternalScope::from_path("other"), 100)
            .unwrap();

        assert_eq!(resolved, "core.User");
    }

    #[test]
    fn test_resolve_reference_several_leaves_is_ambiguous() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&make_class("User"), &InternalScope::from_path("p"))
            .unwrap();
        resolver
            .process_element(&make_class("User"), &InternalScope::from_path("q"))
            .unwrap();

        let result = resolver.resolve_reference("User", &InternalScope::default(), 100);

        assert!(matches!(
            result,
            Err(ClassPumlResolverError::AmbiguousReference { ref candidates, .. })
                if candidates == &["p.User".to_string(), "q.User".to_string()]
        ));
    }

    #[test]
    fn test_resolve_reference_only_sees_elements_declared_before_it() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&make_class_at("User", 10), &InternalScope::from_path("p"))
            .unwrap();

        let too_early = resolver.resolve_reference("User", &InternalScope::default(), 5);
        assert!(matches!(
            too_early,
            Err(ClassPumlResolverError::UnresolvedReference { .. })
        ));

        let resolved = resolver
            .resolve_reference("User", &InternalScope::default(), 20)
            .unwrap();
        assert_eq!(resolved, "p.User");
    }

    #[test]
    fn test_resolve_reference_direct_scope_hit_ignores_declaration_order() {
        let mut resolver = ClassResolver::new();
        let scope = InternalScope::from_path("p");
        resolver
            .process_element(&make_class_at("User", 10), &scope)
            .unwrap();

        let resolved = resolver.resolve_reference("User", &scope, 1).unwrap();
        assert_eq!(resolved, "p.User");
    }

    #[test]
    fn test_resolve_reference_root_marker_bypasses_scope() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&make_class("X"), &InternalScope::default())
            .unwrap();
        resolver
            .process_element(&make_class("X"), &InternalScope::from_path("a"))
            .unwrap();

        let resolved = resolver
            .resolve_reference(".X", &InternalScope::from_path("a"), 100)
            .unwrap();
        assert_eq!(resolved, "X");
    }

    #[test]
    fn test_root_anchor_prefixes_ids_and_rooted_refs() {
        let mut resolver = ClassResolver::with_root_anchor(Some("score::mw"));
        resolver
            .process_element(&make_class("X"), &InternalScope::default())
            .unwrap();
        resolver
            .process_element(&make_class("X"), &InternalScope::from_path("a"))
            .unwrap();
        resolver
            .process_element(&make_class("C"), &InternalScope::from_path("a"))
            .unwrap();

        let ids: Vec<&str> = resolver
            .logic
            .entities
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert!(ids.contains(&"score.mw.X"));
        assert!(ids.contains(&"score.mw.a.X"));

        let resolved = resolver
            .resolve_reference(".X", &InternalScope::from_path("a"), 100)
            .unwrap();
        assert_eq!(resolved, "score.mw.X");
    }

    // ----------------------------
    // convert_arrow
    // ----------------------------
    #[test]
    fn test_convert_arrow_cases() {
        let resolver = ClassResolver::new();

        struct Case {
            arrow: Arrow,
            expected_ty: RelationType,
            expected_reversed: bool,
        }

        let cases = vec![
            Case {
                arrow: make_arrow(Some("<|"), "--", None),
                expected_ty: RelationType::Inheritance,
                expected_reversed: true,
            },
            Case {
                arrow: make_arrow(None, "--", Some("|>")),
                expected_ty: RelationType::Inheritance,
                expected_reversed: false,
            },
            Case {
                arrow: make_arrow(None, "--", Some(">")),
                expected_ty: RelationType::Association,
                expected_reversed: false,
            },
            Case {
                arrow: make_arrow(None, "..", Some("|>")),
                expected_ty: RelationType::Implementation,
                expected_reversed: false,
            },
            Case {
                arrow: make_arrow(None, "--", Some("*")),
                expected_ty: RelationType::Composition,
                expected_reversed: false,
            },
            Case {
                arrow: make_arrow(None, "--", Some("o")),
                expected_ty: RelationType::Aggregation,
                expected_reversed: false,
            },
            Case {
                arrow: make_arrow(Some("<"), "--", None),
                expected_ty: RelationType::Association,
                expected_reversed: true,
            },
            Case {
                arrow: make_arrow(Some("<"), "..", None),
                expected_ty: RelationType::Dependency,
                expected_reversed: true,
            },
        ];

        for (i, case) in cases.into_iter().enumerate() {
            let (ty, reversed) = resolver.convert_arrow(&case.arrow).unwrap();

            assert_eq!(ty, case.expected_ty, "case {} failed: type mismatch", i);
            assert_eq!(
                reversed, case.expected_reversed,
                "case {} failed: reversed mismatch",
                i
            );
        }
    }

    #[test]
    fn test_convert_arrow_invalid() {
        let resolver = ClassResolver::new();

        let invalid_cases = vec![
            make_arrow(Some("?"), "~~", Some("?")),
            make_arrow(None, "--", None),
            make_arrow(None, "..", None),
            make_arrow(Some("+"), "-", None),
        ];

        for arrow in invalid_cases {
            let result = resolver.convert_arrow(&arrow);
            assert!(matches!(
                result,
                Err(ClassPumlResolverError::InvalidRelationship { .. })
            ));
        }
    }

    // ----------------------------
    // relationship
    // ----------------------------
    #[test]
    fn test_process_relationship_inheritance() {
        let mut resolver = ClassResolver::new();

        resolver
            .process_element(&make_class("A"), &InternalScope::default())
            .unwrap();
        resolver
            .process_element(&make_class("B"), &InternalScope::default())
            .unwrap();

        let rel = ParserRelationship {
            left: "A".to_string(),
            right: "B".to_string(),
            arrow: make_arrow(Some("<|"), "--", None),
            left_multiplicity: None,
            right_multiplicity: None,
            label: Some("<<label>>".to_string()),
            source_location: SourceLocation::new("test.puml", 42),
        };

        resolver
            .process_relationship(&rel, &InternalScope::default())
            .unwrap();

        let source_entity = resolver
            .logic
            .entities
            .iter()
            .find(|entity| entity.id == "B")
            .expect("missing source entity");
        assert_eq!(source_entity.relationships.len(), 1);

        let r = &source_entity.relationships[0];
        assert_eq!(r.source, "B");
        assert_eq!(r.target, "A");
        assert_eq!(r.relation_type, RelationType::Inheritance);
        assert_eq!(r.source_location.line, 42);
    }

    #[test]
    fn test_process_relationship_unresolved_left() {
        let mut resolver = ClassResolver::new();

        let rel = ParserRelationship {
            left: "UnknownA".to_string(),
            right: "KnownB".to_string(),
            arrow: make_arrow(None, "--", Some(">")),
            left_multiplicity: None,
            right_multiplicity: None,
            label: None,
            source_location: SourceLocation::new("test.puml", 0),
        };

        let result = resolver.process_relationship(&rel, &InternalScope::default());

        assert!(matches!(
            result,
            Err(ClassPumlResolverError::UnresolvedReference { ref reference }) if reference == "UnknownA"
        ));
    }

    // ----------------------------
    // namespace
    // ----------------------------
    #[test]
    fn test_process_namespace() {
        let mut resolver = ClassResolver::new();

        let ns = Namespace {
            name: make_name("core::geometry"),
            types: vec![make_class("User")],
            relationships: vec![],
            namespaces: vec![],
        };

        resolver
            .process_namespace(&ns, &InternalScope::default())
            .unwrap();

        assert_eq!(resolver.logic.entities.len(), 1);

        let entity = &resolver.logic.entities[0];
        assert_eq!(entity.id, "core.geometry.User");
    }

    // ----------------------------
    // resolve integration
    // ----------------------------
    #[test]
    fn test_visit_document_simple() {
        let mut resolver = ClassResolver::new();

        let file = ClassUmlFile {
            name: "test".to_string(),
            elements: vec![ClassUmlTopLevel::Types(make_class("User"))],
            relationships: vec![],
        };

        let logic = resolver.resolve(&file).unwrap();
        assert_eq!(logic.name, "test");
        assert_eq!(logic.entities.len(), 1);
        assert_eq!(logic.entities[0].id, "User");
        assert!(logic.entities[0].relationships.is_empty());
        assert_eq!(logic.entities[0].source_location.file.as_ref(), "test.puml");
    }

    // ----------------------------
    // top_level
    // ----------------------------
    #[test]
    fn test_process_top_level_enum_and_namespace() {
        let cases = vec![
            ClassUmlTopLevel::Enum(EnumDef {
                name: make_name("MyEnum"),
                namespace: "".to_string(),
                package: "".to_string(),
                source_location: SourceLocation::new("test.puml", 1),
                stereotypes: Vec::new(),
                items: vec![],
            }),
            ClassUmlTopLevel::Namespace(Namespace {
                name: make_name("ns"),
                types: vec![],
                relationships: vec![],
                namespaces: vec![],
            }),
        ];

        for case in cases {
            let mut resolver = ClassResolver::new();
            assert!(resolver
                .process_top_level(&case, &InternalScope::default())
                .is_ok());
        }
    }
}
