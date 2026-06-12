///////////////////////////////////////////////////////////////////////////////////
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
////////////////////////////////////////////////////////////////////////////////////
use clang::{Entity, EntityKind};
use std::collections::HashSet;

use class_diagram::{
    EntityType, FunctionArgument, MemberVariable, Method, MethodModifier, RelationType,
    Relationship, SimpleEntity, TypeAlias, Visibility,
};

use crate::class_parser_helper::{
    render_type_for_display, resolve_type, to_workspace_relative_or_abs_path, ResolvedType,
};
use crate::context::{ParsedClassInfo, ParsedMethodType, ParsedVariableType, VisitContext};
use crate::visitor::AstVisitor;

pub struct ClassVisitor;
impl AstVisitor for ClassVisitor {
    fn visit(ctx: &mut VisitContext, entity: Entity) {
        let template_params = if ctx.is_templated {
            parse_template_parameters(&entity)
        } else {
            None
        };

        let namespace = Self::get_namespace_id(&entity);

        if let Some((builder, mut class_entity)) = Self::visit_class(&entity, namespace.as_deref())
        {
            class_entity.template_parameters = template_params;
            ctx.parsed_class_info.push(builder);
            ctx.types.insert(class_entity.id.clone(), class_entity);
        }
    }
}

impl ClassVisitor {
    pub fn resolve_relationships(ctx: &mut VisitContext) {
        let builders = std::mem::take(&mut ctx.parsed_class_info);
        let known_type_ids: HashSet<String> = ctx.types.keys().cloned().collect();

        for builder in builders {
            build_relationships_for_class(ctx, &builder);
            infer_relationships_from_builder(ctx, &builder, &known_type_ids);
        }
    }

    fn visit_class(
        entity: &Entity,
        namespace: Option<&str>,
    ) -> Option<(ParsedClassInfo, SimpleEntity)> {
        let name = entity.get_name()?;

        let id = match namespace {
            Some(ns) if !ns.is_empty() => format!("{ns}::{name}"),
            _ => name.clone(),
        };

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

        (class_entity.source_file, class_entity.source_line) = parse_source_location(entity);

        Some((builder, class_entity))
    }

