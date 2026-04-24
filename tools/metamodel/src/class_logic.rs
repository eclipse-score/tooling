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
    pub entities: Vec<LogicEntity>,
    pub containers: Vec<LogicContainer>,
    pub relationships: Vec<LogicRelationship>,
    pub source_files: Vec<String>,
    pub version: Option<String>,
}

/// Represents a class, struct, interface, enum, or other type entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LogicEntity {
    /// Fully Qualified Name (FQN) - unique identifier including namespace path
    pub id: String,
    /// Display name (may differ from id when alias is used)
    pub name: Option<String>,
    /// Short alias for referencing (e.g., `class "LongName" as Alias`)
    pub alias: Option<String>,
    /// FQN of parent namespace/package
    pub parent_id: Option<String>,
    /// Type of entity (class, struct, interface, enum, etc.)
    pub entity_type: EntityType,
    /// Stereotypes applied to this entity (e.g., <<Model>>, <<Singleton>>)
    pub stereotypes: Vec<String>,
    /// Attributes (member variables)
    pub attributes: Vec<LogicAttribute>,
    /// Methods (member functions)
    pub methods: Vec<LogicMethod>,
    /// Template parameters for generic types
    pub template_params: Vec<String>,
    /// Enum literals (only for Enum entity_type)
    pub enum_literals: Vec<LogicEnumLiteral>,
    /// Source file location
    pub source_file: Option<String>,
    /// 1-based line number in source; `None` means the source line is unknown
    pub source_line: Option<u32>,
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
    /// Object instance node
    Object,
    /// Abstract interface
    Interface,
    /// Enumeration
    Enum,
    /// Abstract class
    AbstractClass,
    /// Annotation type
    Annotation,
}

/// The type of container/grouping in a class diagram
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum ContainerType {
    /// C++ namespace
    #[default]
    Namespace,
    /// Logical package grouping
    Package,
}

/// Represents a namespace or package container
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LogicContainer {
    /// Fully Qualified Name (FQN)
    pub id: String,
    /// Display name
    pub name: String,
    /// FQN of parent container
    pub parent_id: Option<String>,
    /// Type of container
    pub container_type: ContainerType,
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
    /// Package-private visibility (~)
    Package,
}

/// Represents a class attribute (member variable)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LogicAttribute {
    /// Attribute name
    pub name: String,
    /// Data type (e.g., "int", "string", "std::vector<int>")
    pub data_type: Option<String>,
    /// Visibility modifier
    pub visibility: Visibility,
    /// Default/initial value
    pub default_value: Option<String>,
    /// Whether this is a static member
    pub is_static: bool,
    /// Whether this is a const member
    pub is_const: bool,
    /// Description or documentation
    pub description: Option<String>,
}

/// Represents a method parameter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LogicParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: Option<String>,
    /// Default value if any
    pub default_value: Option<String>,
    /// Whether passed by reference
    pub is_reference: bool,
    /// Whether the parameter is const
    pub is_const: bool,
    /// Whether this is a variadic parameter (...)
    pub is_variadic: bool,
}

/// Represents a class method (member function)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LogicMethod {
    /// Method name
    pub name: String,
    /// Return type
    pub return_type: Option<String>,
    /// Visibility modifier
    pub visibility: Visibility,
    /// Method parameters
    pub parameters: Vec<LogicParameter>,
    /// Template parameters for generic methods
    pub template_params: Vec<String>,
    /// Whether this is a static method
    pub is_static: bool,
    /// Whether this is a const method
    pub is_const: bool,
    /// Whether this is a virtual method
    pub is_virtual: bool,
    /// Whether this is a pure virtual (abstract) method
    pub is_abstract: bool,
    /// Whether this is an override
    pub is_override: bool,
    /// Whether this is a constructor
    pub is_constructor: bool,
    /// Whether this is a destructor
    pub is_destructor: bool,
}

