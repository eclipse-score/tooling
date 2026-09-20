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

use clang::{Entity, EntityKind};

use class_diagram::{
    EntityType, MemberVariable, Method, MethodModifier, SimpleEntity, TypeAlias, Visibility,
};

use crate::callable_declaration::parse_template_parameters;
use crate::clang_adapter::scope::{namespace_id, semantic_parent_id};
use crate::clang_adapter::source_location::parse_source_location;
use crate::context::{
    ExtractedMethodDeclaration, ParsedBaseClass, ParsedClassInfo, ParsedVariableType, VisitContext,
};
use crate::types::renderer::render_type_for_display;
use crate::types::resolver::resolve_type;
use crate::visitor::AstVisitor;

pub struct ClassVisitor;
impl AstVisitor for ClassVisitor {
    fn visit(ctx: &mut VisitContext, entity: Entity) {
        let template_params = match entity.get_kind() {
            EntityKind::ClassTemplate | EntityKind::ClassTemplatePartialSpecialization => {
                parse_template_parameters(&entity)
            }
            _ => None,
        };

        let namespace = namespace_id(&entity);
        let semantic_parent = semantic_parent_id(&entity);

        if let Some((builder, mut class_entity)) =
            Self::visit_class(&entity, semantic_parent.as_deref(), namespace.as_deref())
        {
            class_entity.template_parameters = template_params;
            ctx.parsed_class_info.insert(builder.id.clone(), builder);
            ctx.types.insert(class_entity.id.clone(), class_entity);
        }
    }
}

impl ClassVisitor {
    /// Compatibility entry point for callers that previously invoked the class visitor's
    /// relationship phase directly.
    pub fn resolve_relationships(ctx: &mut VisitContext) {
        crate::class_relationship_resolver::resolve_relationships(ctx);
    }

    /// Adds a callable declaration to its owning class and preserves the
    /// class-level metadata used by relationship inference.
    pub(crate) fn add_method_declaration(
        ctx: &mut VisitContext,
        declaration: ExtractedMethodDeclaration,
    ) {
        let (types, parsed_class_info) = (&mut ctx.types, &mut ctx.parsed_class_info);
        let (Some(class), Some(builder)) = (
            types.get_mut(&declaration.class_id),
            parsed_class_info.get_mut(&declaration.class_id),
        ) else {
            log::warn!(
                "method '{}' has incompletely registered owning class '{}'; skipping declaration",
                declaration.method.name,
                declaration.class_id
            );
            return;
        };

        update_entity_type_for_method(class, builder, &declaration.method);
        class.methods.push(declaration.method);
        builder.method_types.push(declaration.method_type);
    }

    fn visit_class(
        entity: &Entity,
        semantic_parent: Option<&str>,
        namespace: Option<&str>,
    ) -> Option<(ParsedClassInfo, SimpleEntity)> {
        let Some(name) = entity.get_name() else {
            log::debug!("skipping class/struct: anonymous type has no name");
            return None;
        };

        let id = class_entity_id(entity, semantic_parent, &name);

        let mut builder = ParsedClassInfo {
            id: id.clone(),
            base_classes: vec![],
            variable_types: vec![],
            method_types: vec![],
            has_abstract_methods: false,
            has_concrete_methods: false,
        };

        let mut class_entity = SimpleEntity {
            id,
            name: name.clone(),
            enclosing_namespace_id: namespace.map(|ns| ns.to_string()),
            ..Default::default()
        };

        // Note: nested class/struct shall be parsed by `visit_recursive` in visitor.rs file, not here.
        for child in entity.get_children() {
            Self::visit_member(&child, &mut class_entity, &mut builder);
        }

        if entity.get_kind() == EntityKind::StructDecl {
            class_entity.entity_type = EntityType::Struct;
        }

        class_entity.source_location = parse_source_location(entity);

        Some((builder, class_entity))
    }

