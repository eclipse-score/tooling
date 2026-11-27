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
    ClassDiagram, ClassTemplateKind, EntityType, EnumLiteral, FreeFunctionDecl, FunctionArgument,
    MemberVariable, Method, MethodModifier, ParameterDirection, QualifiedTypeName, RelationType,
    Relationship, SimpleEntity, TemplateParameter, TypeAlias, TypeArgument, TypeDescription,
    TypeExpr, TypeNameSegment, Visibility,
};

pub struct ClassDiagramReader;

fn read_type_description(
    description: fb_class::TypeDescription<'_>,
    context: &str,
) -> Result<TypeDescription, String> {
    let raw_text = description.raw_text();
    let expression = description
        .expression()
        .map(|expression| read_type_expr(expression, context))
        .transpose()?;
    Ok(TypeDescription {
        raw_text: raw_text.to_string(),
        expression,
    })
}

fn read_optional_type_description(
    description: Option<fb_class::TypeDescription<'_>>,
    context: &str,
) -> Result<Option<TypeDescription>, String> {
    description
        .map(|description| read_type_description(description, context))
        .transpose()
}

fn read_type_expr(expression: fb_class::TypeExpr<'_>, context: &str) -> Result<TypeExpr, String> {
    match expression.value_type() {
        fb_class::TypeExprValue::BuiltinType => expression
            .value_as_builtin_type()
            .map(|value| TypeExpr::Builtin(value.name().to_string()))
            .ok_or_else(|| format!("{context}: missing builtin type payload")),
        fb_class::TypeExprValue::NamedType => expression
            .value_as_named_type()
            .map(|value| read_qualified_type_name(value.name(), context).map(TypeExpr::NamedType))
            .transpose()?
            .ok_or_else(|| format!("{context}: missing named type payload")),
        fb_class::TypeExprValue::TemplateType => {
            let value = expression
                .value_as_template_type()
                .ok_or_else(|| format!("{context}: missing template type payload"))?;
            Ok(TypeExpr::Template {
                name: read_qualified_type_name(value.name(), context)?,
                arguments: read_type_arguments(value.arguments(), context)?,
            })
        }
        fb_class::TypeExprValue::ArrayType => {
            let value = expression
                .value_as_array_type()
                .ok_or_else(|| format!("{context}: missing array type payload"))?;
            Ok(TypeExpr::Array {
                element: Box::new(read_type_expr(value.element(), context)?),
                size: value.size().map(str::to_string),
            })
        }
        fb_class::TypeExprValue::FunctionType => {
            let value = expression
                .value_as_function_type()
                .ok_or_else(|| format!("{context}: missing function type payload"))?;
            let parameter_types = value
                .parameter_types()
                .map(|parameters| {
                    parameters
                        .iter()
                        .map(|parameter| read_type_expr(parameter, context))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
                .unwrap_or_default();
            Ok(TypeExpr::Function {
                return_type: Box::new(read_type_expr(value.return_type(), context)?),
                parameter_types,
                is_variadic: value.is_variadic(),
                is_noexcept: value.is_noexcept(),
            })
        }
        fb_class::TypeExprValue::PointerType => expression
            .value_as_pointer_type()
            .map(|value| read_type_expr(value.inner(), context).map(Box::new))
            .transpose()?
            .map(TypeExpr::Pointer)
            .ok_or_else(|| format!("{context}: missing pointer type payload")),
        fb_class::TypeExprValue::ReferenceType => expression
            .value_as_reference_type()
            .map(|value| read_type_expr(value.inner(), context).map(Box::new))
            .transpose()?
            .map(TypeExpr::Reference)
            .ok_or_else(|| format!("{context}: missing reference type payload")),
        fb_class::TypeExprValue::RValueReferenceType => expression
            .value_as_rvalue_reference_type()
            .map(|value| read_type_expr(value.inner(), context).map(Box::new))
            .transpose()?
            .map(TypeExpr::RValueReference)
            .ok_or_else(|| format!("{context}: missing rvalue reference type payload")),
        fb_class::TypeExprValue::ConstType => expression
            .value_as_const_type()
            .map(|value| read_type_expr(value.inner(), context).map(Box::new))
            .transpose()?
            .map(TypeExpr::Const)
            .ok_or_else(|| format!("{context}: missing const type payload")),
        fb_class::TypeExprValue::VolatileType => expression
            .value_as_volatile_type()
            .map(|value| read_type_expr(value.inner(), context).map(Box::new))
            .transpose()?
            .map(TypeExpr::Volatile)
            .ok_or_else(|| format!("{context}: missing volatile type payload")),
        value => Err(unsupported_enum(context, "type_expression", value)),
    }
}

fn read_qualified_type_name(
    name: fb_class::QualifiedTypeName<'_>,
    context: &str,
) -> Result<QualifiedTypeName, String> {
    let segments = name
        .segments()
        .map(|segments| {
            segments
                .iter()
                .map(|segment| {
                    let Some(arguments) = segment.arguments() else {
                        return Ok(TypeNameSegment::Identifier(segment.name().to_string()));
                    };
                    Ok(TypeNameSegment::Template {
                        name: segment.name().to_string(),
                        arguments: read_type_arguments(Some(arguments), context)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(QualifiedTypeName {
        segments,
        is_global: name.is_global(),
    })
}

fn read_type_arguments(
    arguments: Option<
        flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<fb_class::TypeArgument<'_>>>,
    >,
    context: &str,
) -> Result<Vec<TypeArgument>, String> {
    arguments
        .map(|arguments| {
            arguments
                .iter()
                .map(|argument| match argument.value_type() {
                    fb_class::TypeArgumentValue::TypeExpr => argument
                        .value_as_type_expr()
                        .map(|value| read_type_expr(value, context).map(TypeArgument::Type))
                        .transpose()?
                        .ok_or_else(|| format!("{context}: missing type argument payload")),
                    fb_class::TypeArgumentValue::TemplateValue => argument
                        .value_as_template_value()
                        .map(|value| TypeArgument::Value(value.text().to_string()))
                        .ok_or_else(|| format!("{context}: missing template value payload")),
                    value => Err(unsupported_enum(context, "type_argument", value)),
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()
        .map(|arguments| arguments.unwrap_or_default())
}

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

fn read_type_aliases(
    entity: fb_class::SimpleEntity<'_>,
    path: &str,
) -> Result<Vec<TypeAlias>, String> {
    entity
        .type_aliases()
        .map(|values| {
            values
                .iter()
                .map(|value| {
                    let context =
                        format!("{path}:entity:{}:type_alias:{}", entity.id(), value.alias());
                    Ok(TypeAlias {
                        alias: value.alias().to_string(),
                        original_type: read_type_description(value.original_type(), &context)?,
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
                        data_type: read_optional_type_description(
                            value.data_type(),
                            &format!("{path}:entity:{}:variable:{}", entity.id(), value.name()),
                        )?,
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
    let parameters = read_function_arguments(
        method.parameters(),
        &format!("{path}:entity:{}:method:{}", entity.id(), method.name()),
    )?;

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
        conversion_target: method.conversion_target().map(str::to_string),
        return_type: read_optional_type_description(
            method.return_type(),
            &format!(
                "{path}:entity:{}:method:{}:return_type",
                entity.id(),
                method.name()
            ),
        )?,
        return_multiplicity: method.return_multiplicity().map(str::to_string),
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

fn read_function_arguments(
    values: Option<
        flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<fb_class::FunctionArgument<'_>>>,
    >,
    context: &str,
) -> Result<Vec<FunctionArgument>, String> {
    values
        .map(|parameters| {
            parameters
                .iter()
                .map(|parameter| {
                    Ok(FunctionArgument {
                        name: parameter.name().to_string(),
                        param_type: read_optional_type_description(
                            parameter.param_type(),
                            &format!("{context}:parameter:{}", parameter.name()),
                        )?,
                        direction: match parameter.direction() {
                            fb_class::ParameterDirection::Unspecified => None,
                            fb_class::ParameterDirection::In => Some(ParameterDirection::In),
                            fb_class::ParameterDirection::Out => Some(ParameterDirection::Out),
                            fb_class::ParameterDirection::InOut => Some(ParameterDirection::InOut),
                            value => {
                                return Err(unsupported_enum(
                                    context,
                                    "parameter_direction",
                                    value,
                                ));
                            }
                        },
                        multiplicity: parameter.multiplicity().map(|value| value.to_string()),
                        is_variadic: parameter.is_variadic(),
                        is_pack_expansion: parameter.is_pack_expansion(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()
        .map(|values| values.unwrap_or_default())
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
        id: uid_utils::normalize(entity.id()),
        name: entity.name().to_string(),
        enclosing_namespace_id: entity.enclosing_namespace_id().map(uid_utils::normalize),
        stereotypes: entity
            .stereotypes()
            .map(|values| values.iter().map(|value| value.to_string()).collect())
            .unwrap_or_default(),
        entity_type: map_entity_type(
            entity.entity_type(),
            &format!("{path}:entity:{}", entity.id()),
        )?,
        template_kind: read_template_kind(entity, &format!("{path}:entity:{}", entity.id()))?,
        type_aliases: read_type_aliases(entity, path)?,
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

fn read_free_functions(
    diagram: fb_class::ClassDiagram<'_>,
    path: &str,
) -> Result<Vec<FreeFunctionDecl>, String> {
    diagram
        .free_functions()
        .map(|values| {
            values
                .iter()
                .map(|function| {
                    Ok(FreeFunctionDecl {
                        name: function.name().to_string(),
                        enclosing_namespace_id: function
                            .enclosing_namespace_id()
                            .map(uid_utils::normalize),
                        return_type: read_optional_type_description(
                            function.return_type(),
                            &format!("{path}:free_function:{}:return_type", function.name()),
                        )?,
                        return_multiplicity: function.return_multiplicity().map(str::to_string),
                        parameters: read_function_arguments(
                            function.parameters(),
                            &format!("{path}:free_function:{}", function.name()),
                        )?,
                        template_parameters: read_template_parameters(
                            function.template_parameters(),
                            &format!("{path}:free_function:{}", function.name()),
                        )?,
                        source_location: to_source_location(
                            function.source_location().file(),
                            function.source_location().line(),
                        ),
                    })
                })
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
        source: uid_utils::normalize(rel.source()),
        target: uid_utils::normalize(rel.target()),
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

            // The PlantUML parser auto-detects diagram kind; a `.puml` may end up parsed
            // as a different type (e.g. Component). Skip mismatched buffers instead of
            // letting the verifier fail on misaligned data.
            if !fb_class::class_diagram_buffer_has_identifier(&data) {
                log::warn!("{path}: not a class-diagram, skipping validation");
                continue;
            }

            let diagram = flatbuffers::root::<fb_class::ClassDiagram>(&data)
                .map_err(|e| format!("Failed to parse class FlatBuffer {path}: {e}"))?;

            let entities = read_entities(diagram, path)?;
            let free_functions = read_free_functions(diagram, path)?;

            diagrams.push(ClassDiagram {
                source: diagram.source().map(|s| s.to_string()),
                name: diagram.name().to_string(),
                entities,
                free_functions,
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

fn read_template_kind(
    entity: fb_class::SimpleEntity<'_>,
    context: &str,
) -> Result<ClassTemplateKind, String> {
    match entity.template_kind() {
        fb_class::ClassTemplateKind::NonTemplate => Ok(ClassTemplateKind::NonTemplate),
        fb_class::ClassTemplateKind::Primary => Ok(ClassTemplateKind::Primary),
        fb_class::ClassTemplateKind::PartialSpecialization => {
            Ok(ClassTemplateKind::PartialSpecialization)
        }
        fb_class::ClassTemplateKind::ExplicitSpecialization => {
            Ok(ClassTemplateKind::ExplicitSpecialization)
        }
        value => Err(unsupported_enum(context, "template_kind", value)),
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
        fb_class::MethodModifier::Final => Ok(MethodModifier::Final),
        fb_class::MethodModifier::Const => Ok(MethodModifier::Const),
        _ => Err(unsupported_enum(context, "method_modifier", value)),
    }
}
