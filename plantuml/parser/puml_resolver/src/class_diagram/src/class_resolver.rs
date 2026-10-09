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
use std::collections::{HashMap, HashSet};

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
use uid_normalization::{
    identity_name, is_identifier_path, leaf_key, strip_root_marker, DeclarationScope, IdentityKind,
    InternalScope, Resolution,
};

#[derive(Debug, Error)]
pub enum ClassPumlResolverError {
    #[error("Class Resolver: Unresolved reference: {reference}")]
    UnresolvedReference { reference: String },

    #[error(
        "Class Resolver: {reference} is the name of an element declared with alias {alias}; \
         refer to it by the alias"
    )]
    NameOfAliasedElement { reference: String, alias: String },

    #[error("Duplicate entity id: {entity_id} (line {line})", line = source_location.line)]
    DuplicateEntity {
        entity_id: String,
        source_location: SourceLocation,
    },

    #[error("Duplicate alias: {alias} (line {line})", line = source_location.line)]
    DuplicateAlias {
        alias: String,
        source_location: SourceLocation,
    },

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

/// An entity as Rule C sees it.
struct Declared {
    id: String,
    line: u32,
}

pub struct ClassResolver {
    pub logic: ClassDiagram,
    // reference path -> declaration; also the reference existence check
    declared: HashMap<String, Declared>,
    // entity leaf (last reference segment) -> every reference path registered under it
    name_map: HashMap<String, Vec<String>>,
    ids: HashSet<String>,
    // reference path of every package and namespace -> its id path
    scopes: HashMap<String, String>,
    // id path of every aliased entity, package and namespace -> its alias
    aliased: HashMap<String, String>,
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
                free_functions: Vec::new(),
            },
            declared: HashMap::new(),
            name_map: HashMap::new(),
            ids: HashSet::new(),
            scopes: HashMap::new(),
            aliased: HashMap::new(),
        }
    }

    fn analyze(&mut self, file: &ClassUmlFile) -> Result<(), ClassPumlResolverError> {
        let root = DeclarationScope::default();

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

    /// Scope of the declaration `name` under `scope`, and its identity text
    /// (Rule A'): the name decides the id, the alias only the reference path.
    fn declare(
        name: &Name,
        kind: IdentityKind,
        scope: &DeclarationScope,
        source_location: Option<&SourceLocation>,
    ) -> Result<(DeclarationScope, String), ClassPumlResolverError> {
        let text = identity_name(&name.internal, kind).map_err(|error| {
            ClassPumlResolverError::InvalidIdentifier {
                name: name.internal.clone(),
                reason: error.reason().to_string(),
                source_location: source_location.cloned(),
            }
        })?;

        Ok((scope.declare(&text, name.alias.as_deref()), text))
    }

    fn enclosing_namespace_id(&self, declared: &DeclarationScope) -> Option<String> {
        let parent = declared.id.parent();
        (!parent.is_empty()).then(|| parent.id())
    }

    fn declared_before(&self, reference_path: &str, reference_line: u32) -> bool {
        self.declared
            .get(reference_path)
            .is_some_and(|declared| declared.line < reference_line)
    }

    fn leaf_candidates(&self, leaf: &str, reference_line: u32) -> Vec<String> {
        self.name_map
            .get(leaf)
            .into_iter()
            .flatten()
            .filter(|path| self.declared_before(path, reference_line))
            .cloned()
            .collect()
    }

    fn push_entity(
        &mut self,
        entity: SimpleEntity,
        declared: &DeclarationScope,
        alias: Option<&str>,
    ) -> Result<(), ClassPumlResolverError> {
        let reference_path = declared.reference.id();

        if self.ids.contains(&entity.id) {
            return Err(ClassPumlResolverError::DuplicateEntity {
                entity_id: entity.id,
                source_location: entity.source_location,
            });
        }

        if self.declared.contains_key(&reference_path) {
            return Err(ClassPumlResolverError::DuplicateAlias {
                alias: leaf_key(&reference_path),
                source_location: entity.source_location,
            });
        }

        self.ids.insert(entity.id.clone());
        self.declared.insert(
            reference_path.clone(),
            Declared {
                id: entity.id.clone(),
                line: entity.source_location.line,
            },
        );
        self.name_map
            .entry(leaf_key(&reference_path))
            .or_default()
            .push(reference_path);
        if let Some(alias) = alias {
            self.aliased.insert(entity.id.clone(), alias.to_string());
        }
        self.logic.entities.push(entity);
        Ok(())
    }

    /// Rule C lookup over reference paths; sees entities declared earlier,
    /// plus the scope-local hit. Yields the entity id.
    fn resolve_reference(
        &self,
        raw: &str,
        scope: &DeclarationScope,
        reference_line: u32,
    ) -> Result<String, ClassPumlResolverError> {
        let resolution = uid_normalization::resolve_reference(
            &scope.reference,
            raw,
            |path| self.declared.contains_key(path),
            |path| self.declared_before(path, reference_line),
            |leaf| self.leaf_candidates(leaf, reference_line),
        );

        match resolution {
            Resolution::Resolved(path) => Ok(self.declared[&path].id.clone()),
            Resolution::Unresolved => Err(self.unresolved(raw, scope)),
            Resolution::Ambiguous(paths) => {
                let mut candidates: Vec<String> = paths
                    .iter()
                    .map(|path| self.declared[path].id.clone())
                    .collect();
                candidates.sort();
                Err(ClassPumlResolverError::AmbiguousReference {
                    reference: raw.to_string(),
                    candidates,
                })
            }
        }
    }

    /// `raw` names an aliased element by its name instead of its alias, or
    /// nothing at all. A path also hides its name behind an aliased package
    /// or namespace.
    fn unresolved(&self, raw: &str, scope: &DeclarationScope) -> ClassPumlResolverError {
        let through_scopes = is_identifier_path(raw);
        let hidden = |id: &str| {
            self.ids.contains(id)
                && if through_scopes {
                    self.alias_on_path(id).is_some()
                } else {
                    self.aliased.contains_key(id)
                }
        };
        let by_name =
            uid_normalization::resolve_reference(&scope.id, raw, hidden, hidden, |leaf| {
                self.ids
                    .iter()
                    .filter(|id| leaf_key(id) == leaf && self.aliased.contains_key(*id))
                    .cloned()
                    .collect()
            });

        match by_name {
            Resolution::Resolved(id) => ClassPumlResolverError::NameOfAliasedElement {
                reference: raw.to_string(),
                alias: self.alias_on_path(&id).cloned().unwrap_or_default(),
            },
            _ => ClassPumlResolverError::UnresolvedReference {
                reference: raw.to_string(),
            },
        }
    }

    /// Alias of the nearest aliased element on `id`'s path, `id` included.
    fn alias_on_path(&self, id: &str) -> Option<&String> {
        let mut scope = InternalScope::from_path(id);
        loop {
            if let Some(alias) = self.aliased.get(&scope.id()) {
                return Some(alias);
            }
            if scope.is_empty() {
                return None;
            }
            scope = scope.parent();
        }
    }

    /// Records a package or namespace; two with the same reference path but
    /// different ids share an alias.
    fn register_scope(
        &mut self,
        declared: &DeclarationScope,
        name: &Name,
        source_location: &SourceLocation,
    ) -> Result<(), ClassPumlResolverError> {
        let reference_path = declared.reference.id();
        let id = declared.id.id();

        match self.scopes.get(&reference_path) {
            Some(existing) if *existing != id => {
                return Err(ClassPumlResolverError::DuplicateAlias {
                    alias: leaf_key(&reference_path),
                    source_location: source_location.clone(),
                });
            }
            Some(_) => {}
            None => {
                self.scopes.insert(reference_path, id.clone());
            }
        }

        if let Some(alias) = &name.alias {
            self.aliased.insert(id, alias.clone());
        }
        Ok(())
    }

    fn process_top_level(
        &mut self,
        elem: &ClassUmlTopLevel,
        scope: &DeclarationScope,
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
        scope: &DeclarationScope,
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
        scope: &DeclarationScope,
    ) -> Result<(), ClassPumlResolverError> {
        let (nested, _) = Self::declare(
            &pkg.name,
            IdentityKind::Other,
            scope,
            Some(&pkg.source_location),
        )?;
        self.register_scope(&nested, &pkg.name, &pkg.source_location)?;

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
        scope: &DeclarationScope,
    ) -> Result<(), ClassPumlResolverError> {
        let (nested, _) = Self::declare(
            &ns.name,
            IdentityKind::Other,
            scope,
            Some(&ns.source_location),
        )?;
        self.register_scope(&nested, &ns.name, &ns.source_location)?;

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
        scope: &DeclarationScope,
    ) -> Result<(), ClassPumlResolverError> {
        let (nested, _) = Self::declare(
            &pkg.name,
            IdentityKind::Other,
            scope,
            Some(&pkg.source_location),
        )?;

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
        scope: &DeclarationScope,
    ) -> Result<(), ClassPumlResolverError> {
        let (nested, _) = Self::declare(
            &ns.name,
            IdentityKind::Other,
            scope,
            Some(&ns.source_location),
        )?;

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
        scope: &DeclarationScope,
    ) -> Result<(), ClassPumlResolverError> {
        match element {
            Element::ClassDef(def) => {
                self.process_declared_relationships(
                    &def.name,
                    &def.extends,
                    scope,
                    RelationType::Inheritance,
                    &def.source_location,
                )?;
                self.process_declared_relationships(
                    &def.name,
                    &def.implements,
                    scope,
                    RelationType::Implementation,
                    &def.source_location,
                )?;
            }
            Element::InterfaceDef(def) => {
                self.process_declared_relationships(
                    &def.name,
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
        source_name: &Name,
        targets: &[String],
        scope: &DeclarationScope,
        relation_type: RelationType,
        source_location: &SourceLocation,
    ) -> Result<(), ClassPumlResolverError> {
        if targets.is_empty() {
            return Ok(());
        }

        let (declared, _) = Self::declare(
            source_name,
            IdentityKind::ClassLike,
            scope,
            Some(source_location),
        )?;
        let source = declared.id.id();

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
        scope: &DeclarationScope,
    ) -> Result<(), ClassPumlResolverError> {
        match element {
            Element::EnumDef(def) => self.process_enum(def, scope),
            Element::EntityDef(def) if self.is_function_entity(def) => {
                self.process_free_function_entity(def, scope)
            }
            Element::EntityDef(_) => Ok(()),
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

    fn is_function_entity(&self, def: &class_parser::EntityDef) -> bool {
        def.stereotypes
            .iter()
            .any(|stereotype| stereotype.eq_ignore_ascii_case("function"))
    }

    fn process_free_function_entity(
        &mut self,
        def: &class_parser::EntityDef,
        scope: &DeclarationScope,
    ) -> Result<(), ClassPumlResolverError> {
        let (declared, _) = Self::declare(
            &def.name,
            IdentityKind::ClassLike,
            scope,
            Some(&def.source_location),
        )?;
        let enclosing_namespace_id = self.enclosing_namespace_id(&declared);

        self.logic
            .free_functions
            .extend(def.methods.iter().map(|method| FreeFunctionDecl {
                name: method.name.clone(),
                enclosing_namespace_id: enclosing_namespace_id.clone(),
                return_type: method.r#type.clone(),
                parameters: method.params.iter().map(Self::convert_param).collect(),
                template_parameters: Self::convert_template_parameters_with_pack_expansions(
                    &method.template_parameters,
                    &method.params,
                ),
                source_location: method.source_location.clone(),
            }));

        Ok(())
    }

    fn process_class(
        &mut self,
        def: &Element,
        scope: &DeclarationScope,
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
            Element::EntityDef(_) => {
                unreachable!("EntityDef should not be passed to process_class")
            }
            Element::EnumDef(_) => {
                unreachable!("EnumDef should not be passed to process_class")
            }
        };

        let (declared, text) =
            Self::declare(name, IdentityKind::ClassLike, scope, Some(source_location))?;
        let id = declared.id.id();
        let owner_name = leaf_key(&text);

        let template_parameters =
            Self::convert_class_template_parameters(template_parameters, methods);

        let entity = SimpleEntity {
            id: id.clone(),
            name: strip_root_marker(name.alias.as_deref().unwrap_or(&name.internal)).to_string(),
            enclosing_namespace_id: self.enclosing_namespace_id(&declared),
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

        self.push_entity(entity, &declared, name.alias.as_deref())
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
        scope: &DeclarationScope,
    ) -> Result<(), ClassPumlResolverError> {
        let (declared, _) = Self::declare(
            &def.name,
            IdentityKind::ClassLike,
            scope,
            Some(&def.source_location),
        )?;
        let id = declared.id.id();

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
            name: strip_root_marker(def.name.alias.as_deref().unwrap_or(&def.name.internal))
                .to_string(),
            enclosing_namespace_id: self.enclosing_namespace_id(&declared),
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

        self.push_entity(entity, &declared, def.name.alias.as_deref())
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
        scope: &DeclarationScope,
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
        self.declared.clear();
        self.ids.clear();
        self.aliased.clear();

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
            alias: None,
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
            def.name.alias = Some(alias.to_string());
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
    // process_class
    // ----------------------------
    #[test]
    fn test_process_class() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&make_class("User"), &DeclarationScope::default())
            .unwrap();
        assert_eq!(resolver.logic.entities.len(), 1);

        let entity = &resolver.logic.entities[0];
        assert_eq!(entity.id, "User");
        assert_eq!(entity.name, "User");
        assert_eq!(entity.entity_type, EntityType::Class);
    }

    #[test]
    fn test_process_class_uses_name_for_id_leaf() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(
                &make_aliased_class("Foo", "F"),
                &DeclarationScope::from_path("pkg"),
            )
            .unwrap();

        let entity = &resolver.logic.entities[0];
        assert_eq!(entity.id, "pkg.Foo");
        assert_eq!(entity.name, "F");
        assert_eq!(entity.enclosing_namespace_id.as_deref(), Some("pkg"));
    }

    #[test]
    fn test_process_class_qualified_name_with_alias() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(
                &make_aliased_class("score::x::Y", "Y"),
                &DeclarationScope::from_path("p"),
            )
            .unwrap();

        let entity = &resolver.logic.entities[0];
        assert_eq!(entity.id, "p.score.x.Y");
        assert_eq!(entity.name, "Y");
        assert_eq!(entity.enclosing_namespace_id.as_deref(), Some("p.score.x"));
    }

    #[test]
    fn test_process_class_template_arguments_are_not_part_of_the_name() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(
                &make_aliased_class("Proxy<Spec...>", "P"),
                &DeclarationScope::default(),
            )
            .unwrap();

        assert_eq!(resolver.logic.entities[0].id, "Proxy");
    }

    #[test]
    fn test_process_class_prose_name_is_invalid() {
        let mut resolver = ClassResolver::new();
        let result = resolver.process_element(
            &make_aliased_class("Some prose", "P"),
            &DeclarationScope::default(),
        );

        assert!(matches!(
            result,
            Err(ClassPumlResolverError::InvalidIdentifier { ref name, .. }) if name == "Some prose"
        ));
    }

    #[test]
    fn test_constructor_of_aliased_class_is_recognized() {
        let mut element = make_aliased_class("score::Foo", "F");
        if let Element::ClassDef(def) = &mut element {
            def.methods.push(ParserMethod {
                name: "Foo".to_string(),
                ..Default::default()
            });
        }

        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&element, &DeclarationScope::default())
            .unwrap();

        assert!(resolver.logic.entities[0].methods[0]
            .modifiers
            .contains(&MethodModifier::Constructor));
    }

    #[test]
    fn test_duplicate_entity_ids_are_rejected() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&make_class("User"), &DeclarationScope::default())
            .unwrap();

        let result = resolver.process_element(&make_class("User"), &DeclarationScope::default());

        assert!(matches!(
            result,
            Err(ClassPumlResolverError::DuplicateEntity { ref entity_id, .. }) if entity_id == "User"
        ));
    }

    #[test]
    fn test_duplicate_alias_is_rejected() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(
                &make_aliased_class("a::Foo", "F"),
                &DeclarationScope::default(),
            )
            .unwrap();

        let result = resolver.process_element(
            &make_aliased_class("b::Bar", "F"),
            &DeclarationScope::default(),
        );

        assert!(matches!(
            result,
            Err(ClassPumlResolverError::DuplicateAlias { ref alias, .. }) if alias == "F"
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
                &DeclarationScope::default(),
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
        let scope = DeclarationScope::from_path("core");
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
                &DeclarationScope::from_path("core::geometry"),
            )
            .unwrap();

        let resolved = resolver
            .resolve_reference("core::geometry::User", &DeclarationScope::default(), 100)
            .unwrap();

        assert_eq!(resolved, "core.geometry.User");
    }

    #[test]
    fn test_resolve_reference_falls_back_to_unique_leaf() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&make_class("User"), &DeclarationScope::from_path("core"))
            .unwrap();

        let resolved = resolver
            .resolve_reference("User", &DeclarationScope::from_path("other"), 100)
            .unwrap();

        assert_eq!(resolved, "core.User");
    }

    #[test]
    fn test_resolve_reference_several_leaves_is_ambiguous() {
        let mut resolver = ClassResolver::new();
        resolver
            .process_element(&make_class("User"), &DeclarationScope::from_path("p"))
            .unwrap();
        resolver
            .process_element(&make_class("User"), &DeclarationScope::from_path("q"))
            .unwrap();

        let result = resolver.resolve_reference("User", &DeclarationScope::default(), 100);

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
            .process_element(
                &make_class_at("User", 10),
                &DeclarationScope::from_path("p"),
            )
            .unwrap();

        let too_early = resolver.resolve_reference("User", &DeclarationScope::default(), 5);
        assert!(matches!(
            too_early,
            Err(ClassPumlResolverError::UnresolvedReference { .. })
        ));

        let resolved = resolver
            .resolve_reference("User", &DeclarationScope::default(), 20)
            .unwrap();
        assert_eq!(resolved, "p.User");
    }

    #[test]
    fn test_resolve_reference_direct_scope_hit_ignores_declaration_order() {
        let mut resolver = ClassResolver::new();
        let scope = DeclarationScope::from_path("p");
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
            .process_element(&make_class("X"), &DeclarationScope::default())
            .unwrap();
        resolver
            .process_element(&make_class("X"), &DeclarationScope::from_path("a"))
            .unwrap();

        let resolved = resolver
            .resolve_reference(".X", &DeclarationScope::from_path("a"), 100)
            .unwrap();
        assert_eq!(resolved, "X");
    }

    #[test]
    fn test_resolve_reference_uses_the_alias_and_yields_the_id() {
        let mut resolver = ClassResolver::new();
        let scope = DeclarationScope::from_path("p");
        resolver
            .process_element(&make_aliased_class("score::x::Y", "Y"), &scope)
            .unwrap();

        assert_eq!(
            resolver.resolve_reference("Y", &scope, 100).unwrap(),
            "p.score.x.Y"
        );
        assert_eq!(
            resolver
                .resolve_reference("p::Y", &DeclarationScope::default(), 100)
                .unwrap(),
            "p.score.x.Y"
        );
    }

    #[test]
    fn test_resolve_reference_by_name_of_aliased_element_is_reported() {
        let mut resolver = ClassResolver::new();
        let scope = DeclarationScope::from_path("p");
        resolver
            .process_element(&make_aliased_class("score::x::Y", "Z"), &scope)
            .unwrap();

        for raw in ["Y", "score::x::Y"] {
            let result = resolver.resolve_reference(raw, &scope, 100);
            assert!(
                matches!(
                    result,
                    Err(ClassPumlResolverError::NameOfAliasedElement { ref alias, .. }) if alias == "Z"
                ),
                "{raw}"
            );
        }
    }

    // ----------------------------
    // package alias
    // ----------------------------
    fn make_package(name: &str, alias: Option<&str>, types: Vec<Element>) -> Package {
        let mut package_name = make_name(name);
        package_name.alias = alias.map(str::to_string);
        Package {
            name: package_name,
            source_location: SourceLocation::new("test.puml", 3),
            types,
            relationships: vec![],
            packages: vec![],
        }
    }

    #[test]
    fn test_duplicate_package_alias_is_rejected() {
        let mut resolver = ClassResolver::new();
        let root = DeclarationScope::default();
        resolver
            .process_package(&make_package("a", Some("X"), vec![]), &root)
            .unwrap();

        let result = resolver.process_package(&make_package("b", Some("X"), vec![]), &root);

        assert!(matches!(
            result,
            Err(ClassPumlResolverError::DuplicateAlias { ref alias, ref source_location })
                if alias == "X" && source_location.line == 3
        ));
    }

    #[test]
    fn test_reopened_package_is_not_a_duplicate() {
        let mut resolver = ClassResolver::new();
        let root = DeclarationScope::default();

        resolver
            .process_package(&make_package("a", Some("X"), vec![]), &root)
            .unwrap();
        resolver
            .process_package(&make_package("a", Some("X"), vec![]), &root)
            .unwrap();
    }

    #[test]
    fn test_name_of_aliased_package_is_reported() {
        let mut resolver = ClassResolver::new();
        let root = DeclarationScope::default();
        let package = make_package("a::b", Some("B"), vec![make_class("C")]);
        resolver.process_package(&package, &root).unwrap();

        assert_eq!(
            resolver.resolve_reference("B::C", &root, 100).unwrap(),
            "a.b.C"
        );
        assert!(matches!(
            resolver.resolve_reference("a::b::C", &root, 100),
            Err(ClassPumlResolverError::NameOfAliasedElement { ref alias, .. }) if alias == "B"
        ));
    }

    #[test]
    fn test_forward_reference_into_an_aliased_package_is_unresolved() {
        let mut resolver = ClassResolver::new();
        let root = DeclarationScope::default();
        let package = make_package("a", Some("A"), vec![make_class_at("C", 10)]);
        resolver.process_package(&package, &root).unwrap();

        assert!(matches!(
            resolver.resolve_reference("C", &root, 5),
            Err(ClassPumlResolverError::UnresolvedReference { .. })
        ));
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
            .process_element(&make_class("A"), &DeclarationScope::default())
            .unwrap();
        resolver
            .process_element(&make_class("B"), &DeclarationScope::default())
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
            .process_relationship(&rel, &DeclarationScope::default())
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

        let result = resolver.process_relationship(&rel, &DeclarationScope::default());

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
            source_location: SourceLocation::new("test.puml", 1),
            types: vec![make_class("User")],
            relationships: vec![],
            namespaces: vec![],
        };

        resolver
            .process_namespace(&ns, &DeclarationScope::default())
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
                source_location: SourceLocation::new("test.puml", 1),
                types: vec![],
                relationships: vec![],
                namespaces: vec![],
            }),
        ];

        for case in cases {
            let mut resolver = ClassResolver::new();
            assert!(resolver
                .process_top_level(&case, &DeclarationScope::default())
                .is_ok());
        }
    }
}
