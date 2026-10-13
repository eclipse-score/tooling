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
pub mod class_ast;
mod class_parser;
mod source_map;

pub use class_ast::{
    Attribute, ClassLikeDef, ClassLikeKind, ClassLikeMembers, ClassUmlFile, ClassUmlTopLevel,
    DeclarationCommon, EntityDef, EnumDef, EnumItem, EnumValue, Method, Name, Namespace, Package,
    Param, Relationship, TypeAlias, TypeDefinition, Visibility,
};
pub use class_parser::{ClassError, PumlClassParser};
