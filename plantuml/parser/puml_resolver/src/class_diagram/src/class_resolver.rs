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

#[derive(Debug, Error)]
pub enum ClassPumlResolverError {
    #[error("Class Resolver: Unresolved reference: {reference}")]
    UnresolvedReference { reference: String },

    #[error("Duplicate entity id: {entity_id}")]
    DuplicateEntity { entity_id: String },

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

    // internal name or alias -> FQN
    name_map: HashMap<String, String>,
}

impl Default for ClassResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassResolver {
    pub fn new() -> Self {
        Self {
            logic: ClassDiagram {
                name: String::new(),
                entities: Vec::new(),
                relationships: Vec::new(),
                source_files: Vec::new(),
                version: None,
            },
            name_map: HashMap::new(),
        }
    }

    fn analyze(&mut self, file: &ClassUmlFile) -> Result<(), ClassPumlResolverError> {
        for elem in &file.elements {
            self.process_top_level(elem, None)?;
        }

        for elem in &file.elements {
            self.process_declared_relations_top_level(elem, None)?;
        }

        for rel in &file.relationships {
            self.process_relationship(rel, None)?;
        }

        self.populate_entity_relationships();

        Ok(())
    }

    fn populate_entity_relationships(&mut self) {
        let mut outgoing_relationships: HashMap<String, Vec<Relationship>> = HashMap::new();

        for relationship in &self.logic.relationships {
            outgoing_relationships
                .entry(relationship.source.clone())
                .or_default()
                .push(relationship.clone());
        }

        for entity in &mut self.logic.entities {
            entity.relationships = outgoing_relationships
                .get(&entity.id)
                .cloned()
                .unwrap_or_default();
        }
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

    fn normalize_fqn(raw: &str) -> String {
        raw.replace("::", ".").trim_matches('.').to_string()
    }

    fn build_fqn(&self, name: &str, parent: &Option<String>) -> String {
        let normalized_name = Self::normalize_fqn(name);

        match parent {
            Some(p) => {
                let normalized_parent = Self::normalize_fqn(p);

                if normalized_parent.is_empty() {
                    normalized_name
                } else if normalized_name.is_empty() {
                    normalized_parent
                } else {
                    format!("{}.{}", normalized_parent, normalized_name)
                }
            }
            None => normalized_name,
        }
    }

    fn entity_name(name: &Name) -> String {
        name.display
            .clone()
            .unwrap_or_else(|| name.internal.clone())
    }

    fn current_source_file(&self) -> Option<String> {
        if self.logic.name.is_empty() {
            None
        } else {
            Some(self.logic.name.clone())
        }
    }

    fn register_entity_names(&mut self, name: &Name, id: &str) {
        self.name_map.insert(name.internal.clone(), id.to_string());

        if let Some(display) = &name.display {
            self.name_map.insert(display.clone(), id.to_string());
        }
    }

    fn resolve_name(&self, name: &str, parent: &Option<String>) -> Option<String> {
        // 1. FQN
        if name.contains('.') || name.contains("::") {
            return Some(Self::normalize_fqn(name));
        }

        // 2. Current Namespace
        if let Some(p) = parent {
            let candidate = format!("{}.{}", p, name);

            if self.logic.entities.iter().any(|e| e.id == candidate) {
                return Some(candidate);
            }
        }

        // 3. Global search id
        if let Some(id) = self.name_map.get(name) {
            return Some(id.clone());
        }

        // 4. Global search exported name
        if let Some(e) = self.logic.entities.iter().find(|e| e.name == name) {
            return Some(e.id.clone());
        }

        None
    }

    fn process_top_level(
        &mut self,
        elem: &ClassUmlTopLevel,
        parent: Option<String>,
    ) -> Result<(), ClassPumlResolverError> {
        match elem {
            ClassUmlTopLevel::Types(element) => {
                self.process_element(element, parent);
            }

            ClassUmlTopLevel::Enum(enum_def) => {
                self.process_enum(enum_def, parent);
            }

            ClassUmlTopLevel::Namespace(ns) => {
                self.process_namespace(ns, parent)?;
            }

            ClassUmlTopLevel::Package(pkg) => {
                self.process_package(pkg, parent)?;
            }
        }
        Ok(())
    }

    fn process_declared_relations_top_level(
        &mut self,
        elem: &ClassUmlTopLevel,
        parent: Option<String>,
    ) -> Result<(), ClassPumlResolverError> {
        match elem {
            ClassUmlTopLevel::Types(element) => {
                self.process_declared_relations_element(element, parent)?;
            }
            ClassUmlTopLevel::Enum(_) => {}
            ClassUmlTopLevel::Namespace(ns) => {
                self.process_namespace_declared_relations(ns, parent)?;
            }
            ClassUmlTopLevel::Package(pkg) => {
                self.process_package_declared_relations(pkg, parent)?;
            }
        }

        Ok(())
    }

    fn process_package(
        &mut self,
        pkg: &Package,
        parent: Option<String>,
    ) -> Result<(), ClassPumlResolverError> {
        let fqn = self.build_fqn(&pkg.name.internal, &parent);

        for t in &pkg.types {
            self.process_element(t, Some(fqn.clone()));
        }

        for sub in &pkg.packages {
            self.process_package(sub, Some(fqn.clone()))?;
        }

        for rel in &pkg.relationships {
            self.process_relationship(rel, Some(fqn.clone()))?;
        }

        Ok(())
    }

    fn process_namespace(
        &mut self,
        ns: &Namespace,
        parent: Option<String>,
    ) -> Result<(), ClassPumlResolverError> {
        let fqn = self.build_fqn(&ns.name.internal, &parent);

        for t in &ns.types {
            self.process_element(t, Some(fqn.clone()));
        }

        for sub in &ns.namespaces {
            self.process_namespace(sub, Some(fqn.clone()))?;
        }

        Ok(())
    }

    fn process_package_declared_relations(
        &mut self,
        pkg: &Package,
        parent: Option<String>,
    ) -> Result<(), ClassPumlResolverError> {
        let fqn = self.build_fqn(&pkg.name.internal, &parent);

        for t in &pkg.types {
            self.process_declared_relations_element(t, Some(fqn.clone()))?;
        }

        for sub in &pkg.packages {
            self.process_package_declared_relations(sub, Some(fqn.clone()))?;
        }

        Ok(())
    }

    fn process_namespace_declared_relations(
        &mut self,
        ns: &Namespace,
        parent: Option<String>,
    ) -> Result<(), ClassPumlResolverError> {
        let fqn = self.build_fqn(&ns.name.internal, &parent);

        for t in &ns.types {
            self.process_declared_relations_element(t, Some(fqn.clone()))?;
        }

        for sub in &ns.namespaces {
            self.process_namespace_declared_relations(sub, Some(fqn.clone()))?;
        }

        Ok(())
    }

    fn process_declared_relations_element(
        &mut self,
        element: &Element,
        parent: Option<String>,
    ) -> Result<(), ClassPumlResolverError> {
        match element {
            Element::ClassDef(def) => {
                self.process_extends_relationships(
                    &def.name.internal,
                    &def.extends,
                    parent.clone(),
                )?;
                self.process_implements_relationships(&def.name.internal, &def.implements, parent)?;
            }
            Element::InterfaceDef(def) => {
                self.process_extends_relationships(&def.name.internal, &def.extends, parent)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn process_extends_relationships(
        &mut self,
        child_name: &str,
        bases: &[String],
        parent: Option<String>,
    ) -> Result<(), ClassPumlResolverError> {
        self.process_declared_relationships(child_name, bases, parent, RelationType::Inheritance)
    }

    fn process_implements_relationships(
        &mut self,
        class_name: &str,
        interfaces: &[String],
        parent: Option<String>,
    ) -> Result<(), ClassPumlResolverError> {
        self.process_declared_relationships(
            class_name,
            interfaces,
            parent,
            RelationType::Implementation,
        )
    }

    fn process_declared_relationships(
        &mut self,
        source_name: &str,
        targets: &[String],
        parent: Option<String>,
        relation_type: RelationType,
    ) -> Result<(), ClassPumlResolverError> {
        if targets.is_empty() {
            return Ok(());
        }

        let source = self.build_fqn(source_name, &parent);

        for declared_target in targets {
            let target = self.resolve_name(declared_target, &parent).ok_or_else(|| {
                ClassPumlResolverError::UnresolvedReference {
                    reference: declared_target.clone(),
                }
            })?;

            self.logic.relationships.push(Relationship {
                source: source.clone(),
                target,
                relation_type,
                source_multiplicity: None,
                target_multiplicity: None,
            });
        }

        Ok(())
    }

    fn process_element(&mut self, element: &Element, parent: Option<String>) {
        match element {
            Element::EnumDef(def) => self.process_enum(def, parent),
            _ => {
                let entity_type = match element {
                    Element::ClassDef(def) if def.is_abstract => EntityType::AbstractClass,
                    Element::ClassDef(_) => EntityType::Class,
                    Element::StructDef(_) => EntityType::Struct,
                    Element::InterfaceDef(_) => EntityType::Interface,
                    _ => unreachable!(),
                };
                self.process_class(element, parent, entity_type);
            }
        }
    }

    fn process_class(&mut self, def: &Element, parent: Option<String>, entity_type: EntityType) {
        let (name, attributes, type_aliases, methods, template_parameters, source_line) = match def
        {
            Element::ClassDef(c) => (
                &c.name,
                &c.attributes,
                &c.type_aliases,
                &c.methods,
                &c.template_parameters,
                c.source_line,
            ),
            Element::StructDef(s) => (
                &s.name,
                &s.attributes,
                &s.type_aliases,
                &s.methods,
                &s.template_parameters,
                s.source_line,
            ),
            Element::InterfaceDef(i) => (
                &i.name,
                &i.attributes,
                &i.type_aliases,
                &i.methods,
                &i.template_parameters,
                i.source_line,
            ),
            Element::EnumDef(_) => unreachable!("EnumDef should not be passed to process_class"),
        };

        let id = self.build_fqn(&name.internal, &parent);

        let entity = SimpleEntity {
            id: id.clone(),
            name: Self::entity_name(name),
            enclosing_namespace_id: parent.clone(),
            entity_type,
            type_aliases: type_aliases.iter().map(Self::convert_type_alias).collect(),
            variables: attributes.iter().map(Self::convert_variable).collect(),
            methods: methods
                .iter()
                .map(|method| Self::convert_method(method, &name.internal))
                .collect(),
            template_parameters: template_parameters.clone(),
            enum_literals: vec![],
            relationships: vec![],
            source_file: self.current_source_file(),
            source_line,
        };

        self.register_entity_names(name, &id);
        self.logic.entities.push(entity);
    }

    fn convert_type_alias(type_alias: &ParserTypeAlias) -> TypeAlias {
        TypeAlias {
            alias: type_alias.alias.clone(),
            original_type: type_alias.original_type.clone(),
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
            is_const: has_modifier(&attr.modifiers, "const"),
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

        Method {
            name: m.name.clone(),
            return_type: m.r#type.clone(),
            visibility: Self::map_visibility(m.visibility.clone()),
            parameters: m.params.iter().map(Self::convert_param).collect(),
            template_parameters: m.template_parameters.clone(),
            modifiers: MethodModifier::make_modifier_vec(
                has_modifier(&m.modifiers, "static"),
                false,
                has_modifier(&m.modifiers, "abstract"),
                false,
                is_constructor,
                is_destructor,
            ),
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

    fn convert_param(param: &ParserParam) -> FunctionArgument {
        FunctionArgument {
            name: param.name.clone().unwrap_or_default(),
            param_type: param.param_type.clone(),
            is_variadic: param.varargs,
        }
    }

    fn process_enum(&mut self, def: &EnumDef, parent: Option<String>) {
        let id = self.build_fqn(&def.name.internal, &parent);

        let literals = def
            .items
            .iter()
            .map(|item| EnumLiteral {
                name: item.name.clone(),
                value: match &item.value {
                    Some(EnumValue::Literal(v)) => Some(v.clone()),
                    Some(EnumValue::Description(d)) => Some(d.clone()),
                    None => None,
                },
            })
            .collect();

        self.logic.entities.push(SimpleEntity {
            id: id.clone(),
            name: Self::entity_name(&def.name),
            enclosing_namespace_id: parent.clone(),
            entity_type: EntityType::Enum,
            type_aliases: vec![],
            variables: vec![],
            methods: vec![],
            template_parameters: None,
            enum_literals: literals,
            relationships: vec![],
            source_file: self.current_source_file(),
            source_line: def.source_line,
        });

        self.register_entity_names(&def.name, &id);
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
        parent: Option<String>,
    ) -> Result<(), ClassPumlResolverError> {
        let left = self.resolve_name(&rel.left, &parent).ok_or_else(|| {
            ClassPumlResolverError::UnresolvedReference {
                reference: rel.left.clone(),
            }
        })?;

        let right = self.resolve_name(&rel.right, &parent).ok_or_else(|| {
            ClassPumlResolverError::UnresolvedReference {
                reference: rel.right.clone(),
            }
        })?;

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

        self.logic.relationships.push(Relationship {
            source: source_id,
            target: target_id,
            relation_type,
            source_multiplicity,
            target_multiplicity,
        });

        Ok(())
    }
}

impl DiagramResolver for ClassResolver {
    type Document = ClassUmlFile;
    type Output = ClassDiagram;
    type Error = ClassPumlResolverError;

    // DESIGN: single-pass resolver — all logic lives in analyze(); there is no
    // per-statement visitor step.
    fn resolve(&mut self, document: &Self::Document) -> Result<Self::Output, Self::Error> {
        self.name_map.clear();

        self.logic.name = document.name.clone();
        self.logic.source_files.push(document.name.clone());

        self.analyze(document)?;

        let logic_class = std::mem::replace(
            &mut self.logic,
            ClassDiagram {
                name: String::new(),
                entities: Vec::new(),
                relationships: Vec::new(),
                source_files: Vec::new(),
                version: None,
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

    fn make_class(name: &str) -> Element {
        Element::ClassDef(ClassDef {
            name: make_name(name),
            namespace: "".to_string(),
            package: "".to_string(),
            source_line: None,
            is_abstract: false,
            template_parameters: None,
            extends: vec![],
            implements: vec![],
            attributes: vec![],
            type_aliases: vec![],
            methods: vec![],
        })
    }

    fn make_enum(name: &str, items: Vec<&str>) -> Element {
        Element::EnumDef(EnumDef {
            name: make_name(name),
            namespace: "".to_string(),
            package: "".to_string(),
            source_line: None,
            stereotypes: vec![],
            items: items
                .into_iter()
                .map(|n| EnumItem {
                    name: n.to_string(),
                    value: None,
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
    // build_fqn
    // ----------------------------
    #[test]
    fn test_build_fqn_root() {
        let resolver = ClassResolver::new();
        let fqn = resolver.build_fqn("User", &None);
        assert_eq!(fqn, "User");
    }

    #[test]
    fn test_build_fqn_nested() {
        let resolver = ClassResolver::new();
        let fqn = resolver.build_fqn("User", &Some("core".to_string()));
        assert_eq!(fqn, "core.User");
    }

    #[test]
    fn test_build_fqn_normalizes_namespace_separator() {
        let resolver = ClassResolver::new();

        let root_fqn = resolver.build_fqn("core::geometry", &None);
        let nested_fqn = resolver.build_fqn("User", &Some("core::geometry".to_string()));

        assert_eq!(root_fqn, "core.geometry");
        assert_eq!(nested_fqn, "core.geometry.User");
    }

    // ----------------------------
    // process_class
    // ----------------------------
    #[test]
    fn test_process_class() {
        let mut resolver = ClassResolver::new();
        resolver.process_element(&make_class("User"), None);
        assert_eq!(resolver.logic.entities.len(), 1);

        let entity = &resolver.logic.entities[0];
        assert_eq!(entity.id, "User");
        assert_eq!(entity.name, "User");
        assert_eq!(entity.entity_type, EntityType::Class);
    }

    // ----------------------------
    // process_enum
    // ----------------------------
    #[test]
    fn test_process_enum() {
        let mut resolver = ClassResolver::new();
        resolver.process_element(&make_enum("Color", vec!["Red", "Green", "Blue"]), None);

        assert_eq!(resolver.logic.entities.len(), 1);

        let entity = &resolver.logic.entities[0];
        assert_eq!(entity.id, "Color");
        assert_eq!(entity.entity_type, EntityType::Enum);
        assert_eq!(entity.enum_literals.len(), 3);
    }

    // ----------------------------
    // resolve_name
    // ----------------------------
    #[test]
    fn test_resolve_name_global() {
        let mut resolver = ClassResolver::new();
        resolver.process_element(&make_class("User"), None);

        let resolved = resolver.resolve_name("User", &None);
        assert_eq!(resolved, Some("User".to_string()));
    }

    #[test]
    fn test_resolve_name_namespace() {
        let mut resolver = ClassResolver::new();
        resolver.process_element(&make_class("User"), Some("core".to_string()));

        let resolved = resolver.resolve_name("User", &Some("core".to_string()));
        assert_eq!(resolved, Some("core.User".to_string()));
    }

    #[test]
    fn test_resolve_name_normalizes_namespace_separator() {
        let resolver = ClassResolver::new();

        let resolved = resolver.resolve_name("core::geometry::User", &None);

        assert_eq!(resolved, Some("core.geometry.User".to_string()));
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

        resolver.process_element(&make_class("A"), None);
        resolver.process_element(&make_class("B"), None);

        let rel = ParserRelationship {
            left: "A".to_string(),
            right: "B".to_string(),
            arrow: make_arrow(Some("<|"), "--", None),
            left_multiplicity: None,
            right_multiplicity: None,
            label: Some("<<label>>".to_string()),
        };

        resolver.process_relationship(&rel, None).unwrap();

        assert_eq!(resolver.logic.relationships.len(), 1);

        let r = &resolver.logic.relationships[0];
        assert_eq!(r.source, "B");
        assert_eq!(r.target, "A");
        assert_eq!(r.relation_type, RelationType::Inheritance);
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
        };

        let result = resolver.process_relationship(&rel, None);

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
            namespaces: vec![],
        };

        resolver.process_namespace(&ns, None).unwrap();

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
        assert_eq!(logic.entities[0].source_file.as_deref(), Some("test"));
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
                source_line: None,
                stereotypes: vec![],
                items: vec![],
            }),
            ClassUmlTopLevel::Namespace(Namespace {
                name: make_name("ns"),
                types: vec![],
                namespaces: vec![],
            }),
        ];

        for case in cases {
            let mut resolver = ClassResolver::new();
            assert!(resolver.process_top_level(&case, None).is_ok());
        }
    }
}
