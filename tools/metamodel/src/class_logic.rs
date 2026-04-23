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
use serde::{Deserialize, Serialize};

/// Represents a complete class diagram model containing all resolved entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ClassDiagram {
    pub name: String,
    pub entities: Vec<SimpleEntity>,
    // all relationships in the entire diagram
    pub relationships: Vec<Relationship>, // would make sense inside entities

    pub source_files: Vec<String>,
    pub version: Option<String>,
}

/// Represents a class, struct, interface, enum, or other type entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SimpleEntity {
    /// Fully Qualified Name (FQN) - unique identifier including namespace path
    // mw::com::test{
    //     int bla
    // } -> mw.com.test.bla
    //
    //package core::geometry <<subdomain>> {
    // class Circle <<entity>> {
    // "id": "core.geometry.Circle",
    pub id: String,
    /// Display name (may differ from id when alias is used)
    /// just variable name in c++ and alias in plantuml (if alias does not exist label name is the
    /// fallback)
    pub name: String,

    /// FQN of parent namespace/package
    /// enclosing namespace name
    pub enclosing_namespace_id: Option<String>,
    /// Type of entity (class, struct, interface, enum, etc.)
    pub entity_type: EntityType,

    // aliased type with using keyword also called annotaion in plantuml
    pub type_aliases: Vec<TypeAlias>,
    pub variables: Vec<MemberVariable>,
    /// Methods (member functions)
    pub methods: Vec<Method>,
    /// Template parameters for generic types (empty option means not templated) empty vector means
    /// template is an ampty bracket like template<> which can be encountered during explicit template specialization
    pub template_parameters: Option<Vec<String>>,

    /// Enum literals (only for Enum entity_type)
    pub enum_literals: Vec<EnumLiteral>,

    // all relationships for current entity
    pub relationships: Vec<Relationship>,

    /// Debug info for display in case of missmatch
    ///
    /// Source file location
    pub source_file: Option<String>,
    /// 1-based line number in source; `None` means the source line is unknown
    pub source_line: Option<u32>,
    // pub relstionships: Vec<LogicRelationship>, // relationships where this entity is the source
}

/// The type of entity in a class diagram
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum EntityType {
    /// Standard class
    #[default]
    Class,
    /// Data structure (typically POD in C++)
    Struct,

    /// Abstract interface
    Interface,
    /// Abstract class
    AbstractClass,

    /// Enumeration
    Enum,
}

/// Visibility modifier for members
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// Public visibility (+)
    #[default]
    Public,
    /// Private visibility (-)
    Private,
    /// Protected visibility (#)
    Protected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TypeAlias {
    /// example: using Number = double;
    /// alias = "Number"
    /// original_type = "double"
    pub alias: String,
    pub original_type: String,
}

/// Represents a class attribute (member variable)
/// renamed from LogicAttribute
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MemberVariable {
    /// Attribute name
    pub name: String,
    /// Data type (e.g., "int", "string", "std::vector<int>")
    pub data_type: Option<String>,
    /// Visibility modifier
    pub visibility: Visibility,
    /// Whether this is a static member
    pub is_static: bool,
    /// Whether this is a const member
    pub is_const: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArgumentModifier {
    Const,
    Pointer,
    LValueReference,
    RValueReference,
    Variadic,
}

impl ArgumentModifier {
    pub fn make_modifier_vec(
        is_const: bool,
        is_pointer: bool,
        is_lvalue_reference: bool,
        is_rvalue_reference: bool,
        is_variadic: bool,
    ) -> Vec<ArgumentModifier> {
        let mut modifiers = Vec::new();
        if is_const {
            modifiers.push(ArgumentModifier::Const);
        }
        if is_pointer {
            modifiers.push(ArgumentModifier::Pointer);
        }
        if is_lvalue_reference {
            modifiers.push(ArgumentModifier::LValueReference);
        }
        if is_rvalue_reference {
            modifiers.push(ArgumentModifier::RValueReference);
        }
        if is_variadic {
            modifiers.push(ArgumentModifier::Variadic);
        }
        modifiers
    }
}

/// Represents a method parameter
/// renamed from LogicParameter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FunctionArgument {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: Option<String>,
    pub modifiers: Vec<ArgumentModifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MethodModifier {
    Static,
    Const,
    Virtual,
    Abstract,
    Override,
    Constructor,
    Destructor,
    Noexcept,
}

impl MethodModifier {
    pub fn make_modifier_vec(
        is_static: bool,
        is_const: bool,
        is_virtual: bool,
        is_abstract: bool,
        is_override: bool,
        is_constructor: bool,
        is_destructor: bool,
    ) -> Vec<MethodModifier> {
        let mut modifiers = Vec::new();
        if is_static {
            modifiers.push(MethodModifier::Static);
        }
        if is_const {
            modifiers.push(MethodModifier::Const);
        }
        if is_virtual {
            modifiers.push(MethodModifier::Virtual);
        }
        if is_abstract {
            modifiers.push(MethodModifier::Abstract);
        }
        if is_override {
            modifiers.push(MethodModifier::Override);
        }
        if is_constructor {
            modifiers.push(MethodModifier::Constructor);
        }
        if is_destructor {
            modifiers.push(MethodModifier::Destructor);
        }
        modifiers
    }
}

/// Represents a class method (member function)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Method {
    /// Method name
    pub name: String,
    /// Return type
    pub return_type: Option<String>,
    /// Visibility modifier
    pub visibility: Visibility,
    /// Method parameters
    pub parameters: Vec<FunctionArgument>,
    /// Template parameters for generic methods
    pub template_parameters: Option<Vec<String>>,
    pub modifiers: Vec<MethodModifier>,
}

