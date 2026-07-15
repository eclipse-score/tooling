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

//! Reader for class-diagram FlatBuffer exports used by design verification.

use std::fs;

use class_fbs::class_metamodel as fb_class;

use crate::models::ClassDiagramInputs;
use crate::readers::{to_source_location, Reader};
use class_diagram::{
    ClassDiagram, EntityType, EnumLiteral, FunctionArgument, MemberVariable, Method,
    MethodModifier, RelationType, Relationship, SimpleEntity, TemplateParameter, TypeAlias,
    Visibility,
};

pub struct ClassDiagramReader;

fn read_template_parameters(
    values: Option<
        flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<fb_class::TemplateParameter<'_>>>,
    >,
    context: &str,
) -> Result<Option<Vec<TemplateParameter>>, String> {
    values
        .map(|items| {
            items
                .iter()
                .enumerate()
                .map(|(index, parameter)| {
                    read_template_parameter(
                        parameter,
                        &format!("{context}:template_parameter[{index}]"),
                    )
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()
}

fn read_template_parameter(
    parameter: fb_class::TemplateParameter<'_>,
    context: &str,
) -> Result<TemplateParameter, String> {
    match parameter.kind() {
        fb_class::TemplateParameterKind::Type => Ok(TemplateParameter::Type {
            name: parameter.name().to_string(),
            is_pack: parameter.is_pack(),
        }),
        fb_class::TemplateParameterKind::NonType => Ok(TemplateParameter::NonType {
            name: parameter.name().to_string(),
            value_type: parameter.value_type().unwrap_or_default().to_string(),
            is_pack: parameter.is_pack(),
        }),
        fb_class::TemplateParameterKind::Template => Ok(TemplateParameter::Template {
            name: parameter.name().to_string(),
            parameters: read_template_parameters(
                parameter.parameters(),
                &format!("{context}:parameters"),
            )?
            .unwrap_or_default(),
            is_pack: parameter.is_pack(),
        }),
        _ => Err(unsupported_enum(
            context,
            "template_parameter_kind",
            parameter.kind(),
        )),
    }
}

fn read_type_aliases(entity: fb_class::SimpleEntity<'_>) -> Vec<TypeAlias> {
    entity
        .type_aliases()
        .map(|values| {
            values
                .iter()
                .map(|value| TypeAlias {
                    alias: value.alias().to_string(),
                    original_type: value.original_type().to_string(),
                    source_location: to_source_location(
                        value.source_location().file(),
                        value.source_location().line(),
                    ),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn read_variables(
    entity: fb_class::SimpleEntity<'_>,
    path: &str,
) -> Result<Vec<MemberVariable>, String> {
    entity
        .variables()
        .map(|values| {
            values
                .iter()
                .map(|value| {
                    Ok(MemberVariable {
                        name: value.name().to_string(),
                        data_type: value.data_type().map(|s| s.to_string()),
                        visibility: map_visibility(
                            value.visibility(),
                            &format!("{path}:entity:{}:variable:{}", entity.id(), value.name()),
                        )?,
                        is_static: value.is_static(),
                        source_location: to_source_location(
                            value.source_location().file(),
                            value.source_location().line(),
                        ),
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()
        .map(|values| values.unwrap_or_default())
}

fn read_method(
    method: fb_class::Method<'_>,
    entity: fb_class::SimpleEntity<'_>,
    path: &str,
) -> Result<Method, String> {
    let parameters = method
        .parameters()
        .map(|params| {
            params
                .iter()
                .map(|param| FunctionArgument {
                    name: param.name().to_string(),
                    param_type: param.param_type().map(|s| s.to_string()),
                    is_variadic: param.is_variadic(),
                    is_pack_expansion: param.is_pack_expansion(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let template_parameters = read_template_parameters(
        method.template_parameters(),
        &format!("{path}:entity:{}:method:{}", entity.id(), method.name()),
    )?;

    let modifiers = method
        .modifiers()
        .map(|mods| {
            mods.iter()
                .map(|modifier| {
                    map_method_modifier(
                        modifier,
                        &format!("{path}:entity:{}:method:{}", entity.id(), method.name()),
                    )
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?
        .unwrap_or_default();

    Ok(Method {
        name: method.name().to_string(),
        return_type: method.return_type().map(|s| s.to_string()),
        visibility: map_visibility(
            method.visibility(),
            &format!("{path}:entity:{}:method:{}", entity.id(), method.name()),
        )?,
        parameters,
        template_parameters,
        modifiers,
        source_location: to_source_location(
            method.source_location().file(),
            method.source_location().line(),
        ),
    })
}

fn read_methods(entity: fb_class::SimpleEntity<'_>, path: &str) -> Result<Vec<Method>, String> {
    entity
        .methods()
        .map(|values| {
            values
                .iter()
                .map(|method| read_method(method, entity, path))
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()
        .map(|values| values.unwrap_or_default())
}

fn read_enum_literals(entity: fb_class::SimpleEntity<'_>) -> Vec<EnumLiteral> {
    entity
        .enum_literals()
        .map(|values| {
            values
                .iter()
                .map(|value| EnumLiteral {
                    name: value.name().to_string(),
                    value: value.value().and_then(|s| s.parse::<i128>().ok()),
                    source_location: to_source_location(
                        value.source_location().file(),
                        value.source_location().line(),
                    ),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn read_entity_relationships(
    entity: fb_class::SimpleEntity<'_>,
    path: &str,
) -> Result<Vec<Relationship>, String> {
    entity
        .relationships()
        .map(|values| {
            values
                .iter()
                .map(|rel| {
                    read_relationship(rel, &format!("{path}:entity:{}:relationship", entity.id()))
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()
        .map(|values| values.unwrap_or_default())
}

fn read_entity(entity: fb_class::SimpleEntity<'_>, path: &str) -> Result<SimpleEntity, String> {
    Ok(SimpleEntity {
        id: entity.id().to_string(),
        name: entity.name().to_string(),
        enclosing_namespace_id: entity.enclosing_namespace_id().map(|s| s.to_string()),
        entity_type: map_entity_type(
            entity.entity_type(),
            &format!("{path}:entity:{}", entity.id()),
        )?,
        type_aliases: read_type_aliases(entity),
        variables: read_variables(entity, path)?,
        methods: read_methods(entity, path)?,
        template_parameters: read_template_parameters(
            entity.template_parameters(),
            &format!("{path}:entity:{}", entity.id()),
        )?,
        enum_literals: read_enum_literals(entity),
        relationships: read_entity_relationships(entity, path)?,
        source_location: to_source_location(
            entity.source_location().file(),
            entity.source_location().line(),
        ),
    })
}

fn read_entities(
    diagram: fb_class::ClassDiagram<'_>,
    path: &str,
) -> Result<Vec<SimpleEntity>, String> {
    diagram
        .entities()
        .map(|raw_entities| {
            raw_entities
                .iter()
                .map(|entity| read_entity(entity, path))
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()
        .map(|values| values.unwrap_or_default())
}
fn read_relationship(
    rel: fb_class::Relationship<'_>,
    context: &str,
) -> Result<Relationship, String> {
    Ok(Relationship {
        source: rel.source().to_string(),
        target: rel.target().to_string(),
        relation_type: map_relation_type(rel.relation_type(), context)?,
        source_multiplicity: rel.source_multiplicity().map(|s| s.to_string()),
        target_multiplicity: rel.target_multiplicity().map(|s| s.to_string()),
        source_location: to_source_location(
            rel.source_location().file(),
            rel.source_location().line(),
        ),
    })
}

impl Reader for ClassDiagramReader {
    type Input = [String];
    type Raw = ClassDiagramInputs;
    type Error = String;

    fn read(input: &Self::Input) -> Result<Self::Raw, Self::Error> {
        let mut diagrams = Vec::new();

        for path in input {
            let data = fs::read(path).map_err(|e| format!("Failed to read {path}: {e}"))?;

            let diagram = flatbuffers::root::<fb_class::ClassDiagram>(&data)
                .map_err(|e| format!("Failed to parse class FlatBuffer {path}: {e}"))?;

            let entities = read_entities(diagram, path)?;

            diagrams.push(ClassDiagram {
                name: diagram.name().to_string(),
                entities,
            });
        }

        Ok(diagrams)
    }
}

fn unsupported_enum<T: std::fmt::Debug>(context: &str, label: &str, value: T) -> String {
    format!("{context}: unsupported {label} {value:?}")
}

fn map_entity_type(value: fb_class::EntityType, context: &str) -> Result<EntityType, String> {
    match value {
        fb_class::EntityType::Class => Ok(EntityType::Class),
        fb_class::EntityType::Struct => Ok(EntityType::Struct),
        fb_class::EntityType::Interface => Ok(EntityType::Interface),
        fb_class::EntityType::AbstractClass => Ok(EntityType::AbstractClass),
        fb_class::EntityType::Enum => Ok(EntityType::Enum),
        _ => Err(unsupported_enum(context, "entity_type", value)),
    }
}

fn map_visibility(value: fb_class::Visibility, context: &str) -> Result<Visibility, String> {
    match value {
        fb_class::Visibility::Public => Ok(Visibility::Public),
        fb_class::Visibility::Private => Ok(Visibility::Private),
        fb_class::Visibility::Protected => Ok(Visibility::Protected),
        _ => Err(unsupported_enum(context, "visibility", value)),
    }
}

fn map_relation_type(value: fb_class::RelationType, context: &str) -> Result<RelationType, String> {
    match value {
        fb_class::RelationType::Inheritance => Ok(RelationType::Inheritance),
        fb_class::RelationType::Implementation => Ok(RelationType::Implementation),
        fb_class::RelationType::Composition => Ok(RelationType::Composition),
        fb_class::RelationType::Aggregation => Ok(RelationType::Aggregation),
        fb_class::RelationType::Association => Ok(RelationType::Association),
        fb_class::RelationType::Dependency => Ok(RelationType::Dependency),
        _ => Err(unsupported_enum(context, "relation_type", value)),
    }
}

fn map_method_modifier(
    value: fb_class::MethodModifier,
    context: &str,
) -> Result<MethodModifier, String> {
    match value {
        fb_class::MethodModifier::Static => Ok(MethodModifier::Static),
        fb_class::MethodModifier::Virtual => Ok(MethodModifier::Virtual),
        fb_class::MethodModifier::Abstract => Ok(MethodModifier::Abstract),
        fb_class::MethodModifier::Override => Ok(MethodModifier::Override),
        fb_class::MethodModifier::Constructor => Ok(MethodModifier::Constructor),
        fb_class::MethodModifier::Destructor => Ok(MethodModifier::Destructor),
        fb_class::MethodModifier::Noexcept => Ok(MethodModifier::Noexcept),
        _ => Err(unsupported_enum(context, "method_modifier", value)),
    }
}
