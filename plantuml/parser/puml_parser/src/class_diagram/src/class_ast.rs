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
pub use parser_core::common_ast::Arrow;
use serde::{Deserialize, Serialize};
use std::default::Default;

use crate::class_traits::{TypeDef, WritableName};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Visibility {
    Public,    // "+"
    Private,   // "-"
    Protected, // "#"
    Package,   // "~"
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Element {
    ClassDef(ClassDef),
    StructDef(StructDef),
    EnumDef(EnumDef),
    InterfaceDef(InterfaceDef),
}
impl Element {
    pub fn set_namespace(&mut self, ns: String) {
        match self {
            Element::ClassDef(def) => def.namespace = ns,
            Element::StructDef(def) => def.namespace = ns,
            Element::EnumDef(def) => def.namespace = ns,
            Element::InterfaceDef(def) => def.namespace = ns,
        }
    }
    pub fn set_package(&mut self, ns: String) {
        match self {
            Element::ClassDef(def) => def.package = ns,
            Element::StructDef(def) => def.package = ns,
            Element::EnumDef(def) => def.package = ns,
            Element::InterfaceDef(def) => def.package = ns,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ClassUmlTopLevel {
    Types(Element),
    Enum(EnumDef),
    Namespace(Namespace),
    Package(Package),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum EnumValue {
    Literal(String),
    Description(String),
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Name {
    pub internal: String,
    pub display: Option<String>,
}
impl WritableName for Name {
    fn write_name(&mut self, internal: impl Into<String>, display: Option<impl Into<String>>) {
        self.internal = internal.into();
        self.display = display.map(|d| d.into());
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Relationship {
    pub left: String,
    pub right: String,
    pub arrow: Arrow,
    pub label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Param {
    pub name: Option<String>,
    pub param_type: String,
    pub varargs: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Attribute {
    pub visibility: Visibility,
    pub name: String,
    pub r#type: Option<String>,
}
impl Default for Attribute {
    fn default() -> Self {
        Attribute {
            visibility: Visibility::Public,
            name: String::new(),
            r#type: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Method {
    pub visibility: Visibility,
    pub name: String,
    pub generic_params: Vec<String>,
    pub params: Vec<Param>,
    pub r#type: Option<String>,
}
impl Default for Method {
    fn default() -> Self {
        Method {
            visibility: Visibility::Public,
            name: String::new(),
            generic_params: Vec::new(),
            params: Vec::new(),
            r#type: None,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassDef {
    pub name: Name,
    pub namespace: String,
    pub package: String,
    pub attributes: Vec<Attribute>,
    pub methods: Vec<Method>,
}
impl TypeDef for ClassDef {
    fn name_mut(&mut self) -> &mut Name {
        &mut self.name
    }

    fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
        &mut self.attributes
    }

    fn methods_mut(&mut self) -> &mut Vec<Method> {
        &mut self.methods
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct StructDef {
    pub name: Name,
    pub namespace: String,
    pub package: String,
    pub attributes: Vec<Attribute>,
    pub methods: Vec<Method>,
}
impl TypeDef for StructDef {
    fn name_mut(&mut self) -> &mut Name {
        &mut self.name
    }

    fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
        &mut self.attributes
    }

    fn methods_mut(&mut self) -> &mut Vec<Method> {
        &mut self.methods
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct InterfaceDef {
    pub name: Name,
    pub namespace: String,
    pub package: String,
    pub attributes: Vec<Attribute>,
    pub methods: Vec<Method>,
}
impl TypeDef for InterfaceDef {
    fn name_mut(&mut self) -> &mut Name {
        &mut self.name
    }

    fn attributes_mut(&mut self) -> &mut Vec<Attribute> {
        &mut self.attributes
    }

    fn methods_mut(&mut self) -> &mut Vec<Method> {
        &mut self.methods
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EnumDef {
    pub name: Name,
    pub namespace: String,
    pub package: String,
    pub stereotypes: Vec<String>,
    pub items: Vec<EnumItem>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EnumItem {
    pub visibility: Option<Visibility>,
    pub name: String,
    pub value: Option<EnumValue>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Namespace {
    pub name: Name,
    pub types: Vec<Element>,
    pub namespaces: Vec<Namespace>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Package {
    pub name: Name,
    pub types: Vec<Element>,
    pub relationships: Vec<Relationship>,
    pub packages: Vec<Package>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassUmlFile {
    pub name: String,
    pub elements: Vec<ClassUmlTopLevel>,
    pub relationships: Vec<Relationship>,
}
impl ClassUmlFile {
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty() && self.relationships.is_empty()
    }
}
impl AsRef<str> for ClassUmlFile {
    fn as_ref(&self) -> &str {
        &self.name
    }
}
