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

use class_diagram::{
    ClassDiagram, ClassTemplateKind, EntityType, EnumLiteral, FreeFunctionDecl, FunctionArgument,
    MemberVariable, Method, MethodModifier, ParameterDirection, QualifiedTypeName, RelationType,
    Relationship, SimpleEntity, SourceLocation, TemplateParameter, TypeAlias, TypeArgument,
    TypeDescription, TypeExpr, TypeNameSegment, Visibility,
};
use class_fbs::class_metamodel as fb;
use flatbuffers::FlatBufferBuilder;

pub struct ClassSerializer;

impl ClassSerializer {
    pub fn serialize(diagram: &ClassDiagram) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        let name_offset = builder.create_string(&diagram.name);
        let source_offset = diagram.source.as_deref().map(|s| builder.create_string(s));

        let entity_offsets: Vec<_> = diagram
            .entities
            .iter()
            .map(|entity| Self::serialize_entity(&mut builder, entity))
            .collect();
        let entities_offset = builder.create_vector(&entity_offsets);

        let free_function_offsets: Vec<_> = diagram
            .free_functions
            .iter()
            .map(|function| Self::serialize_free_function(&mut builder, function))
            .collect();
        let free_functions_offset = builder.create_vector(&free_function_offsets);

        let root = fb::ClassDiagram::create(
            &mut builder,
            &fb::ClassDiagramArgs {
                name: Some(name_offset),
                entities: Some(entities_offset),
                free_functions: Some(free_functions_offset),
                source: source_offset,
            },
        );

