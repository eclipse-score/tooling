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
use std::collections::HashSet;

use class_diagram::{
    EntityType, MemberVariable, Method, MethodModifier, SimpleEntity, TypeAlias, Visibility,
};

use crate::callable_declaration::parse_template_parameters;
use crate::clang_adapter::scope::{namespace_id, semantic_parent_id};
use crate::clang_adapter::source_location::parse_source_location;
use crate::context::{
    CallableDeclarationKey, CallableOwnerKey, ExtractedMethodDeclaration, ParsedBaseClass,
    ParsedClassInfo, ParsedVariableType, VisitContext,
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

    /// Registers a method declaration exactly once, but only if its owning class
    /// has already been registered in the visit context.
    pub(crate) fn register_method_declaration(
        ctx: &mut VisitContext,
        seen_method_declarations: &mut HashSet<CallableDeclarationKey>,
        declaration: ExtractedMethodDeclaration,
    ) -> bool {
        let identity = CallableDeclarationKey {
            owner: CallableOwnerKey::Method {
                class_id: declaration.class_id.clone(),
            },
            signature: declaration.signature_key.clone(),
        };

        if seen_method_declarations.contains(&identity) {
            return false;
        }

        if !Self::attach_method_declaration(ctx, declaration) {
            return false;
        }

        seen_method_declarations.insert(identity)
    }

    /// Adds a callable declaration to its owning class and preserves the
    /// class-level metadata used by relationship inference.
    fn attach_method_declaration(
        ctx: &mut VisitContext,
        declaration: ExtractedMethodDeclaration,
    ) -> bool {
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
            return false;
        };

        update_entity_type_for_method(class, builder, &declaration.method);
        class.methods.push(declaration.method);
        builder.method_types.push(declaration.method_type);
        true
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

#[cfg(test)]
mod tests {
    use super::ClassVisitor;
    use crate::context::{
        CallableDeclarationKey, CallableOwnerKey, CallableSignatureKey, ExtractedMethodDeclaration,
        ParsedClassInfo, ParsedMethodType, VisitContext,
    };
    use class_diagram::{Method, SimpleEntity};
    use cpp_semantics::ResolvedType;
    use std::collections::HashSet;

    #[test]
    fn missing_owning_class_does_not_burn_method_identity() {
        let mut ctx = VisitContext::default();
        let mut seen = HashSet::<CallableDeclarationKey>::new();
        let declaration = method_declaration();

        assert!(!ClassVisitor::register_method_declaration(
            &mut ctx,
            &mut seen,
            declaration,
        ));
        assert!(seen.is_empty());
    }

    #[test]
    fn successful_method_insert_is_still_deduplicated() {
        let mut ctx = VisitContext::default();
        ctx.types.insert(
            "Widget".to_string(),
            SimpleEntity {
                id: "Widget".to_string(),
                name: "Widget".to_string(),
                ..Default::default()
            },
        );
        ctx.parsed_class_info.insert(
            "Widget".to_string(),
            ParsedClassInfo {
                id: "Widget".to_string(),
                ..Default::default()
            },
        );

        let mut seen = HashSet::<CallableDeclarationKey>::new();

        assert!(ClassVisitor::register_method_declaration(
            &mut ctx,
            &mut seen,
            method_declaration(),
        ));
        assert!(!ClassVisitor::register_method_declaration(
            &mut ctx,
            &mut seen,
            method_declaration(),
        ));

        let class = ctx.types.get("Widget").expect("class should exist");
        assert_eq!(class.methods.len(), 1);
        assert_eq!(ctx.parsed_class_info["Widget"].method_types.len(), 1);
        assert_eq!(
            seen,
            HashSet::from([CallableDeclarationKey {
                owner: CallableOwnerKey::Method {
                    class_id: "Widget".to_string(),
                },
                signature: CallableSignatureKey {
                    name: "compute".to_string(),
                    parameters: vec![],
                },
            }])
        );
    }

    fn method_declaration() -> ExtractedMethodDeclaration {
        ExtractedMethodDeclaration {
            class_id: "Widget".to_string(),
            method: Method {
                name: "compute".to_string(),
                parameters: vec![],
                ..Default::default()
            },
            method_type: ParsedMethodType {
                name: "compute".to_string(),
                return_type: ResolvedType::Builtin("void".to_string()),
                parameter_types: vec![],
                source_location: Default::default(),
            },
            signature_key: CallableSignatureKey {
                name: "compute".to_string(),
                parameters: vec![],
            },
        }
    }
}
