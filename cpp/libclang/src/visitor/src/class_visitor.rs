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
    EntityType, FunctionArgument, MemberVariable, Method, MethodModifier, SimpleEntity,
    TemplateParameter, TypeAlias, Visibility,
};
use cpp_semantics::ResolvedType;

use crate::clang_adapter::scope::{namespace_id, semantic_parent_id};
use crate::clang_adapter::source_location::parse_source_location;
use crate::context::{
    ParsedBaseClass, ParsedClassInfo, ParsedMethodType, ParsedVariableType, VisitContext,
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
            ctx.parsed_class_info.push(builder);
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

        class_entity.entity_type = infer_entity_type_from_members(entity.get_kind(), &class_entity);

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
            EntityKind::Method | EntityKind::Constructor | EntityKind::Destructor => {
                let parsed_method_type = collect_method_type(entity, builder);
                if let Some(method) = parse_method(entity, &parsed_method_type) {
                    class.methods.push(method);
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
            EntityKind::FunctionTemplate => {
                let template_params = parse_template_parameters(entity);
                let parsed_method_type = collect_method_type(entity, builder);

                // In current libclang/clang-rs output, method templates are represented
                // directly on the FunctionTemplate entity.
                if let Some(mut method) = parse_method(entity, &parsed_method_type) {
                    method.template_parameters = template_params;
                    class.methods.push(method);
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

fn collect_method_type(entity: &Entity, builder: &mut ParsedClassInfo) -> ParsedMethodType {
    let name = entity.get_name().unwrap_or_default();

    let return_type = entity
        .get_result_type()
        .map(|t| resolve_type(&t))
        .unwrap_or_else(|| ResolvedType::Builtin("void".to_string()));
    let parameter_types = method_arguments(entity)
        .into_iter()
        .filter_map(|arg| arg.get_type().map(|t| resolve_type(&t)))
        .collect();

    let parsed_method_type = ParsedMethodType {
        name,
        return_type,
        parameter_types,
        source_location: parse_source_location(entity),
    };
    builder.method_types.push(parsed_method_type.clone());

    parsed_method_type
}

/// Normally libclang provides the parameter list via `Entity::get_arguments()`.
/// However, for some cursor kinds (e.g. `FunctionTemplate`) or certain libclang
/// versions, `get_arguments()` may return `None` even though the AST still
/// contains `ParmDecl` child cursors.
fn method_arguments<'tu>(entity: &Entity<'tu>) -> Vec<Entity<'tu>> {
    entity.get_arguments().unwrap_or_else(|| {
        // fall back to collecting all direct `ParmDecl` children from
        // the cursor to recover the parameter list.
        entity
            .get_children()
            .into_iter()
            .filter(|child| child.get_kind() == EntityKind::ParmDecl)
            .collect()
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

fn parse_method(entity: &Entity, parsed_method_type: &ParsedMethodType) -> Option<Method> {
    let kind = entity.get_kind();
    let name = entity.get_name()?;
    let is_override_method = entity
        .get_overridden_methods()
        .map(|methods| !methods.is_empty())
        .unwrap_or(false);

    let return_type = if matches!(kind, EntityKind::Constructor | EntityKind::Destructor) {
        None
    } else {
        entity
            .get_result_type()
            .map(|ret| render_type_for_display(&ret, &parsed_method_type.return_type))
    };

    let mut parameters = Vec::new();
    let method_is_variadic = entity.get_type().map(|t| t.is_variadic()).unwrap_or(false);

    let args = method_arguments(entity);

    let arg_count = args.len();
    for (idx, arg) in args.into_iter().enumerate() {
        let raw_param_type = arg
            .get_type()
            .map(|ty| ty.get_display_name())
            .unwrap_or_default();
        let is_pack_expansion = raw_param_type.contains("...");
        let param_type = normalize_pack_expansion_type(&raw_param_type);

        parameters.push(FunctionArgument {
            name: arg.get_name().unwrap_or_default(),
            param_type: Some(param_type),
            is_variadic: method_is_variadic && idx + 1 == arg_count,
            is_pack_expansion,
        });
    }

    Some(Method {
        name,
        return_type,
        visibility: parse_visibility(entity),
        parameters,
        template_parameters: None,
        modifiers: MethodModifier::from_conditions([
            (entity.is_static_method(), MethodModifier::Static),
            (entity.is_virtual_method(), MethodModifier::Virtual),
            (entity.is_pure_virtual_method(), MethodModifier::Abstract),
            (is_override_method, MethodModifier::Override),
            (kind == EntityKind::Constructor, MethodModifier::Constructor),
            (kind == EntityKind::Destructor, MethodModifier::Destructor),
        ]),
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

fn parse_template_parameters(entity: &Entity) -> Option<Vec<TemplateParameter>> {
    let params: Vec<TemplateParameter> = entity
        .get_children()
        .into_iter()
        .enumerate()
        .filter_map(|(idx, child)| match child.get_kind() {
            EntityKind::TemplateTypeParameter => {
                // template <typename Foo>  →  "name: Foo, is_pack: False"
                // template <typename, typename> -> "name: T0, is_pack: False", "name: T1, is_pack: False"
                // template <typename... Foo> -> "name: Foo, is_pack: True"
                let name = child.get_name().unwrap_or_else(|| format!("T{idx}"));

                Some(TemplateParameter::Type {
                    name,
                    is_pack: is_template_parameter_pack(&child),
                })
            }
            EntityKind::NonTypeTemplateParameter => {
                // template <int N>  →  "name: N, value_type: int"
                let type_name = child
                    .get_type()
                    .map(|t| t.get_display_name())
                    .unwrap_or_default();
                let name = child.get_name().unwrap_or_default();

                Some(TemplateParameter::NonType {
                    name,
                    value_type: type_name,
                    is_pack: is_template_parameter_pack(&child),
                })
            }
            EntityKind::TemplateTemplateParameter => {
                // template <template<...> class C>  → "name: C, parameters: [...], is_pack: False"
                let parameters = parse_template_parameters(&child).unwrap_or_default();
                let name = child.get_name().unwrap_or_else(|| format!("T{idx}"));

                Some(TemplateParameter::Template {
                    name,
                    parameters,
                    is_pack: is_template_parameter_pack(&child),
                })
            }
            _ => None,
        })
        .collect();

    if params.is_empty() {
        None
    } else {
        Some(params)
    }
}

fn normalize_pack_expansion_type(param_type: &str) -> String {
    param_type.replace("...", "").trim().to_string()
}

fn is_template_parameter_pack(entity: &Entity) -> bool {
    entity.get_range().is_some_and(|range| {
        range
            .tokenize()
            .iter()
            .any(|token| token.get_spelling() == "...")
    }) || entity
        .get_display_name()
        .as_deref()
        .is_some_and(|display_name| display_name.contains("..."))
}

fn parse_visibility(entity: &Entity) -> Visibility {
    match entity.get_accessibility() {
        Some(clang::Accessibility::Public) => Visibility::Public,
        Some(clang::Accessibility::Private) => Visibility::Private,
        Some(clang::Accessibility::Protected) => Visibility::Protected,
        _ => Visibility::Public,
    }
}

fn infer_entity_type_from_members(kind: EntityKind, class: &SimpleEntity) -> EntityType {
    if kind == EntityKind::StructDecl {
        return EntityType::Struct;
    }

    let has_data_members = !class.variables.is_empty();
    let mut has_abstract_methods = false;
    let mut has_concrete_methods = false;

    for method in &class.methods {
        let is_abstract = method
            .modifiers
            .iter()
            .any(|m| matches!(m, MethodModifier::Abstract));
        let is_constructor_or_destructor = method
            .modifiers
            .iter()
            .any(|m| matches!(m, MethodModifier::Constructor | MethodModifier::Destructor));

        if is_abstract {
            has_abstract_methods = true;
        } else if !is_constructor_or_destructor {
            has_concrete_methods = true;
        }
    }

    if has_abstract_methods {
        if !has_concrete_methods && !has_data_members {
            EntityType::Interface
        } else {
            EntityType::AbstractClass
        }
    } else {
        EntityType::Class
    }
}
