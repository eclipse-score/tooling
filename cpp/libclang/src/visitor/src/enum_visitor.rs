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
use crate::class_visitor::parse_source_location;
use crate::context::VisitContext;
use crate::visitor::AstVisitor;
use clang::Entity;
use class_diagram::{EntityType, EnumLiteral, SimpleEntity};

pub struct EnumVisitor;

impl AstVisitor for EnumVisitor {
    fn visit(ctx: &mut VisitContext, entity: Entity) {
        if let Some(enum_entity) = Self::visit_enum(entity) {
            ctx.types.insert(enum_entity.id.clone(), enum_entity);
        }
    }
}

impl EnumVisitor {
    fn visit_enum(entity: Entity) -> Option<SimpleEntity> {
        let name = entity.get_name()?;
        let namespace_id = Self::get_namespace_id(&entity);
        let full_qualified_id = if let Some(namespace_id) = &namespace_id {
            format!("{}::{}", namespace_id, name)
        } else {
            name.clone()
        };
        let source_location = parse_source_location(&entity);

        Some(SimpleEntity {
            id: full_qualified_id,
            name,
            enclosing_namespace_id: namespace_id,
            entity_type: EntityType::Enum,
            enum_literals: get_literals(entity),
            source_location,
            ..Default::default()
        })
    }
}

fn get_literals(entity: Entity) -> Vec<EnumLiteral> {
    let check_if_underlying_type_is_unsigned = |entity: Entity| {
        if let Some(underlying_type) = entity.get_enum_underlying_type() {
            let expanded_type = underlying_type.get_canonical_type().get_kind();
            let type_kind_is_unsigned = expanded_type == clang::TypeKind::UChar
                || expanded_type == clang::TypeKind::UShort
                || expanded_type == clang::TypeKind::UInt
                || expanded_type == clang::TypeKind::ULong
                || expanded_type == clang::TypeKind::ULongLong;
            return type_kind_is_unsigned;
        }
        false
    };

    let is_unsigned = check_if_underlying_type_is_unsigned(entity);
    entity
        .get_children()
        .into_iter()
        .filter(|c| c.get_kind() == clang::EntityKind::EnumConstantDecl)
        .filter_map(|c| {
            let constant_val_wrapped = c.get_enum_constant_value();
            let constant_val = if let Some(constant_val_tuple) = constant_val_wrapped {
                // NOTE: we upcast to i128 to guarantee that any value of u64 and i64 can be represented (in the
                // end everything will be serialized as a json number
                if is_unsigned {
                    Some(constant_val_tuple.1 as i128)
                } else {
                    Some(constant_val_tuple.0 as i128)
                }
            } else {
                return None;
            };

            Some(EnumLiteral {
                name: c.get_name()?,
                value: constant_val,
                source_location: parse_source_location(&c),
            })
        })
        .collect()
}
