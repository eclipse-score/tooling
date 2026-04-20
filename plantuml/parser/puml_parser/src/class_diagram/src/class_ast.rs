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
    ObjectDef(ObjectDef),
}
impl Element {
    pub fn set_namespace(&mut self, ns: String) {
        match self {
            Element::ClassDef(def) => def.namespace = ns,
            Element::StructDef(def) => def.namespace = ns,
            Element::EnumDef(def) => def.namespace = ns,
            Element::InterfaceDef(def) => def.namespace = ns,
            Element::ObjectDef(def) => def.namespace = ns,
        }
    }
    pub fn set_package(&mut self, ns: String) {
        match self {
            Element::ClassDef(def) => def.package = ns,
            Element::StructDef(def) => def.package = ns,
            Element::EnumDef(def) => def.package = ns,
            Element::InterfaceDef(def) => def.package = ns,
            Element::ObjectDef(def) => def.package = ns,
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
    pub param_type: Option<String>,
    pub varargs: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Attribute {
    pub visibility: Visibility,
    pub name: String,
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<String>,
}
impl Default for Attribute {
    fn default() -> Self {
        Attribute {
            visibility: Visibility::Public,
            name: String::new(),
            r#type: None,
            modifiers: Vec::new(),
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<String>,
}
impl Default for Method {
    fn default() -> Self {
        Method {
            visibility: Visibility::Public,
            name: String::new(),
            generic_params: Vec::new(),
            params: Vec::new(),
            r#type: None,
            modifiers: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassDef {
    pub name: Name,
    pub namespace: String,
    pub package: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub template_params: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extends: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implements: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub template_params: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub template_params: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extends: Vec<String>,
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
pub struct ObjectDef {
    pub name: Name,
    pub namespace: String,
    pub package: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub template_params: Vec<String>,
    pub attributes: Vec<Attribute>,
    pub methods: Vec<Method>,
}
impl TypeDef for ObjectDef {
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

#[cfg(test)]
mod tests {
    use super::*;
    use parser_core::common_ast::ArrowLine;

    #[test]
    fn test_element_set_namespace_and_package() {
        let mut class = ClassDef::default();
        class.name.internal = "TestClass".into();

        let mut element = Element::ClassDef(class);

        element.set_namespace("ns1".into());
        element.set_package("pkg1".into());

        let Element::ClassDef(def) = element else {
            unreachable!();
        };
        assert_eq!(def.namespace, "ns1");
        assert_eq!(def.package, "pkg1");
    }

    #[test]
    fn test_name_write() {
        let mut name = Name::default();

        name.write_name("InternalName", Some("DisplayName"));

        assert_eq!(name.internal, "InternalName");
        assert_eq!(name.display, Some("DisplayName".to_string()));
    }

    #[test]
    fn test_name_write_without_display() {
        let mut name = Name::default();

        name.write_name("OnlyInternal", None::<String>);

        assert_eq!(name.internal, "OnlyInternal");
        assert_eq!(name.display, None);
    }

    #[test]
    fn test_attribute_default() {
        let attr = Attribute::default();

        assert_eq!(attr.visibility, Visibility::Public);
        assert_eq!(attr.name, "");
        assert_eq!(attr.r#type, None);
        assert!(attr.modifiers.is_empty());
    }

    #[test]
    fn test_method_default() {
        let method = Method::default();

        assert_eq!(method.visibility, Visibility::Public);
        assert_eq!(method.name, "");
        assert!(method.generic_params.is_empty());
        assert!(method.params.is_empty());
        assert_eq!(method.r#type, None);
        assert!(method.modifiers.is_empty());
    }

    #[test]
    fn test_class_uml_file_is_empty() {
        let file = ClassUmlFile::default();
        assert!(file.is_empty());
    }

    #[test]
    fn test_class_uml_file_not_empty_elements() {
        let mut file = ClassUmlFile::default();

        file.elements
            .push(ClassUmlTopLevel::Namespace(Namespace::default()));

        assert!(!file.is_empty());
    }

    #[test]
    fn test_class_uml_file_not_empty_relationships() {
        let mut file = ClassUmlFile::default();

        file.relationships.push(Relationship {
            left: "A".into(),
            right: "B".into(),
            arrow: Arrow {
                left: None,
                line: ArrowLine { raw: "-->".into() },
                middle: None,
                right: None,
            },
            label: None,
        });

        assert!(!file.is_empty());
    }

    #[test]
    fn test_enum_def() {
        let mut enum_def = EnumDef::default();

        enum_def.name.internal = "Color".into();
        enum_def.items.push(EnumItem {
            visibility: Some(Visibility::Public),
            name: "RED".into(),
            value: Some(EnumValue::Literal("1".into())),
        });

        assert_eq!(enum_def.name.internal, "Color");
        assert_eq!(enum_def.items.len(), 1);
    }

    #[test]
    fn test_namespace_nested() {
        let mut root = Namespace::default();
        root.name.internal = "root".into();

        let mut child = Namespace::default();
        child.name.internal = "child".into();

        root.namespaces.push(child);

        assert_eq!(root.namespaces.len(), 1);
        assert_eq!(root.namespaces[0].name.internal, "child");
    }

    #[test]
    fn test_package_relationships() {
        let mut pkg = Package::default();

        pkg.relationships.push(Relationship {
            left: "A".into(),
            right: "B".into(),
            arrow: Arrow {
                left: None,
                line: ArrowLine { raw: "-->".into() },
                middle: None,
                right: None,
            },
            label: Some("uses".into()),
        });

        assert_eq!(pkg.relationships.len(), 1);
    }

    #[test]
    fn test_element_all_variants_set_namespace() {
        let mut elements = vec![
            Element::ClassDef(ClassDef::default()),
            Element::StructDef(StructDef::default()),
            Element::EnumDef(EnumDef::default()),
            Element::InterfaceDef(InterfaceDef::default()),
            Element::ObjectDef(ObjectDef::default()),
        ];

        for el in elements.iter_mut() {
            el.set_namespace("test_ns".into());
        }

        for el in elements {
            match el {
                Element::ClassDef(d) => assert_eq!(d.namespace, "test_ns"),
                Element::StructDef(d) => assert_eq!(d.namespace, "test_ns"),
                Element::EnumDef(d) => assert_eq!(d.namespace, "test_ns"),
                Element::InterfaceDef(d) => assert_eq!(d.namespace, "test_ns"),
                Element::ObjectDef(d) => assert_eq!(d.namespace, "test_ns"),
            }
        }
    }

    #[test]
    fn test_methods_mut() {
        let mut class = ClassDef::default();

        class.methods_mut().push(Method {
            name: "foo".into(),
            ..Default::default()
        });

        assert_eq!(class.methods.len(), 1);
        assert_eq!(class.methods[0].name, "foo");
    }

    #[test]
    fn test_attributes_mut() {
        let mut class = ClassDef::default();

        class.attributes_mut().push(Attribute {
            name: "field".into(),
            ..Default::default()
        });

        assert_eq!(class.attributes.len(), 1);
    }

    #[test]
    fn test_name_mut() {
        let mut class = ClassDef::default();

        class.name_mut().internal = "MyClass".into();

        assert_eq!(class.name.internal, "MyClass");
    }

    #[test]
    fn test_as_ref() {
        let file = ClassUmlFile {
            name: "test_file".into(),
            ..Default::default()
        };

        let name: &str = file.as_ref();

        assert_eq!(name, "test_file");
    }

    #[test]
    fn test_struct_methods_mut() {
        let mut s = StructDef::default();

        s.methods_mut().push(Method::default());

        assert_eq!(s.methods.len(), 1);
    }

    #[test]
    fn test_interface_methods_mut() {
        let mut i = InterfaceDef::default();

        i.methods_mut().push(Method::default());

        assert_eq!(i.methods.len(), 1);
    }

    #[test]
    fn test_object_methods_mut() {
        let mut object = ObjectDef::default();

        object.methods_mut().push(Method::default());

        assert_eq!(object.methods.len(), 1);
    }

    #[test]
    fn test_typedef_trait_object_calls_for_class_def() {
        use crate::class_traits::TypeDef;

        let mut c = ClassDef::default();

        {
            let obj: &mut dyn TypeDef = &mut c;

            obj.name_mut().internal = "ClassViaTrait".into();
            obj.attributes_mut().push(Attribute {
                name: "field".into(),
                ..Default::default()
            });
            obj.methods_mut().push(Method {
                name: "method".into(),
                ..Default::default()
            });
        }

        assert_eq!(c.name.internal, "ClassViaTrait");
        assert_eq!(c.attributes.len(), 1);
        assert_eq!(c.attributes[0].name, "field");
        assert_eq!(c.methods.len(), 1);
        assert_eq!(c.methods[0].name, "method");
    }

    #[test]
    fn test_typedef_trait_object_calls_for_struct_def() {
        use crate::class_traits::TypeDef;

        let mut s = StructDef::default();

        {
            let obj: &mut dyn TypeDef = &mut s;

            obj.name_mut().internal = "StructViaTrait".into();
            obj.attributes_mut().push(Attribute {
                name: "field".into(),
                ..Default::default()
            });
            obj.methods_mut().push(Method {
                name: "method".into(),
                ..Default::default()
            });
        }

        assert_eq!(s.name.internal, "StructViaTrait");
        assert_eq!(s.attributes.len(), 1);
        assert_eq!(s.attributes[0].name, "field");
        assert_eq!(s.methods.len(), 1);
        assert_eq!(s.methods[0].name, "method");
    }

    #[test]
    fn test_typedef_trait_object_calls_for_interface_def() {
        use crate::class_traits::TypeDef;

        let mut i = InterfaceDef::default();

        {
            let obj: &mut dyn TypeDef = &mut i;

            obj.name_mut().internal = "InterfaceViaTrait".into();
            obj.attributes_mut().push(Attribute {
                name: "field".into(),
                ..Default::default()
            });
            obj.methods_mut().push(Method {
                name: "method".into(),
                ..Default::default()
            });
        }

        assert_eq!(i.name.internal, "InterfaceViaTrait");
        assert_eq!(i.attributes.len(), 1);
        assert_eq!(i.attributes[0].name, "field");
        assert_eq!(i.methods.len(), 1);
        assert_eq!(i.methods[0].name, "method");
    }

    #[test]
    fn test_typedef_trait_object_calls_for_object_def() {
        use crate::class_traits::TypeDef;

        let mut object = ObjectDef::default();

        {
            let obj: &mut dyn TypeDef = &mut object;

            obj.name_mut().internal = "ObjectViaTrait".into();
            obj.attributes_mut().push(Attribute {
                name: "field".into(),
                ..Default::default()
            });
            obj.methods_mut().push(Method {
                name: "method".into(),
                ..Default::default()
            });
        }

        assert_eq!(object.name.internal, "ObjectViaTrait");
        assert_eq!(object.attributes.len(), 1);
        assert_eq!(object.attributes[0].name, "field");
        assert_eq!(object.methods.len(), 1);
        assert_eq!(object.methods[0].name, "method");
    }

    #[test]
    fn test_class_uml_file_supports_generic_as_ref_usage() {
        fn use_as_ref<T: AsRef<str>>(v: T) -> String {
            v.as_ref().to_string()
        }

        let file = ClassUmlFile {
            name: "abc".into(),
            ..Default::default()
        };

        let s = use_as_ref(file);

        assert_eq!(s, "abc");
    }

    #[test]
    fn test_class_uml_file_is_empty_for_empty_named_file() {
        let file = ClassUmlFile {
            name: "x".into(),
            elements: vec![],
            relationships: vec![],
        };

        assert!(file.is_empty());
    }

    #[test]
    fn test_name_supports_generic_write_name_usage() {
        fn call_write<T: WritableName>(mut t: T) -> T {
            t.write_name("a", Some("b"));
            t
        }

        let name = Name::default();
        let name = call_write(name);

        assert_eq!(name.internal, "a");
    }

    #[test]
    fn test_name_write_accepts_owned_and_borrowed_variants() {
        let mut name = Name {
            internal: "abc".into(),
            ..Default::default()
        };

        name.write_name("abc", None::<String>);
        assert_eq!(name.internal, "abc");
        assert_eq!(name.display, None);

        name.write_name("abc", Some("ABC"));
        assert_eq!(name.internal, "abc");
        assert_eq!(name.display, Some("ABC".to_string()));

        let internal = String::from("xyz");
        let display = String::from("XYZ");

        name.write_name(internal, Some(display));

        assert_eq!(name.internal, "xyz");
        assert_eq!(name.display, Some("XYZ".to_string()));
    }

    #[test]
    fn test_struct_methods_mut_returns_mutable_reference() {
        let mut s = StructDef::default();

        s.methods_mut().push(Method {
            name: "struct_method".into(),
            ..Default::default()
        });

        assert_eq!(s.methods.len(), 1);
        assert_eq!(s.methods[0].name, "struct_method");
    }

    #[test]
    fn test_class_uml_file_default_supports_as_ref_and_is_empty() {
        let file1 = ClassUmlFile::default();
        assert_eq!(file1.as_ref(), "");
        assert!(file1.is_empty());

        let file2 = ClassUmlFile::default();
        assert_eq!(file2.as_ref(), "");
        assert!(file2.is_empty());
    }

    #[test]
    fn test_generic_trait_calls_support_mixed_input_variants() {
        let mut name = Name::default();

        name.write_name("abc", None::<String>);
        assert_eq!(name.internal, "abc");
        assert_eq!(name.display, None);

        name.write_name("abc", Some(String::from("x")));
        assert_eq!(name.internal, "abc");
        assert_eq!(name.display, Some("x".to_string()));

        name.write_name("abc", None::<&str>);
        assert_eq!(name.internal, "abc");
        assert_eq!(name.display, None);

        name.write_name("abc", Some("x"));
        assert_eq!(name.internal, "abc");
        assert_eq!(name.display, Some("x".to_string()));

        let mut s = StructDef::default();
        s.methods_mut().push(Method {
            name: "mixed_method".into(),
            ..Default::default()
        });
        assert_eq!(s.methods.len(), 1);
        assert_eq!(s.methods[0].name, "mixed_method");

        let file1 = ClassUmlFile::default();
        assert_eq!(file1.as_ref(), "");
        assert!(file1.is_empty());

        let file2 = ClassUmlFile::default();
        assert_eq!(file2.as_ref(), "");
        assert!(file2.is_empty());
    }
}