    fn visit_member(entity: &Entity, class: &mut SimpleEntity, builder: &mut ParsedClassInfo) {
        match entity.get_kind() {
            EntityKind::BaseSpecifier => {
                if let Some(base_type) = entity.get_type() {
                    builder.base_classes.push(resolve_type(&base_type));
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

fn parse_source_location(entity: &Entity) -> (Option<String>, Option<u32>) {
    let Some(location) = entity.get_location() else {
        return (None, None);
    };

    let file_location = location.get_file_location();
    let source_file = file_location
        .file
        .map(|f| to_workspace_relative_or_abs_path(f.get_path()));
    let source_line = Some(file_location.line);

    (source_file, source_line)
}

fn collect_variable_type(entity: &Entity) -> Option<ParsedVariableType> {
    let name = entity.get_name()?;
    let field_type = entity.get_type()?;

    Some(ParsedVariableType {
        name,
        resolved_type: resolve_type(&field_type),
    })
}

fn collect_method_type(entity: &Entity, builder: &mut ParsedClassInfo) -> ParsedMethodType {
    let name = entity.get_name().unwrap_or_default();

    let return_type = entity
        .get_result_type()
        .map(|t| resolve_type(&t))
        .unwrap_or_else(|| ResolvedType::Builtin("void".to_string()));
    let parameter_types = entity
        .get_arguments()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|arg| arg.get_type().map(|t| resolve_type(&t)))
        .collect();

    let parsed_method_type = ParsedMethodType {
        name,
        return_type,
        parameter_types,
    };
    builder.method_types.push(parsed_method_type.clone());

    parsed_method_type
}

fn parse_type_alias(entity: &Entity) -> Option<TypeAlias> {
    let alias = entity.get_name()?;

    let original_type = entity
        .get_typedef_underlying_type()
        .map(|t| render_type_for_display(&t, &resolve_type(&t)))?;

    Some(TypeAlias {
        alias,
        original_type,
    })
}

fn parse_method(entity: &Entity, parsed_method_type: &ParsedMethodType) -> Option<Method> {
    let kind = entity.get_kind();
    let name = entity.get_name()?;
    let is_override_method = entity
        .get_overridden_methods()
        .map(|methods| !methods.is_empty())
        .unwrap_or(false);

    let return_type = entity
        .get_result_type()
        .map(|ret| render_type_for_display(&ret, &parsed_method_type.return_type))
        .unwrap_or_else(|| "void".to_string());

    let mut parameters = Vec::new();
    let method_is_variadic = entity.get_type().map(|t| t.is_variadic()).unwrap_or(false);

    if let Some(args) = entity.get_arguments() {
        let arg_count = args.len();
        for (idx, arg) in args.into_iter().enumerate() {
            let param_type = arg
                .get_type()
                .map(|ty| ty.get_display_name())
                .unwrap_or_default();

            parameters.push(FunctionArgument {
                name: arg.get_name().unwrap_or_default(),
                param_type: Some(param_type),
                is_variadic: method_is_variadic && idx + 1 == arg_count,
            });
        }
    }

    Some(Method {
        name,
        return_type: Some(return_type),
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
    })
}

fn parse_template_parameters(entity: &Entity) -> Option<Vec<String>> {
    let params: Vec<String> = entity
        .get_children()
        .into_iter()
        .enumerate()
        .filter_map(|(idx, child)| match child.get_kind() {
            EntityKind::TemplateTypeParameter => {
                // template <typename Foo>  →  "Foo"
                // template <typename, typename> -> "T0", "T1"
                Some(child.get_name().unwrap_or_else(|| format!("T{idx}")))
            }
            EntityKind::NonTypeTemplateParameter => {
                // template <int N>  →  "int N"
                let type_name = child
                    .get_type()
                    .map(|t| t.get_display_name())
                    .unwrap_or_default();
                let name = child.get_name().unwrap_or_default();
                Some(format!("{type_name} {name}").trim().to_string())
            }
            EntityKind::TemplateTemplateParameter => {
                // template <template<...> class C>  →  "template<...> C"
                Some(format!(
                    "template<...> {}",
                    child.get_name().unwrap_or_default()
                ))
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

// Relationship part
fn build_relationships_for_class(ctx: &mut VisitContext, builder: &ParsedClassInfo) {
    for base in &builder.base_classes {
        let resolved_base = base.referenced_entity_id().unwrap_or_else(|| {
            panic!(
                "Unresolved base type '{}' referenced by '{}'",
                base.render_for_display(),
                builder.id
            )
        });

        let target_class = ctx.types.get(resolved_base).unwrap_or_else(|| {
            panic!(
                "Resolved base type '{}' missing in type map for '{}'",
                resolved_base, builder.id
            )
        });

        let relation_type = if target_class.entity_type == EntityType::Interface {
            RelationType::Implementation
        } else {
            RelationType::Inheritance
        };

        let class = ctx
            .types
            .get_mut(&builder.id)
            .expect("Source class must exist before building relationships");

        add_relationship(class, resolved_base.to_string(), relation_type);
    }
}

fn add_relationship(class: &mut SimpleEntity, target: String, relation_type: RelationType) {
    if target == class.id {
        return;
    }

    let relationship = Relationship {
        source: class.id.clone(),
        target,
        relation_type,
        source_multiplicity: None,
        target_multiplicity: None,
    };

    if !class.relationships.contains(&relationship) {
        class.relationships.push(relationship);
    }
}

fn infer_relationships_from_builder(
    ctx: &mut VisitContext,
    builder: &ParsedClassInfo,
    known_class_ids: &HashSet<String>,
) {
    let Some(class) = ctx.types.get_mut(&builder.id) else {
        return;
    };

    infer_variable_relationships(class, &builder.variable_types, known_class_ids);
    infer_method_relationships(class, &builder.method_types, known_class_ids);
}

fn infer_variable_relationships(
    class: &mut SimpleEntity,
    variable_types: &[ParsedVariableType],
    known_class_ids: &HashSet<String>,
) {
    for variable in variable_types {
        add_relationship_from_resolved_type(
            class,
            &variable.resolved_type,
            known_class_ids,
            RelationType::Aggregation,
            RelationType::Composition,
        );
    }
}

fn infer_method_relationships(
    class: &mut SimpleEntity,
    method_types: &[ParsedMethodType],
    known_class_ids: &HashSet<String>,
) {
    for method in method_types {
        add_relationship_from_resolved_type(
            class,
            &method.return_type,
            known_class_ids,
            RelationType::Dependency,
            RelationType::Association,
        );

        for parameter_type in &method.parameter_types {
            add_relationship_from_resolved_type(
                class,
                parameter_type,
                known_class_ids,
                RelationType::Dependency,
                RelationType::Association,
            );
        }
    }
}

fn add_relationship_from_resolved_type(
    class: &mut SimpleEntity,
    resolved_type: &ResolvedType,
    known_class_ids: &HashSet<String>,
    non_owning_relation: RelationType,
    owning_relation: RelationType,
) {
    let Some(raw_target) = resolved_type.relationship_target_entity_id() else {
        return;
    };

    let Some(target) = resolve_in_model_target(class, raw_target, known_class_ids) else {
        return;
    };

    let relation_type = if resolved_type.is_non_owning() {
        non_owning_relation
    } else {
        owning_relation
    };

    add_relationship(class, target, relation_type);
}

fn resolve_in_model_target(
    source_class: &SimpleEntity,
    raw_target: &str,
    known_class_ids: &HashSet<String>,
) -> Option<String> {
    if known_class_ids.contains(raw_target) {
        return Some(raw_target.to_string());
    }

    if !raw_target.contains("::") {
        if let Some(ns) = source_class.enclosing_namespace_id.as_deref() {
            let mut current_ns: Option<&str> = Some(ns);
            while let Some(n) = current_ns {
                let candidate = format!("{n}::{raw_target}");
                if known_class_ids.contains(&candidate) {
                    return Some(candidate);
                }
                current_ns = n.rsplit_once("::").map(|(parent, _)| parent);
            }
        }
    }

    None
}