/// Represents a relationship between two entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LogicRelationship {
    /// FQN of the source entity
    pub source: String,
    /// FQN of the target entity
    pub target: String,
    /// Type of relationship
    pub relation_type: RelationType,
    /// Label/annotation on the relationship
    pub label: Option<String>,
    /// Stereotype on the relationship (e.g., <<DependsOn>>)
    pub stereotype: Option<String>,
    /// Source multiplicity (e.g., "1", "0..*", "1..n")
    pub source_multiplicity: Option<String>,
    /// Target multiplicity
    pub target_multiplicity: Option<String>,
    /// Source role name
    pub source_role: Option<String>,
    /// Target role name
    pub target_role: Option<String>,
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
    /// Directed association - `-->`
    Association,
    /// Dependency (uses) - `..>`
    Dependency,
    /// Simple link - `--`
    Link,
    /// Dashed link - `..`
    DashedLink,
}

/// Represents an enum literal/value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LogicEnumLiteral {
    /// Literal name
    pub name: String,
    /// Visibility (if specified)
    pub visibility: Visibility,
    /// Explicit value (e.g., `HIGH = 0`)
    pub value: Option<String>,
    /// Description/documentation
    pub description: Option<String>,
}

/// Error types for class diagram resolution
#[derive(Debug, thiserror::Error)]
pub enum ClassResolverError {
    #[error("Class Resolver: Unresolved reference: {reference}")]
    UnresolvedReference { reference: String },

    #[error("Duplicate entity id: {entity_id}")]
    DuplicateEntity { entity_id: String },

    #[error("Unknown entity type: {entity_type}")]
    UnknownEntityType { entity_type: String },

    #[error("Invalid relationship: {from} -> {to}: {reason}")]
    InvalidRelationship {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Circular inheritance detected: {cycle}")]
    CircularInheritance { cycle: String },

    #[error("Invalid visibility modifier: {modifier}")]
    InvalidVisibility { modifier: String },

    #[error("Parse error: {message}")]
    ParseError { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_serialization() {
        let entity = LogicEntity {
            id: "Core::User".to_string(),
            name: Some("User".to_string()),
            alias: None,
            parent_id: Some("Core".to_string()),
            entity_type: EntityType::Class,
            stereotypes: vec!["Model".to_string()],
            attributes: vec![LogicAttribute {
                name: "name".to_string(),
                data_type: Some("string".to_string()),
                visibility: Visibility::Public,
                default_value: None,
                is_static: false,
                is_const: false,
                description: None,
            }],
            methods: vec![LogicMethod {
                name: "getName".to_string(),
                return_type: Some("string".to_string()),
                visibility: Visibility::Public,
                parameters: vec![],
                template_params: vec![],
                is_static: false,
                is_const: true,
                is_virtual: false,
                is_abstract: false,
                is_override: false,
                is_constructor: false,
                is_destructor: false,
            }],
            template_params: vec![],
            enum_literals: vec![],
            source_file: None,
            source_line: None,
        };

        let json = serde_json::to_string_pretty(&entity).unwrap();
        let deserialized: LogicEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(entity, deserialized);
    }

    #[test]
    fn test_relationship_types() {
        let inheritance = LogicRelationship {
            source: "Derived".to_string(),
            target: "Base".to_string(),
            relation_type: RelationType::Inheritance,
            label: None,
            stereotype: None,
            source_multiplicity: None,
            target_multiplicity: None,
            source_role: None,
            target_role: None,
        };

        assert_eq!(inheritance.relation_type, RelationType::Inheritance);
    }

    #[test]
    fn test_partial_plantuml_entity() {
        // PlantUML often has incomplete information - this should still work
        let entity = LogicEntity {
            id: "UserService".to_string(),
            name: Some("UserService".to_string()),
            entity_type: EntityType::Class,
            stereotypes: vec!["service".to_string()],
            // No attributes specified (common in high-level diagrams)
            attributes: vec![],
            // Method with no return type (PlantUML allows this)
            methods: vec![LogicMethod {
                name: "getUser".to_string(),
                return_type: None, // <-- Often omitted in PlantUML
                visibility: Visibility::Public,
                parameters: vec![],
                template_params: vec![],
                is_static: false,
                is_const: false,
                is_virtual: false,
                is_abstract: false,
                is_override: false,
                is_constructor: false,
                is_destructor: false,
            }],
            ..Default::default()
        };

        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: LogicEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(entity, deserialized);
        assert!(entity.methods[0].return_type.is_none());
    }
}