        builder.finish(root, Some("CLSD"));
        builder.finished_data().to_vec()
    }

    fn serialize_entity<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        entity: &SimpleEntity,
    ) -> flatbuffers::WIPOffset<fb::SimpleEntity<'a>> {
        let id_offset = builder.create_string(&entity.id);
        let name_offset = builder.create_string(&entity.name);
        let enclosing_namespace_id_offset = entity
            .enclosing_namespace_id
            .as_ref()
            .map(|namespace| builder.create_string(namespace));
        let stereotype_offsets: Vec<_> = entity
            .stereotypes
            .iter()
            .map(|stereotype| builder.create_string(stereotype))
            .collect();
        let stereotypes_offset = builder.create_vector(&stereotype_offsets);

        let type_alias_offsets: Vec<_> = entity
            .type_aliases
            .iter()
            .map(|type_alias| Self::serialize_type_alias(builder, type_alias))
            .collect();
        let type_aliases_offset = builder.create_vector(&type_alias_offsets);

        let variable_offsets: Vec<_> = entity
            .variables
            .iter()
            .map(|variable| Self::serialize_variable(builder, variable))
            .collect();
        let variables_offset = builder.create_vector(&variable_offsets);

        let method_offsets: Vec<_> = entity
            .methods
            .iter()
            .map(|method| Self::serialize_method(builder, method))
            .collect();
        let methods_offset = builder.create_vector(&method_offsets);

        let template_parameters_offset = entity.template_parameters.as_ref().map(|parameters| {
            let template_offsets: Vec<_> = parameters
                .iter()
                .map(|parameter| Self::serialize_template_parameter(builder, parameter))
                .collect();
            builder.create_vector(&template_offsets)
        });

        let enum_literal_offsets: Vec<_> = entity
            .enum_literals
            .iter()
            .map(|literal| Self::serialize_enum_literal(builder, literal))
            .collect();
        let enum_literals_offset = builder.create_vector(&enum_literal_offsets);

        let entity_relationship_offsets: Vec<_> = entity
            .relationships
            .iter()
            .map(|relationship| Self::serialize_relationship(builder, relationship))
            .collect();
        let entity_relationships_offset = builder.create_vector(&entity_relationship_offsets);

        let source_location_offset =
            Self::serialize_source_location(builder, &entity.source_location);

        fb::SimpleEntity::create(
            builder,
            &fb::SimpleEntityArgs {
                id: Some(id_offset),
                name: Some(name_offset),
                enclosing_namespace_id: enclosing_namespace_id_offset,
                stereotypes: Some(stereotypes_offset),
                entity_type: Self::map_entity_type(entity.entity_type),
                type_aliases: Some(type_aliases_offset),
                variables: Some(variables_offset),
                methods: Some(methods_offset),
                template_parameters: template_parameters_offset,
                enum_literals: Some(enum_literals_offset),
                relationships: Some(entity_relationships_offset),
                source_location: Some(source_location_offset),
                template_kind: Self::map_template_kind(entity.template_kind),
            },
        )
    }

    fn serialize_source_location<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        source_location: &SourceLocation,
    ) -> flatbuffers::WIPOffset<fb::SourceLocation<'a>> {
        let file_offset = builder.create_string(source_location.file.as_ref());
        fb::SourceLocation::create(
            builder,
            &fb::SourceLocationArgs {
                file: Some(file_offset),
                line: source_location.line,
            },
        )
    }

    fn serialize_type_description<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        description: &TypeDescription,
    ) -> flatbuffers::WIPOffset<fb::TypeDescription<'a>> {
        let raw_text = builder.create_string(&description.raw_text);
        let expression = description
            .expression
            .as_ref()
            .map(|expression| Self::serialize_type_expr(builder, expression));
        fb::TypeDescription::create(
            builder,
            &fb::TypeDescriptionArgs {
                raw_text: Some(raw_text),
                expression,
            },
        )
    }

    fn serialize_type_expr<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        expression: &TypeExpr,
    ) -> flatbuffers::WIPOffset<fb::TypeExpr<'a>> {
        let (value_type, value) = match expression {
            TypeExpr::Builtin(name) => {
                let name = builder.create_string(name);
                let value =
                    fb::BuiltinType::create(builder, &fb::BuiltinTypeArgs { name: Some(name) });
                (fb::TypeExprValue::BuiltinType, value.as_union_value())
            }
            TypeExpr::NamedType(name) => {
                let name = Self::serialize_qualified_type_name(builder, name);
                let value = fb::NamedType::create(builder, &fb::NamedTypeArgs { name: Some(name) });
                (fb::TypeExprValue::NamedType, value.as_union_value())
            }
            TypeExpr::Template { name, arguments } => {
                let name = Self::serialize_qualified_type_name(builder, name);
                let arguments = Self::serialize_type_arguments(builder, arguments);
                let value = fb::TemplateType::create(
                    builder,
                    &fb::TemplateTypeArgs {
                        name: Some(name),
                        arguments: Some(arguments),
                    },
                );
                (fb::TypeExprValue::TemplateType, value.as_union_value())
            }
            TypeExpr::Array { element, size } => {
                let element = Self::serialize_type_expr(builder, element);
                let size = size.as_ref().map(|size| builder.create_string(size));
                let value = fb::ArrayType::create(
                    builder,
                    &fb::ArrayTypeArgs {
                        element: Some(element),
                        size,
                    },
                );
                (fb::TypeExprValue::ArrayType, value.as_union_value())
            }
            TypeExpr::Function {
                return_type,
                parameter_types,
                is_variadic,
                is_noexcept,
            } => {
                let return_type = Self::serialize_type_expr(builder, return_type);
                let parameter_offsets: Vec<_> = parameter_types
                    .iter()
                    .map(|ty| Self::serialize_type_expr(builder, ty))
                    .collect();
                let parameter_types = builder.create_vector(&parameter_offsets);
                let value = fb::FunctionType::create(
                    builder,
                    &fb::FunctionTypeArgs {
                        return_type: Some(return_type),
                        parameter_types: Some(parameter_types),
                        is_variadic: *is_variadic,
                        is_noexcept: *is_noexcept,
                    },
                );
                (fb::TypeExprValue::FunctionType, value.as_union_value())
            }
            TypeExpr::Pointer(inner) => {
                let inner = Self::serialize_type_expr(builder, inner);
                let value =
                    fb::PointerType::create(builder, &fb::PointerTypeArgs { inner: Some(inner) });
                (fb::TypeExprValue::PointerType, value.as_union_value())
            }
            TypeExpr::Reference(inner) => {
                let inner = Self::serialize_type_expr(builder, inner);
                let value = fb::ReferenceType::create(
                    builder,
                    &fb::ReferenceTypeArgs { inner: Some(inner) },
                );
                (fb::TypeExprValue::ReferenceType, value.as_union_value())
            }
            TypeExpr::RValueReference(inner) => {
                let inner = Self::serialize_type_expr(builder, inner);
                let value = fb::RValueReferenceType::create(
                    builder,
                    &fb::RValueReferenceTypeArgs { inner: Some(inner) },
                );
                (
                    fb::TypeExprValue::RValueReferenceType,
                    value.as_union_value(),
                )
            }
            TypeExpr::Const(inner) => {
                let inner = Self::serialize_type_expr(builder, inner);
                let value =
                    fb::ConstType::create(builder, &fb::ConstTypeArgs { inner: Some(inner) });
                (fb::TypeExprValue::ConstType, value.as_union_value())
            }
            TypeExpr::Volatile(inner) => {
                let inner = Self::serialize_type_expr(builder, inner);
                let value =
                    fb::VolatileType::create(builder, &fb::VolatileTypeArgs { inner: Some(inner) });
                (fb::TypeExprValue::VolatileType, value.as_union_value())
            }
        };
        fb::TypeExpr::create(
            builder,
            &fb::TypeExprArgs {
                value_type,
                value: Some(value),
            },
        )
    }

    fn serialize_qualified_type_name<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        name: &QualifiedTypeName,
    ) -> flatbuffers::WIPOffset<fb::QualifiedTypeName<'a>> {
        let segment_offsets: Vec<_> = name
            .segments
            .iter()
            .map(|segment| {
                let (name, arguments) = match segment {
                    TypeNameSegment::Identifier(name) => (name, None),
                    TypeNameSegment::Template { name, arguments } => (
                        name,
                        Some(Self::serialize_type_arguments(builder, arguments)),
                    ),
                };
                let name = builder.create_string(name);
                fb::TypeNameSegment::create(
                    builder,
                    &fb::TypeNameSegmentArgs {
                        name: Some(name),
                        arguments,
                    },
                )
            })
            .collect();
        let segments = builder.create_vector(&segment_offsets);
        fb::QualifiedTypeName::create(
            builder,
            &fb::QualifiedTypeNameArgs {
                segments: Some(segments),
                is_global: name.is_global,
            },
        )
    }

    fn serialize_type_arguments<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        arguments: &[TypeArgument],
    ) -> flatbuffers::WIPOffset<
        flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<fb::TypeArgument<'a>>>,
    > {
        let offsets: Vec<_> = arguments
            .iter()
            .map(|argument| {
                let (value_type, value) = match argument {
                    TypeArgument::Type(expression) => (
                        fb::TypeArgumentValue::TypeExpr,
                        Self::serialize_type_expr(builder, expression).as_union_value(),
                    ),
                    TypeArgument::Value(text) => {
                        let text = builder.create_string(text);
                        let value = fb::TemplateValue::create(
                            builder,
                            &fb::TemplateValueArgs { text: Some(text) },
                        );
                        (fb::TypeArgumentValue::TemplateValue, value.as_union_value())
                    }
                };
                fb::TypeArgument::create(
                    builder,
                    &fb::TypeArgumentArgs {
                        value_type,
                        value: Some(value),
                    },
                )
            })
            .collect();
        builder.create_vector(&offsets)
    }

    fn serialize_type_alias<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        type_alias: &TypeAlias,
    ) -> flatbuffers::WIPOffset<fb::TypeAlias<'a>> {
        let alias_offset = builder.create_string(&type_alias.alias);
        let original_type = Self::serialize_type_description(builder, &type_alias.original_type);
        let source_location_offset =
            Self::serialize_source_location(builder, &type_alias.source_location);

        fb::TypeAlias::create(
            builder,
            &fb::TypeAliasArgs {
                alias: Some(alias_offset),
                original_type: Some(original_type),
                source_location: Some(source_location_offset),
            },
        )
    }

    fn serialize_variable<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        variable: &MemberVariable,
    ) -> flatbuffers::WIPOffset<fb::MemberVariable<'a>> {
        let name_offset = builder.create_string(&variable.name);
        let data_type = variable
            .data_type
            .as_ref()
            .map(|data_type| Self::serialize_type_description(builder, data_type));
        let source_location_offset =
            Self::serialize_source_location(builder, &variable.source_location);

        fb::MemberVariable::create(
            builder,
            &fb::MemberVariableArgs {
                name: Some(name_offset),
                data_type,
                visibility: Self::map_visibility(variable.visibility),
                is_static: variable.is_static,
                source_location: Some(source_location_offset),
            },
        )
    }

    fn serialize_method<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        method: &Method,
    ) -> flatbuffers::WIPOffset<fb::Method<'a>> {
        let name_offset = builder.create_string(&method.name);
        let return_type = method
            .return_type
            .as_ref()
            .map(|return_type| Self::serialize_type_description(builder, return_type));
        let return_multiplicity_offset = method
            .return_multiplicity
            .as_ref()
            .map(|value| builder.create_string(value));
        let conversion_target_offset = method
            .conversion_target
            .as_ref()
            .map(|target| builder.create_string(target));

        let parameter_offsets: Vec<_> = method
            .parameters
            .iter()
            .map(|parameter| Self::serialize_parameter(builder, parameter))
            .collect();
        let parameters_offset = builder.create_vector(&parameter_offsets);

        let template_parameters_offset = method.template_parameters.as_ref().map(|parameters| {
            let template_offsets: Vec<_> = parameters
                .iter()
                .map(|parameter| Self::serialize_template_parameter(builder, parameter))
                .collect();
            builder.create_vector(&template_offsets)
        });

        let modifier_values: Vec<_> = method
            .modifiers
            .iter()
            .map(Self::map_method_modifier)
            .collect();
        let modifiers_offset = builder.create_vector(&modifier_values);
        let source_location_offset =
            Self::serialize_source_location(builder, &method.source_location);

        fb::Method::create(
            builder,
            &fb::MethodArgs {
                name: Some(name_offset),
                conversion_target: conversion_target_offset,
                return_type,
                return_multiplicity: return_multiplicity_offset,
                visibility: Self::map_visibility(method.visibility),
                parameters: Some(parameters_offset),
                template_parameters: template_parameters_offset,
                modifiers: Some(modifiers_offset),
                source_location: Some(source_location_offset),
            },
        )
    }

    fn serialize_free_function<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        function: &FreeFunctionDecl,
    ) -> flatbuffers::WIPOffset<fb::FreeFunctionDecl<'a>> {
        let name_offset = builder.create_string(&function.name);
        let enclosing_namespace_id_offset = function
            .enclosing_namespace_id
            .as_ref()
            .map(|namespace| builder.create_string(namespace));
        let return_type = function
            .return_type
            .as_ref()
            .map(|return_type| Self::serialize_type_description(builder, return_type));
        let return_multiplicity_offset = function
            .return_multiplicity
            .as_ref()
            .map(|value| builder.create_string(value));

        let parameter_offsets: Vec<_> = function
            .parameters
            .iter()
            .map(|parameter| Self::serialize_parameter(builder, parameter))
            .collect();
        let parameters_offset = builder.create_vector(&parameter_offsets);

        let template_parameters_offset = function.template_parameters.as_ref().map(|parameters| {
            let template_offsets: Vec<_> = parameters
                .iter()
                .map(|parameter| Self::serialize_template_parameter(builder, parameter))
                .collect();
            builder.create_vector(&template_offsets)
        });
        let source_location_offset =
            Self::serialize_source_location(builder, &function.source_location);

        fb::FreeFunctionDecl::create(
            builder,
            &fb::FreeFunctionDeclArgs {
                name: Some(name_offset),
                enclosing_namespace_id: enclosing_namespace_id_offset,
                return_type,
                return_multiplicity: return_multiplicity_offset,
                parameters: Some(parameters_offset),
                template_parameters: template_parameters_offset,
                source_location: Some(source_location_offset),
            },
        )
    }

    fn serialize_parameter<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        param: &FunctionArgument,
    ) -> flatbuffers::WIPOffset<fb::FunctionArgument<'a>> {
        let name_offset = builder.create_string(&param.name);
        let param_type = param
            .param_type
            .as_ref()
            .map(|param_type| Self::serialize_type_description(builder, param_type));
        let multiplicity_offset = param
            .multiplicity
            .as_ref()
            .map(|value| builder.create_string(value));

        fb::FunctionArgument::create(
            builder,
            &fb::FunctionArgumentArgs {
                name: Some(name_offset),
                param_type,
                is_variadic: param.is_variadic,
                is_pack_expansion: param.is_pack_expansion,
                direction: Self::map_parameter_direction(param.direction),
                multiplicity: multiplicity_offset,
            },
        )
    }

    fn serialize_template_parameter<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        parameter: &TemplateParameter,
    ) -> flatbuffers::WIPOffset<fb::TemplateParameter<'a>> {
        let (kind, name, value_type, parameters, is_pack) = match parameter {
            TemplateParameter::Type { name, is_pack } => {
                (fb::TemplateParameterKind::Type, name, None, None, *is_pack)
            }
            TemplateParameter::NonType {
                name,
                value_type,
                is_pack,
            } => (
                fb::TemplateParameterKind::NonType,
                name,
                Some(value_type),
                None,
                *is_pack,
            ),
            TemplateParameter::Template {
                name,
                parameters,
                is_pack,
            } => (
                fb::TemplateParameterKind::Template,
                name,
                None,
                Some(parameters),
                *is_pack,
            ),
        };

        let name_offset = builder.create_string(name);
        let value_type_offset = value_type.map(|value| builder.create_string(value));
        let parameters_offset = parameters.map(|nested_parameters| {
            let nested_offsets: Vec<_> = nested_parameters
                .iter()
                .map(|nested| Self::serialize_template_parameter(builder, nested))
                .collect();
            builder.create_vector(&nested_offsets)
        });

        fb::TemplateParameter::create(
            builder,
            &fb::TemplateParameterArgs {
                kind,
                name: Some(name_offset),
                value_type: value_type_offset,
                parameters: parameters_offset,
                is_pack,
            },
        )
    }

    fn serialize_enum_literal<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        literal: &EnumLiteral,
    ) -> flatbuffers::WIPOffset<fb::EnumLiteral<'a>> {
        let name_offset = builder.create_string(&literal.name);
        let value_offset = literal.value.map(|value| {
            let value = value.to_string();
            builder.create_string(&value)
        });
        let source_location_offset =
            Self::serialize_source_location(builder, &literal.source_location);

        fb::EnumLiteral::create(
            builder,
            &fb::EnumLiteralArgs {
                name: Some(name_offset),
                value: value_offset,
                source_location: Some(source_location_offset),
            },
        )
    }

    fn serialize_relationship<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        relationship: &Relationship,
    ) -> flatbuffers::WIPOffset<fb::Relationship<'a>> {
        let source_offset = builder.create_string(&relationship.source);
        let target_offset = builder.create_string(&relationship.target);
        let source_multiplicity_offset = relationship
            .source_multiplicity
            .as_ref()
            .map(|multiplicity| builder.create_string(multiplicity));
        let target_multiplicity_offset = relationship
            .target_multiplicity
            .as_ref()
            .map(|multiplicity| builder.create_string(multiplicity));
        let source_location_offset =
            Self::serialize_source_location(builder, &relationship.source_location);

        fb::Relationship::create(
            builder,
            &fb::RelationshipArgs {
                source: Some(source_offset),
                target: Some(target_offset),
                relation_type: Self::map_relation_type(relationship.relation_type),
                source_multiplicity: source_multiplicity_offset,
                target_multiplicity: target_multiplicity_offset,
                source_location: Some(source_location_offset),
            },
        )
    }

    fn map_parameter_direction(direction: Option<ParameterDirection>) -> fb::ParameterDirection {
        match direction {
            None => fb::ParameterDirection::Unspecified,
            Some(ParameterDirection::In) => fb::ParameterDirection::In,
            Some(ParameterDirection::Out) => fb::ParameterDirection::Out,
            Some(ParameterDirection::InOut) => fb::ParameterDirection::InOut,
        }
    }

    fn map_visibility(v: Visibility) -> fb::Visibility {
        match v {
            Visibility::Public => fb::Visibility::Public,
            Visibility::Private => fb::Visibility::Private,
            Visibility::Protected => fb::Visibility::Protected,
        }
    }

    fn map_entity_type(t: EntityType) -> fb::EntityType {
        match t {
            EntityType::Class => fb::EntityType::Class,
            EntityType::Struct => fb::EntityType::Struct,
            EntityType::Interface => fb::EntityType::Interface,
            EntityType::AbstractClass => fb::EntityType::AbstractClass,
            EntityType::Enum => fb::EntityType::Enum,
        }
    }

    fn map_template_kind(kind: ClassTemplateKind) -> fb::ClassTemplateKind {
        match kind {
            ClassTemplateKind::NonTemplate => fb::ClassTemplateKind::NonTemplate,
            ClassTemplateKind::Primary => fb::ClassTemplateKind::Primary,
            ClassTemplateKind::PartialSpecialization => {
                fb::ClassTemplateKind::PartialSpecialization
            }
            ClassTemplateKind::ExplicitSpecialization => {
                fb::ClassTemplateKind::ExplicitSpecialization
            }
        }
    }

    fn map_method_modifier(t: &MethodModifier) -> fb::MethodModifier {
        match t {
            MethodModifier::Static => fb::MethodModifier::Static,
            MethodModifier::Virtual => fb::MethodModifier::Virtual,
            MethodModifier::Abstract => fb::MethodModifier::Abstract,
            MethodModifier::Override => fb::MethodModifier::Override,
            MethodModifier::Constructor => fb::MethodModifier::Constructor,
            MethodModifier::Destructor => fb::MethodModifier::Destructor,
            MethodModifier::Noexcept => fb::MethodModifier::Noexcept,
            MethodModifier::Final => fb::MethodModifier::Final,
            MethodModifier::Const => fb::MethodModifier::Const,
        }
    }

    fn map_relation_type(t: RelationType) -> fb::RelationType {
        match t {
            RelationType::Inheritance => fb::RelationType::Inheritance,
            RelationType::Implementation => fb::RelationType::Implementation,
            RelationType::Composition => fb::RelationType::Composition,
            RelationType::Aggregation => fb::RelationType::Aggregation,
            RelationType::Association => fb::RelationType::Association,
            RelationType::Dependency => fb::RelationType::Dependency,
        }
    }
}
