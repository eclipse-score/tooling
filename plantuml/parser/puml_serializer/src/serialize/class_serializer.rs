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
    ClassDiagram, ContainerType, EntityType, LogicAttribute, LogicContainer, LogicEntity,
    LogicEnumLiteral, LogicMethod, LogicParameter, LogicRelationship, RelationType, Visibility,
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

        let container_offsets: Vec<_> = diagram
            .containers
            .iter()
            .map(|container| Self::serialize_container(&mut builder, container))
            .collect();
        let containers_offset = builder.create_vector(&container_offsets);

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

        let version_offset = diagram.version.as_ref().map(|v| builder.create_string(v));

        let root = fb::ClassDiagram::create(
            &mut builder,
            &fb::ClassDiagramArgs {
                name: Some(name_offset),
                entities: Some(entities_offset),
                containers: Some(containers_offset),
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
        entity: &LogicEntity,
    ) -> flatbuffers::WIPOffset<fb::Entity<'a>> {
        let id_offset = builder.create_string(&entity.id);
        let name_offset = entity.name.as_ref().map(|name| builder.create_string(name));
        let alias_offset = entity
            .alias
            .as_ref()
            .map(|alias| builder.create_string(alias));
        let parent_offset = entity
            .parent_id
            .as_ref()
            .map(|parent| builder.create_string(parent));

        let stereotype_offsets: Vec<_> = entity
            .stereotypes
            .iter()
            .map(|st| builder.create_string(st))
            .collect();
        let stereotypes_offset = builder.create_vector(&stereotype_offsets);

        let attribute_offsets: Vec<_> = entity
            .attributes
            .iter()
            .map(|attr| Self::serialize_attribute(builder, attr))
            .collect();
        let attributes_offset = builder.create_vector(&attribute_offsets);

        let method_offsets: Vec<_> = entity
            .methods
            .iter()
            .map(|method| Self::serialize_method(builder, method))
            .collect();
        let methods_offset = builder.create_vector(&method_offsets);

        let template_offsets: Vec<_> = entity
            .template_params
            .iter()
            .map(|param| builder.create_string(param))
            .collect();
        let template_params_offset = builder.create_vector(&template_offsets);

        let enum_literal_offsets: Vec<_> = entity
            .enum_literals
            .iter()
            .map(|literal| Self::serialize_enum_literal(builder, literal))
            .collect();
        let enum_literals_offset = builder.create_vector(&enum_literal_offsets);

        let source_file_offset = entity
            .source_file
            .as_ref()
            .map(|source| builder.create_string(source));

        fb::Entity::create(
            builder,
            &fb::EntityArgs {
                id: Some(id_offset),
                name: name_offset,
                alias: alias_offset,
                parent_id: parent_offset,
                entity_type: Self::map_entity_type(entity.entity_type),
                stereotypes: Some(stereotypes_offset),
                attributes: Some(attributes_offset),
                methods: Some(methods_offset),
                template_params: Some(template_params_offset),
                enum_literals: Some(enum_literals_offset),
                source_file: source_file_offset,
                source_line: entity.source_line.unwrap_or(UNKNOWN_SOURCE_LINE),
            },
        )
    }

    fn serialize_attribute<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        attr: &LogicAttribute,
    ) -> flatbuffers::WIPOffset<fb::Attribute<'a>> {
        let name_offset = builder.create_string(&attr.name);
        let data_type_offset = attr
            .data_type
            .as_ref()
            .map(|data_type| builder.create_string(data_type));
        let default_value_offset = attr
            .default_value
            .as_ref()
            .map(|value| builder.create_string(value));
        let description_offset = attr
            .description
            .as_ref()
            .map(|description| builder.create_string(description));

        fb::Attribute::create(
            builder,
            &fb::AttributeArgs {
                name: Some(name_offset),
                data_type: data_type_offset,
                visibility: Self::map_visibility(attr.visibility),
                default_value: default_value_offset,
                is_static: attr.is_static,
                is_const: attr.is_const,
                description: description_offset,
            },
        )
    }

    fn serialize_method<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        method: &LogicMethod,
    ) -> flatbuffers::WIPOffset<fb::Method<'a>> {
        let name_offset = builder.create_string(&method.name);
        let return_type_offset = method
            .return_type
            .as_ref()
            .map(|return_type| builder.create_string(return_type));

        let parameter_offsets: Vec<_> = method
            .parameters
            .iter()
            .map(|param| Self::serialize_parameter(builder, param))
            .collect();
        let parameters_offset = builder.create_vector(&parameter_offsets);

        let template_offsets: Vec<_> = method
            .template_params
            .iter()
            .map(|param| builder.create_string(param))
            .collect();
        let template_params_offset = builder.create_vector(&template_offsets);

        fb::Method::create(
            builder,
            &fb::MethodArgs {
                name: Some(name_offset),
                return_type: return_type_offset,
                visibility: Self::map_visibility(method.visibility),
                parameters: Some(parameters_offset),
                template_params: Some(template_params_offset),
                is_static: method.is_static,
                is_const: method.is_const,
                is_virtual: method.is_virtual,
                is_abstract: method.is_abstract,
                is_override: method.is_override,
                is_constructor: method.is_constructor,
                is_destructor: method.is_destructor,
            },
        )
    }

    fn serialize_parameter<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        param: &LogicParameter,
    ) -> flatbuffers::WIPOffset<fb::Parameter<'a>> {
        let name_offset = builder.create_string(&param.name);
        let param_type_offset = param
            .param_type
            .as_ref()
            .map(|param_type| builder.create_string(param_type));
        let default_value_offset = param
            .default_value
            .as_ref()
            .map(|value| builder.create_string(value));

        fb::Parameter::create(
            builder,
            &fb::ParameterArgs {
                name: Some(name_offset),
                param_type: param_type_offset,
                default_value: default_value_offset,
                is_reference: param.is_reference,
                is_const: param.is_const,
                is_variadic: param.is_variadic,
            },
        )
    }

    fn serialize_enum_literal<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        literal: &LogicEnumLiteral,
    ) -> flatbuffers::WIPOffset<fb::EnumLiteral<'a>> {
        let name_offset = builder.create_string(&literal.name);
        let value_offset = literal
            .value
            .as_ref()
            .map(|value| builder.create_string(value));
        let description_offset = literal
            .description
            .as_ref()
            .map(|description| builder.create_string(description));

        fb::EnumLiteral::create(
            builder,
            &fb::EnumLiteralArgs {
                name: Some(name_offset),
                visibility: Self::map_visibility(literal.visibility),
                value: value_offset,
                description: description_offset,
            },
        )
    }

    fn serialize_container<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        container: &LogicContainer,
    ) -> flatbuffers::WIPOffset<fb::Container<'a>> {
        let id_offset = builder.create_string(&container.id);
        let name_offset = builder.create_string(&container.name);
        let parent_offset = container
            .parent_id
            .as_ref()
            .map(|parent| builder.create_string(parent));

        fb::Container::create(
            builder,
            &fb::ContainerArgs {
                id: Some(id_offset),
                name: Some(name_offset),
                parent_id: parent_offset,
                container_type: Self::map_container_type(container.container_type),
            },
        )
    }

    fn serialize_relationship<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        relationship: &LogicRelationship,
    ) -> flatbuffers::WIPOffset<fb::Relationship<'a>> {
        let source_offset = builder.create_string(&relationship.source);
        let target_offset = builder.create_string(&relationship.target);
        let label_offset = relationship
            .label
            .as_ref()
            .map(|label| builder.create_string(label));
        let stereotype_offset = relationship
            .stereotype
            .as_ref()
            .map(|stereotype| builder.create_string(stereotype));
        let source_multiplicity_offset = relationship
            .source_multiplicity
            .as_ref()
            .map(|multiplicity| builder.create_string(multiplicity));
        let target_multiplicity_offset = relationship
            .target_multiplicity
            .as_ref()
            .map(|multiplicity| builder.create_string(multiplicity));
        let source_role_offset = relationship
            .source_role
            .as_ref()
            .map(|role| builder.create_string(role));
        let target_role_offset = relationship
            .target_role
            .as_ref()
            .map(|role| builder.create_string(role));

        fb::Relationship::create(
            builder,
            &fb::RelationshipArgs {
                source: Some(source_offset),
                target: Some(target_offset),
                relation_type: Self::map_relation_type(relationship.relation_type),
                label: label_offset,
                stereotype: stereotype_offset,
                source_multiplicity: source_multiplicity_offset,
                target_multiplicity: target_multiplicity_offset,
                source_role: source_role_offset,
                target_role: target_role_offset,
            },
        )
    }

    fn map_visibility(v: Visibility) -> fb::Visibility {
        match v {
            Visibility::Public => fb::Visibility::Public,
            Visibility::Private => fb::Visibility::Private,
            Visibility::Protected => fb::Visibility::Protected,
            Visibility::Package => fb::Visibility::Package,
        }
    }

    fn map_entity_type(t: EntityType) -> fb::EntityType {
        match t {
            EntityType::Class => fb::EntityType::Class,
            EntityType::Struct => fb::EntityType::Struct,
            EntityType::Interface => fb::EntityType::Interface,
            EntityType::Enum => fb::EntityType::Enum,
            EntityType::AbstractClass => fb::EntityType::AbstractClass,
            EntityType::Annotation => fb::EntityType::Annotation,
        }
    }

    fn map_container_type(t: ContainerType) -> fb::ContainerType {
        match t {
            ContainerType::Namespace => fb::ContainerType::Namespace,
            ContainerType::Package => fb::ContainerType::Package,
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
            RelationType::Link => fb::RelationType::Link,
            RelationType::DashedLink => fb::RelationType::DashedLink,
        }
    }
}
