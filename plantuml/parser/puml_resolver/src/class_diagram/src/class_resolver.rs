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
    Attribute, ClassUmlFile, ClassUmlTopLevel, Element, EnumDef, EnumValue, Method, Namespace,
    Package, Param, Relationship,
};
use parser_core::common_ast::Arrow;
use resolver_traits::DiagramResolver;

pub struct ClassResolver {
    pub logic: ClassDiagram,

    // simple name -> FQN
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
                containers: Vec::new(),
                relationships: Vec::new(),
                source_files: Vec::new(),
                version: None,
            },
            name_map: HashMap::new(),
        }
    }

    fn analyze(&mut self, file: &ClassUmlFile) -> Result<(), ClassResolverError> {
        for elem in &file.elements {
            self.process_top_level(elem, None)?;
        }

        for rel in &file.relationships {
            self.process_relationship(rel, None)?;
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
            ParserVisibility::Package => ResolverVisibility::Package,
        }
    }

    fn build_fqn(&self, name: &str, parent: &Option<String>) -> String {
        match parent {
            Some(p) => format!("{}.{}", p, name),
            None => name.to_string(),
        }
    }

    fn resolve_name(&self, name: &str, parent: &Option<String>) -> Option<String> {
        // 1. FQN
        if name.contains('.') || name.contains("::") {
            return Some(name.to_string());
        }

        // 2. Current Namespace
        if let Some(p) = parent {
            let candidate = format!("{}.{}", p, name);

            // All three checks now verify the candidate actually exists
            if self.logic.entities.iter().any(|e| e.id == candidate)
                || self
                    .logic
                    .entities
                    .iter()
                    .any(|e| e.id == candidate && e.name.as_deref() == Some(name))
                || self
                    .logic
                    .entities
                    .iter()
                    .any(|e| e.id == candidate && e.alias.as_deref() == Some(name))
            {
                return Some(candidate);
            }
        }

        // 3. Global search id
        if let Some(id) = self.name_map.get(name) {
            return Some(id.clone());
        }

        // 4. Global search name / alias
        if let Some(e) = self
            .logic
            .entities
            .iter()
            .find(|e| e.name.as_deref() == Some(name) || e.alias.as_deref() == Some(name))
        {
            return Some(e.id.clone());
        }

        None
    }

    fn process_top_level(
        &mut self,
        elem: &ClassUmlTopLevel,
        parent: Option<String>,
    ) -> Result<(), ClassResolverError> {
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

    fn process_package(
        &mut self,
        pkg: &Package,
        parent: Option<String>,
    ) -> Result<(), ClassResolverError> {
        let fqn = match &parent {
            Some(p) => format!("{}.{}", p, pkg.name.internal),
            None => pkg.name.internal.clone(),
        };

        self.logic.containers.push(LogicContainer {
            id: fqn.clone(),
            name: pkg.name.internal.clone(),
            parent_id: parent.clone(),
            container_type: ContainerType::Package,
        });

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
    ) -> Result<(), ClassResolverError> {
        let fqn = match &parent {
            Some(p) => format!("{}.{}", p, ns.name.internal),
            None => ns.name.internal.clone(),
        };

        self.logic.containers.push(LogicContainer {
            id: fqn.clone(),
            name: ns.name.internal.clone(),
            parent_id: parent.clone(),
            container_type: ContainerType::Namespace,
        });

        for t in &ns.types {
            self.process_element(t, Some(fqn.clone()));
        }

        for sub in &ns.namespaces {
            self.process_namespace(sub, Some(fqn.clone()))?;
        }

        Ok(())
    }

    fn process_element(&mut self, element: &Element, parent: Option<String>) {
        match element {
            Element::EnumDef(def) => self.process_enum(def, parent),
            _ => {
                let entity_type = match element {
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
        let (name, attributes, methods) = match def {
            Element::ClassDef(c) => (&c.name, &c.attributes, &c.methods),
            Element::StructDef(s) => (&s.name, &s.attributes, &s.methods),
            Element::InterfaceDef(i) => (&i.name, &i.attributes, &i.methods),
            Element::EnumDef(_) => unreachable!("EnumDef should not be passed to process_class"),
        };

        let id = self.build_fqn(&name.internal, &parent);

        let entity = LogicEntity {
            id: id.clone(),
            name: Some(name.internal.clone()),
            alias: name.display.clone(),
            parent_id: parent.clone(),
            entity_type,
            stereotypes: vec![],
            attributes: attributes.iter().map(Self::convert_attr).collect(),
            methods: methods.iter().map(Self::convert_method).collect(),
            template_params: vec![],
            enum_literals: vec![],
            source_file: None,
            source_line: None,
        };

        self.name_map.insert(name.internal.clone(), id.clone());
        self.logic.entities.push(entity);
    }

    fn convert_attr(attr: &Attribute) -> LogicAttribute {
        LogicAttribute {
            name: attr.name.clone(),
            data_type: attr.r#type.clone(),
            visibility: Self::map_visibility(attr.visibility.clone()),
            default_value: None,
            is_static: false,
            is_const: false,
            description: None,
        }
    }

    fn convert_method(m: &Method) -> LogicMethod {
        LogicMethod {
            name: m.name.clone(),
            return_type: m.r#type.clone(),
            visibility: Self::map_visibility(m.visibility.clone()),
            parameters: m.params.iter().map(Self::convert_param).collect(),
            template_params: m.generic_params.clone(),
            is_static: false,
            is_const: false,
            is_virtual: false,
            is_abstract: false,
            is_override: false,
            is_constructor: false,
            is_destructor: false,
        }
    }

    fn convert_param(param: &Param) -> LogicParameter {
        LogicParameter {
            name: param.name.clone().unwrap(),
            param_type: Some(param.param_type.clone()),
            default_value: None,
            is_reference: false,
            is_const: false,
            is_variadic: param.varargs,
        }
    }

    fn process_enum(&mut self, def: &EnumDef, parent: Option<String>) {
        let id = self.build_fqn(&def.name.internal, &parent);

        let literals = def
            .items
            .iter()
            .map(|item| LogicEnumLiteral {
                name: item.name.clone(),
                visibility: item
                    .visibility
                    .clone()
                    .map(Self::map_visibility)
                    .unwrap_or(ResolverVisibility::Public),
                value: match &item.value {
                    Some(EnumValue::Literal(v)) => Some(v.clone()),
                    Some(EnumValue::Description(d)) => Some(d.clone()),
                    None => None,
                },
                description: None,
            })
            .collect();

        self.logic.entities.push(LogicEntity {
            id: id.clone(),
            name: def.name.display.clone(),
            alias: None,
            parent_id: parent.clone(),
            entity_type: EntityType::Enum,
            stereotypes: def.stereotypes.clone(),
            attributes: vec![],
            methods: vec![],
            template_params: vec![],
            enum_literals: literals,
            source_file: None,
            source_line: None,
        });

        self.name_map.insert(def.name.internal.clone(), id);
    }

    fn convert_arrow(&self, arrow: &Arrow) -> Result<(RelationType, bool), ClassResolverError> {
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

        // ---------------- Undirected ----------------
        if line == "--" {
            return Ok((RelationType::Link, false));
        }

        if line == ".." {
            return Ok((RelationType::DashedLink, false));
        }

        Err(ClassResolverError::InvalidRelationship {
            from: left.to_string(),
            to: right.to_string(),
            reason: format!("Unsupported arrow pattern: {}{}{}", left, line, right),
        })
    }

    fn process_relationship(
        &mut self,
        rel: &Relationship,
        parent: Option<String>,
    ) -> Result<(), ClassResolverError> {
        let left = self.resolve_name(&rel.left, &parent).ok_or_else(|| {
            ClassResolverError::UnresolvedReference {
                reference: rel.left.clone(),
            }
        })?;

        let right = self.resolve_name(&rel.right, &parent).ok_or_else(|| {
            ClassResolverError::UnresolvedReference {
                reference: rel.right.clone(),
            }
        })?;

        let (relation_type, reversed) = self.convert_arrow(&rel.arrow)?;

        let (source_id, target_id) = if reversed {
            (right, left)
        } else {
            (left, right)
        };

        let (label, stereotype) = match &rel.label {
            Some(text) => {
                let trimmed = text.trim();
                if trimmed.starts_with("<<") && trimmed.ends_with(">>") {
                    let inner = trimmed
                        .trim_start_matches("<<")
                        .trim_end_matches(">>")
                        .trim()
                        .to_string();
                    (None, Some(inner))
                } else {
                    (Some(text.clone()), None)
                }
            }
            None => (None, None),
        };

        self.logic.relationships.push(LogicRelationship {
            source: source_id,
            target: target_id,
            relation_type,
            label,
            stereotype,
            source_multiplicity: None,
            target_multiplicity: None,
            source_role: None,
            target_role: None,
        });

        Ok(())
    }
}

impl DiagramResolver for ClassResolver {
    type Document = ClassUmlFile;
    type Statement = ();
    type Output = ClassDiagram;
    type Error = ClassResolverError;

    fn visit_document(&mut self, document: &Self::Document) -> Result<Self::Output, Self::Error> {
        self.name_map.clear();

        self.logic.name = document.name.clone();
        self.logic.source_files.push(document.name.clone());

        self.analyze(document)?;

        let logic_class = std::mem::replace(
            &mut self.logic,
            ClassDiagram {
                name: String::new(),
                entities: Vec::new(),
                containers: Vec::new(),
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
            attributes: vec![],
            methods: vec![],
        })
    }

    fn make_enum(name: &str, items: Vec<&str>) -> Element {
        Element::EnumDef(EnumDef {
            name: make_name(name),
            namespace: "".to_string(),
            package: "".to_string(),
            stereotypes: vec![],
            items: items
                .into_iter()
                .map(|n| EnumItem {
                    name: n.to_string(),
                    visibility: Some(ParserVisibility::Public),
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
        assert_eq!(entity.name.as_deref(), Some("User"));
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
            Case {
                arrow: make_arrow(None, "--", None),
                expected_ty: RelationType::Link,
                expected_reversed: false,
            },
            Case {
                arrow: make_arrow(None, "..", None),
                expected_ty: RelationType::DashedLink,
                expected_reversed: false,
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

        let arrow = make_arrow(Some("?"), "~~", Some("?"));

        let result = resolver.convert_arrow(&arrow);

        assert!(result.is_err());
    }

    // ----------------------------
    // relationship
    // ----------------------------
    #[test]
    fn test_process_relationship_inheritance() {
        let mut resolver = ClassResolver::new();

        resolver.process_element(&make_class("A"), None);
        resolver.process_element(&make_class("B"), None);

        let rel = Relationship {
            left: "A".to_string(),
            right: "B".to_string(),
            arrow: make_arrow(Some("<|"), "--", None),
            label: Some("<<label>>".to_string()),
        };

        resolver.process_relationship(&rel, None).unwrap();

        assert_eq!(resolver.logic.relationships.len(), 1);

        let r = &resolver.logic.relationships[0];
        assert_eq!(r.source, "B");
        assert_eq!(r.target, "A");
        assert_eq!(r.relation_type, RelationType::Inheritance);
        assert_eq!(r.label, None);
        assert_eq!(r.stereotype, Some("label".to_string()));
    }

    #[test]
    fn test_process_relationship_unresolved_left() {
        let mut resolver = ClassResolver::new();

        let rel = Relationship {
            left: "UnknownA".to_string(),
            right: "KnownB".to_string(),
            arrow: make_arrow(None, "--", Some(">")),
            label: None,
        };

        let result = resolver.process_relationship(&rel, None);

        assert!(matches!(
            result,
            Err(ClassResolverError::UnresolvedReference { ref reference }) if reference == "UnknownA"
        ));
    }

    // ----------------------------
    // namespace
    // ----------------------------
    #[test]
    fn test_process_namespace() {
        let mut resolver = ClassResolver::new();

        let ns = Namespace {
            name: make_name("core"),
            types: vec![make_class("User")],
            namespaces: vec![],
        };

        resolver.process_namespace(&ns, None).unwrap();

        assert_eq!(resolver.logic.containers.len(), 1);
        assert_eq!(resolver.logic.entities.len(), 1);

        let entity = &resolver.logic.entities[0];
        assert_eq!(entity.id, "core.User");
    }

    // ----------------------------
    // visit_document integration
    // ----------------------------
    #[test]
    fn test_visit_document_simple() {
        let mut resolver = ClassResolver::new();

        let file = ClassUmlFile {
            name: "test".to_string(),
            elements: vec![ClassUmlTopLevel::Types(make_class("User"))],
            relationships: vec![],
        };

        let logic = resolver.visit_document(&file).unwrap();
        assert_eq!(logic.name, "test");
        assert_eq!(logic.entities.len(), 1);
        assert_eq!(logic.entities[0].id, "User");
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
                items: vec![],
                stereotypes: vec![],
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