    fn visit_member(entity: &Entity, class: &mut SimpleEntity, builder: &mut ParsedClassInfo) {
        match entity.get_kind() {
            EntityKind::BaseSpecifier => {
                if let Some(base_type) = entity.get_type() {
                    builder.base_classes.push(ParsedBaseClass {
                        resolved_type: resolve_type(&base_type),
                        source_location: parse_source_location(entity),
                    });
                }
            }
            EntityKind::FieldDecl | EntityKind::VarDecl => {
                let Some(parsed_variable_type) = collect_variable_type(entity) else {
                    return;
                };
                builder.variable_types.push(parsed_variable_type.clone());

                if let Some(variable) = parse_variable(entity, &parsed_variable_type) {
                    class.variables.push(variable);
                }
            }
            // `using Alias = OriginalType;` -> TypeAliasDecl
            // `typedef OriginalType Alias;` -> TypedefDecl
            EntityKind::TypeAliasDecl | EntityKind::TypedefDecl => {
                if let Some(type_alias) = parse_type_alias(entity) {
                    class.type_aliases.push(type_alias);
                }
            }
            _ => {}
        }
    }
}

fn class_entity_id(entity: &Entity, namespace: Option<&str>, name: &str) -> String {
    let base_name = if entity.get_kind() == EntityKind::ClassTemplatePartialSpecialization {
        entity
            .get_display_name()
            .unwrap_or_else(|| name.to_string())
    } else {
        name.to_string()
    };

    match namespace {
        Some(ns) if !ns.is_empty() => format!("{ns}::{base_name}"),
        _ => base_name,
    }
}

fn collect_variable_type(entity: &Entity) -> Option<ParsedVariableType> {
    let Some(name) = entity.get_name() else {
        log::debug!("skipping field/variable: entity has no name");
        return None;
    };
    let Some(field_type) = entity.get_type() else {
        log::debug!(
            "skipping field/variable '{}': could not determine its type",
            name
        );
        return None;
    };

    Some(ParsedVariableType {
        name,
        resolved_type: resolve_type(&field_type),
        source_location: parse_source_location(entity),
    })
}

fn parse_type_alias(entity: &Entity) -> Option<TypeAlias> {
    let Some(alias) = entity.get_name() else {
        log::debug!("skipping type alias: entity has no name");
        return None;
    };

    let Some(original_type) = entity
        .get_typedef_underlying_type()
        .map(|t| render_type_for_display(&t, &resolve_type(&t)))
    else {
        log::debug!(
            "skipping type alias '{}': could not determine underlying type",
            alias
        );
        return None;
    };

    Some(TypeAlias {
        alias,
        original_type,
        source_location: parse_source_location(entity),
    })
}

fn parse_variable(
    entity: &Entity,
    parsed_variable_type: &ParsedVariableType,
) -> Option<MemberVariable> {
    Some(MemberVariable {
        name: parsed_variable_type.name.clone(),
        data_type: entity.get_type().map(|field_type| {
            render_type_for_display(&field_type, &parsed_variable_type.resolved_type)
        }),
        visibility: parse_visibility(entity),
        is_static: entity.get_kind() == EntityKind::VarDecl,
        source_location: parse_source_location(entity),
    })
}

pub(crate) fn parse_visibility(entity: &Entity) -> Visibility {
    match entity.get_accessibility() {
        Some(clang::Accessibility::Public) => Visibility::Public,
        Some(clang::Accessibility::Private) => Visibility::Private,
        Some(clang::Accessibility::Protected) => Visibility::Protected,
        _ => Visibility::Public,
    }
}

fn update_entity_type_for_method(
    class: &mut SimpleEntity,
    builder: &mut ParsedClassInfo,
    method: &Method,
) {
    if class.entity_type == EntityType::Struct {
        return;
    }

    update_method_flags(builder, method);

    class.entity_type = match (
        builder.has_abstract_methods,
        builder.has_concrete_methods,
        class.variables.is_empty(),
    ) {
        (true, false, true) => EntityType::Interface,
        (true, _, _) => EntityType::AbstractClass,
        _ => EntityType::Class,
    };
}

fn update_method_flags(builder: &mut ParsedClassInfo, method: &Method) {
    let is_abstract = method
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier, MethodModifier::Abstract));

    let is_special_method = method.modifiers.iter().any(|modifier| {
        matches!(
            modifier,
            MethodModifier::Constructor | MethodModifier::Destructor
        )
    });

    if is_abstract {
        builder.has_abstract_methods = true;
    } else if !is_special_method {
        builder.has_concrete_methods = true;
    }
}