/// Represents a relationship between two entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Relationship {
    /// FQN of the source entity
    pub source: String,
    /// FQN of the target entity
    pub target: String,
    /// Type of relationship
    pub relation_type: RelationType,

    // NOTE:  these might not be used for validation
    //
    /// Source multiplicity (e.g., "1", "0..*", "1..n")
    pub source_multiplicity: Option<String>,
    /// Target multiplicity
    pub target_multiplicity: Option<String>,
}

/// Types of relationships in class diagrams
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum RelationType {
    /// Inheritance (extends) - `<|--` or `--|>`
    #[default]
    Inheritance,
    /// Interface implementation - `<|..` or `..|>`
    Implementation,
    /// Composition (strong ownership) - `*--`
    Composition,
    /// Aggregation (weak ownership) - `o--`
    Aggregation,
    /// Directed association - `-->` (depends on  A --> B means A depends on B)
    Association,
    /// Dependency (uses) - `..>`
    Dependency,
}

/// Represents an enum literal/value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EnumLiteral {
    /// Literal name
    pub name: String,
    /// Explicit value (e.g., `HIGH = 0`)
    pub value: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_serialization() {
        let entity = SimpleEntity {
            id: "Core::User".to_string(),
            name: "User".to_string(),
            enclosing_namespace_id: Some("Core".to_string()),
            entity_type: EntityType::Class,
            variables: vec![MemberVariable {
                name: "name".to_string(),
                data_type: Some("string".to_string()),
                visibility: Visibility::Public,
                is_static: false,
                is_const: false,
            }],
            methods: vec![Method {
                name: "getName".to_string(),
                return_type: Some("string".to_string()),
                visibility: Visibility::Public,
                parameters: vec![],
                template_parameters: None,
                modifiers: vec![MethodModifier::Const],
            }],
            template_parameters: None,
            enum_literals: vec![],
            source_file: None,
            source_line: None,
            type_aliases: vec![],
            relationships: vec![],
        };

        let json = serde_json::to_string_pretty(&entity).unwrap();
        let deserialized: SimpleEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(entity, deserialized);
    }

    #[test]
    fn test_relationship_types() {
        let inheritance = Relationship {
            source: "Derived".to_string(),
            target: "Base".to_string(),
            relation_type: RelationType::Inheritance,
            source_multiplicity: None,
            target_multiplicity: None,
        };

        assert_eq!(inheritance.relation_type, RelationType::Inheritance);
    }

    #[test]
    fn test_partial_plantuml_entity() {
        // PlantUML often has incomplete information - this should still work
        let entity = SimpleEntity {
            id: "UserService".to_string(),
            name: "UserService".to_string(),
            entity_type: EntityType::Class,
            // No attributes specified (common in high-level diagrams)
            // Method with no return type (PlantUML allows this)
            methods: vec![Method {
                name: "getUser".to_string(),
                return_type: None, // <-- Often omitted in PlantUML
                visibility: Visibility::Public,
                parameters: vec![],
                template_parameters: None,
                modifiers: vec![],
            }],
            ..Default::default()
        };

        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: SimpleEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(entity, deserialized);
        assert!(entity.methods[0].return_type.is_none());
    }
}
