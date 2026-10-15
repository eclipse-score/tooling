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
pub use class_diagram::TemplateParameter;
pub use parser_core::common_ast::Arrow;
use serde::{Deserialize, Serialize};
use source_location::SourceLocation;
use std::default::Default;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Visibility {
    Public,    // "+"
    Private,   // "-"
    Protected, // "#"
    Package,   // "~"
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TypeDefinition {
    ClassLikeDef(ClassLikeDef),
    EntityDef(EntityDef),
    EnumDef(EnumDef),
}
impl TypeDefinition {
    pub fn declaration_mut(&mut self) -> &mut DeclarationCommon {
        match self {
            TypeDefinition::ClassLikeDef(def) => &mut def.declaration,
            TypeDefinition::EntityDef(def) => &mut def.declaration,
            TypeDefinition::EnumDef(def) => &mut def.declaration,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ClassUmlTopLevel {
    Type(TypeDefinition),
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
    pub alias: Option<String>,
}
impl Name {
    pub fn write_name(&mut self, internal: impl Into<String>, alias: Option<impl Into<String>>) {
        self.internal = internal.into();
        self.alias = alias.map(|d| d.into());
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Relationship {
    pub left: String,
    pub right: String,
    pub arrow: Arrow,
    pub left_multiplicity: Option<String>,
    pub right_multiplicity: Option<String>,
    pub label: Option<String>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Param {
    pub name: Option<String>,
    pub param_type: Option<String>,
    /// C-style variadic parameter represented by a standalone `...`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_c_variadic: bool,
    /// Typed template parameter pack represented by a type followed by `...`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_pack_expansion: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Attribute {
    pub visibility: Visibility,
    pub name: String,
    pub r#type: Option<String>,
    pub modifiers: Vec<String>,
    pub source_location: SourceLocation,
}
impl Default for Attribute {
    fn default() -> Self {
        Attribute {
            visibility: Visibility::Public,
            name: String::new(),
            r#type: None,
            modifiers: Vec::new(),
            source_location: SourceLocation::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct TypeAlias {
    pub alias: String,
    pub original_type: String,
    pub source_location: SourceLocation,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Method {
    pub visibility: Visibility,
    pub name: String,
    pub template_parameters: Option<Vec<TemplateParameter>>,
    pub params: Vec<Param>,
    pub r#type: Option<String>,
    pub modifiers: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_friend: bool,
    pub source_location: SourceLocation,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl Default for Method {
    fn default() -> Self {
        Method {
            visibility: Visibility::Public,
            name: String::new(),
            template_parameters: None,
            params: Vec::new(),
            r#type: None,
            modifiers: Vec::new(),
            is_friend: false,
            source_location: SourceLocation::default(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct DeclarationCommon {
    pub name: Name,
    pub namespace: String,
    pub package: String,
    #[serde(default)]
    pub stereotypes: Vec<String>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassLikeMembers {
    pub attributes: Vec<Attribute>,
    pub type_aliases: Vec<TypeAlias>,
    pub methods: Vec<Method>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassLikeDef {
    #[serde(flatten)]
    pub declaration: DeclarationCommon,
    pub template_parameters: Option<Vec<TemplateParameter>>,
    #[serde(flatten)]
    pub members: ClassLikeMembers,
    pub kind: ClassLikeKind,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ClassLikeKind {
    Class {
        is_abstract: bool,
        extends: Vec<String>,
        implements: Vec<String>,
    },
    Struct,
    Interface {
        extends: Vec<String>,
    },
}
impl Default for ClassLikeKind {
    fn default() -> Self {
        Self::Class {
            is_abstract: false,
            extends: Vec::new(),
            implements: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EntityDef {
    #[serde(flatten)]
    pub declaration: DeclarationCommon,
    pub methods: Vec<Method>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EnumDef {
    #[serde(flatten)]
    pub declaration: DeclarationCommon,
    pub items: Vec<EnumItem>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EnumItem {
    pub name: String,
    pub value: Option<EnumValue>,
    pub source_location: SourceLocation,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Namespace {
    pub name: Name,
    pub source_location: SourceLocation,
    pub types: Vec<TypeDefinition>,
    pub relationships: Vec<Relationship>,
    pub namespaces: Vec<Namespace>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Package {
    pub name: Name,
    pub source_location: SourceLocation,
    pub types: Vec<TypeDefinition>,
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
        let mut class = ClassLikeDef::default();
        class.declaration.name.internal = "TestClass".into();

        let mut element = TypeDefinition::ClassLikeDef(class);

        element.declaration_mut().namespace = "ns1".into();
        element.declaration_mut().package = "pkg1".into();

        let TypeDefinition::ClassLikeDef(def) = element else {
            unreachable!();
        };
        assert_eq!(def.declaration.namespace, "ns1");
        assert_eq!(def.declaration.package, "pkg1");
    }

    #[test]
    fn test_name_write() {
        let mut name = Name::default();

        name.write_name("InternalName", Some("AliasName"));

        assert_eq!(name.internal, "InternalName");
        assert_eq!(name.alias, Some("AliasName".to_string()));
    }

    #[test]
    fn test_name_write_without_alias() {
        let mut name = Name::default();

        name.write_name("OnlyInternal", None::<String>);

        assert_eq!(name.internal, "OnlyInternal");
        assert_eq!(name.alias, None);
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
        assert_eq!(method.template_parameters, None);
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
            left_multiplicity: None,
            right_multiplicity: None,
            label: None,
            source_location: SourceLocation::new("test.puml", 0),
        });

        assert!(!file.is_empty());
    }

    #[test]
    fn test_enum_def() {
        let mut enum_def = EnumDef::default();

        enum_def.declaration.name.internal = "Color".into();
        enum_def.items.push(EnumItem {
            name: "RED".into(),
            value: Some(EnumValue::Literal("1".into())),
            source_location: SourceLocation::new("test.puml", 0),
        });

        assert_eq!(enum_def.declaration.name.internal, "Color");
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
            left_multiplicity: None,
            right_multiplicity: None,
            label: Some("uses".into()),
            source_location: SourceLocation::new("test.puml", 0),
        });

        assert_eq!(pkg.relationships.len(), 1);
    }

    #[test]
    fn test_element_all_variants_set_namespace() {
        let mut elements = vec![
            TypeDefinition::ClassLikeDef(ClassLikeDef::default()),
            TypeDefinition::EnumDef(EnumDef::default()),
            TypeDefinition::EntityDef(EntityDef::default()),
        ];

        for el in elements.iter_mut() {
            el.declaration_mut().namespace = "test_ns".into();
        }

        for el in elements {
            match el {
                TypeDefinition::ClassLikeDef(d) => assert_eq!(d.declaration.namespace, "test_ns"),
                TypeDefinition::EnumDef(d) => assert_eq!(d.declaration.namespace, "test_ns"),
                TypeDefinition::EntityDef(d) => assert_eq!(d.declaration.namespace, "test_ns"),
            }
        }
    }

    #[test]
    fn test_class_like_members_methods() {
        let mut class = ClassLikeDef::default();

        class.members.methods.push(Method {
            name: "foo".into(),
            ..Default::default()
        });

        assert_eq!(class.members.methods.len(), 1);
        assert_eq!(class.members.methods[0].name, "foo");
    }

    #[test]
    fn test_class_like_members_attributes() {
        let mut class = ClassLikeDef::default();

        class.members.attributes.push(Attribute {
            name: "field".into(),
            ..Default::default()
        });

        assert_eq!(class.members.attributes.len(), 1);
    }

    #[test]
    fn test_class_like_members_type_aliases() {
        let mut class = ClassLikeDef::default();

        class.members.type_aliases.push(TypeAlias {
            alias: "Byte".into(),
            original_type: "std::uint8_t".into(),
            ..Default::default()
        });

        assert_eq!(class.members.type_aliases.len(), 1);
        assert_eq!(class.members.type_aliases[0].alias, "Byte");
    }

    #[test]
    fn test_type_def_declaration() {
        let mut class = ClassLikeDef::default();

        class.declaration.name.internal = "MyClass".into();

        assert_eq!(class.declaration.name.internal, "MyClass");
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
    fn test_name_write_accepts_owned_and_borrowed_variants() {
        let mut name = Name {
            internal: "abc".into(),
            ..Default::default()
        };

        name.write_name("abc", None::<String>);
        assert_eq!(name.internal, "abc");
        assert_eq!(name.alias, None);

        name.write_name("abc", Some("ABC"));
        assert_eq!(name.internal, "abc");
        assert_eq!(name.alias, Some("ABC".to_string()));

        let internal = String::from("xyz");
        let alias = String::from("XYZ");

        name.write_name(internal, Some(alias));

        assert_eq!(name.internal, "xyz");
        assert_eq!(name.alias, Some("XYZ".to_string()));
    }

    #[test]
    fn test_type_declaration_kind_default_is_class() {
        assert_eq!(
            ClassLikeDef::default().kind,
            ClassLikeKind::Class {
                is_abstract: false,
                extends: vec![],
                implements: vec![],
            }
        );
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
        assert_eq!(name.alias, None);

        name.write_name("abc", Some(String::from("x")));
        assert_eq!(name.internal, "abc");
        assert_eq!(name.alias, Some("x".to_string()));

        name.write_name("abc", None::<&str>);
        assert_eq!(name.internal, "abc");
        assert_eq!(name.alias, None);

        name.write_name("abc", Some("x"));
        assert_eq!(name.internal, "abc");
        assert_eq!(name.alias, Some("x".to_string()));

        let mut type_def = ClassLikeDef::default();
        type_def.members.methods.push(Method {
            name: "mixed_method".into(),
            ..Default::default()
        });
        assert_eq!(type_def.members.methods.len(), 1);
        assert_eq!(type_def.members.methods[0].name, "mixed_method");

        let file1 = ClassUmlFile::default();
        assert_eq!(file1.as_ref(), "");
        assert!(file1.is_empty());

        let file2 = ClassUmlFile::default();
        assert_eq!(file2.as_ref(), "");
        assert!(file2.is_empty());
    }
}
