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
    ClassDiagram, EntityType, EnumLiteral, FunctionArgument, MemberVariable, Method,
    MethodModifier, RelationType, Relationship, SimpleEntity, TypeAlias, Visibility,
};
use class_fbs::class_metamodel as fb;
use flatbuffers::FlatBufferBuilder;

const UNKNOWN_SOURCE_LINE: u32 = 0;

pub struct ClassSerializer;

impl ClassSerializer {
    pub fn serialize(diagram: &ClassDiagram, _source_file: &str) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        let name_offset = builder.create_string(&diagram.name);

        let entity_offsets: Vec<_> = diagram
            .entities
            .iter()
            .map(|entity| Self::serialize_entity(&mut builder, entity))
            .collect();
        let entities_offset = builder.create_vector(&entity_offsets);

        let relationship_offsets: Vec<_> = diagram
            .relationships
            .iter()
            .map(|relationship| Self::serialize_relationship(&mut builder, relationship))
            .collect();
        let relationships_offset = builder.create_vector(&relationship_offsets);

        let source_offsets: Vec<_> = diagram
            .source_files
            .iter()
            .map(|source| builder.create_string(source))
            .collect();
        let source_files_offset = builder.create_vector(&source_offsets);

        let version_offset = diagram
            .version
            .as_ref()
            .map(|version| builder.create_string(version));

        let root = fb::ClassDiagram::create(
            &mut builder,
            &fb::ClassDiagramArgs {
                name: Some(name_offset),
                entities: Some(entities_offset),
                relationships: Some(relationships_offset),
                source_files: Some(source_files_offset),
                version: version_offset,
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
                .map(|parameter| builder.create_string(parameter))
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

        let source_file_offset = entity
            .source_file
            .as_ref()
            .map(|source| builder.create_string(source));

        fb::SimpleEntity::create(
            builder,
            &fb::SimpleEntityArgs {
                id: Some(id_offset),
                name: Some(name_offset),
                enclosing_namespace_id: enclosing_namespace_id_offset,
                entity_type: Self::map_entity_type(entity.entity_type),
                type_aliases: Some(type_aliases_offset),
                variables: Some(variables_offset),
                methods: Some(methods_offset),
                template_parameters: template_parameters_offset,
                enum_literals: Some(enum_literals_offset),
                relationships: Some(entity_relationships_offset),
                source_file: source_file_offset,
                source_line: entity.source_line.unwrap_or(UNKNOWN_SOURCE_LINE),
            },
        )
    }

    fn serialize_type_alias<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        type_alias: &TypeAlias,
    ) -> flatbuffers::WIPOffset<fb::TypeAlias<'a>> {
        let alias_offset = builder.create_string(&type_alias.alias);
        let original_type_offset = builder.create_string(&type_alias.original_type);

        fb::TypeAlias::create(
            builder,
            &fb::TypeAliasArgs {
                alias: Some(alias_offset),
                original_type: Some(original_type_offset),
            },
        )
    }

    fn serialize_variable<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        variable: &MemberVariable,
    ) -> flatbuffers::WIPOffset<fb::MemberVariable<'a>> {
        let name_offset = builder.create_string(&variable.name);
        let data_type_offset = variable
            .data_type
            .as_ref()
            .map(|data_type| builder.create_string(data_type));

        fb::MemberVariable::create(
            builder,
            &fb::MemberVariableArgs {
                name: Some(name_offset),
                data_type: data_type_offset,
                visibility: Self::map_visibility(variable.visibility),
                is_static: variable.is_static,
                is_const: variable.is_const,
            },
        )
    }

    fn serialize_method<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        method: &Method,
    ) -> flatbuffers::WIPOffset<fb::Method<'a>> {
        let name_offset = builder.create_string(&method.name);
        let return_type_offset = method
            .return_type
            .as_ref()
            .map(|return_type| builder.create_string(return_type));

        let parameter_offsets: Vec<_> = method
            .parameters
            .iter()
            .map(|parameter| Self::serialize_parameter(builder, parameter))
            .collect();
        let parameters_offset = builder.create_vector(&parameter_offsets);

        let template_parameters_offset = method.template_parameters.as_ref().map(|parameters| {
            let template_offsets: Vec<_> = parameters
                .iter()
                .map(|parameter| builder.create_string(parameter))
                .collect();
            builder.create_vector(&template_offsets)
        });

        let modifier_values: Vec<_> = method
            .modifiers
            .iter()
            .map(Self::map_method_modifier)
            .collect();
        let modifiers_offset = builder.create_vector(&modifier_values);

        fb::Method::create(
            builder,
            &fb::MethodArgs {
                name: Some(name_offset),
                return_type: return_type_offset,
                visibility: Self::map_visibility(method.visibility),
                parameters: Some(parameters_offset),
                template_parameters: template_parameters_offset,
                modifiers: Some(modifiers_offset),
            },
        )
    }

    fn serialize_parameter<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        param: &FunctionArgument,
    ) -> flatbuffers::WIPOffset<fb::FunctionArgument<'a>> {
        let name_offset = builder.create_string(&param.name);
        let param_type_offset = param
            .param_type
            .as_ref()
            .map(|param_type| builder.create_string(param_type));

        fb::FunctionArgument::create(
            builder,
            &fb::FunctionArgumentArgs {
                name: Some(name_offset),
                param_type: param_type_offset,
                is_variadic: param.is_variadic,
            },
        )
    }

    fn serialize_enum_literal<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        literal: &EnumLiteral,
    ) -> flatbuffers::WIPOffset<fb::EnumLiteral<'a>> {
        let name_offset = builder.create_string(&literal.name);
        let value_offset = literal
            .value
            .as_ref()
            .map(|value| builder.create_string(value));

        fb::EnumLiteral::create(
            builder,
            &fb::EnumLiteralArgs {
                name: Some(name_offset),
                value: value_offset,
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

        fb::Relationship::create(
            builder,
            &fb::RelationshipArgs {
                source: Some(source_offset),
                target: Some(target_offset),
                relation_type: Self::map_relation_type(relationship.relation_type),
                source_multiplicity: source_multiplicity_offset,
                target_multiplicity: target_multiplicity_offset,
            },
        )
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

    fn map_method_modifier(t: &MethodModifier) -> fb::MethodModifier {
        match t {
            MethodModifier::Static => fb::MethodModifier::Static,
            MethodModifier::Virtual => fb::MethodModifier::Virtual,
            MethodModifier::Abstract => fb::MethodModifier::Abstract,
            MethodModifier::Override => fb::MethodModifier::Override,
            MethodModifier::Constructor => fb::MethodModifier::Constructor,
            MethodModifier::Destructor => fb::MethodModifier::Destructor,
            MethodModifier::Noexcept => fb::MethodModifier::Noexcept,
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
