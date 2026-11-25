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
pub use source_location::SourceLocation;

mod type_expr;
pub use type_expr::{QualifiedTypeName, TypeArgument, TypeDescription, TypeExpr};

/// Represents a complete class diagram model containing all resolved entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ClassDiagram {
    pub name: String,
    /// Source path of the diagram file (`--source-name`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default)]
    pub entities: Vec<SimpleEntity>,
    #[serde(default)]
    pub free_functions: Vec<FreeFunctionDecl>,
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
    /// Alias if present, else the declared name; the spelling is kept (`.`/`::`)
    /// without a leading root marker, so rooted and relative declarations agree.
    pub name: String,

    /// FQN of parent namespace/package
    /// enclosing namespace name
    pub enclosing_namespace_id: Option<String>,
    /// PlantUML stereotypes declared on the entity.
    #[serde(default)]
    pub stereotypes: Vec<String>,
    /// Type of entity (class, struct, interface, enum, etc.)
    pub entity_type: EntityType,

    /// The C++ class-template declaration kind, independent of its parameters.
    #[serde(default, skip_serializing_if = "ClassTemplateKind::is_non_template")]
    pub template_kind: ClassTemplateKind,

    // aliased type with using keyword also called annotation in plantuml
    pub type_aliases: Vec<TypeAlias>,
    pub variables: Vec<MemberVariable>,
    /// Methods (member functions)
    pub methods: Vec<Method>,
    /// Declared template parameters, when available. An empty vector represents
    /// `template<>`; None can also occur for an explicit specialization parsed by
    /// libclang. Use template_kind rather than parameter presence to classify C++ templates.
    pub template_parameters: Option<Vec<TemplateParameter>>,

    /// Enum literals (only for Enum entity_type)
    pub enum_literals: Vec<EnumLiteral>,

    // all relationships for current entity
    pub relationships: Vec<Relationship>,

    /// Debug info for display in case of mismatch
    pub source_location: SourceLocation,
}

/// Distinguishes ordinary classes from class templates and their specializations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum ClassTemplateKind {
    #[default]
    NonTemplate,
    Primary,
    PartialSpecialization,
    ExplicitSpecialization,
}

impl ClassTemplateKind {
    pub fn is_non_template(&self) -> bool {
        matches!(self, Self::NonTemplate)
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateParameter {
    Type {
        name: String,
        is_pack: bool,
    },
    NonType {
        name: String,
        value_type: String,
        is_pack: bool,
    },
    Template {
        name: String,
        parameters: Vec<TemplateParameter>,
        is_pack: bool,
    },
}

impl TemplateParameter {
    pub fn name(&self) -> &str {
        match self {
            Self::Type { name, .. } => name,
            Self::NonType { name, .. } => name,
            Self::Template { name, .. } => name,
        }
    }

    pub fn is_pack(&self) -> bool {
        match self {
            Self::Type { is_pack, .. }
            | Self::NonType { is_pack, .. }
            | Self::Template { is_pack, .. } => *is_pack,
        }
    }
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
    pub original_type: TypeDescription,
    /// Source location in input
    pub source_location: SourceLocation,
}

/// Represents a class attribute (member variable)
/// renamed from LogicAttribute
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MemberVariable {
    /// Attribute name
    pub name: String,
    /// Data type (e.g., "int", "string", "std::vector<int>")
    pub data_type: Option<TypeDescription>,
    /// Visibility modifier
    pub visibility: Visibility,
    /// Whether this is a static member
    pub is_static: bool,
    /// Source location in input
    pub source_location: SourceLocation,
}

/// Explicit UML parameter direction, independent of the C++ parameter type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParameterDirection {
    In,
    Out,
    InOut,
}

impl ParameterDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
            Self::InOut => "inout",
        }
    }
}

/// Represents a method parameter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FunctionArgument {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: Option<TypeDescription>,
    /// Explicit UML direction; absent for legacy parameters and C++ declarations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<ParameterDirection>,
    /// UML multiplicity bracket interior; independent of type and C++ identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiplicity: Option<String>,
    pub is_variadic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_pack_expansion: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MethodModifier {
    Static,
    Virtual,
    Abstract,
    Override,
    Constructor,
    Destructor,
    Noexcept,
    Final,
    Const,
}

impl MethodModifier {
    pub fn from_conditions(
        conditions: impl IntoIterator<Item = (bool, MethodModifier)>,
    ) -> Vec<MethodModifier> {
        conditions
            .into_iter()
            .filter_map(|(enabled, modifier)| enabled.then_some(modifier))
            .collect()
    }
}

/// Represents a class method (member function)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Method {
    /// Method name
    pub name: String,
    /// Explicit conversion-operator target; absent for ordinary methods.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion_target: Option<String>,
    /// Return type
    pub return_type: Option<TypeDescription>,
    /// UML return multiplicity bracket interior; independent of the return type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_multiplicity: Option<String>,
    /// Visibility modifier
    pub visibility: Visibility,
    /// Method parameters
    pub parameters: Vec<FunctionArgument>,
    /// Template parameters for generic methods
    pub template_parameters: Option<Vec<TemplateParameter>>,
    pub modifiers: Vec<MethodModifier>,
    /// Source location in input
    pub source_location: SourceLocation,
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
    /// Source location in input
    pub source_location: SourceLocation,
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
    pub value: Option<i128>,
    /// Source location in input
    pub source_location: SourceLocation,
}

/// Represents a global- or namespace-scope function declaration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FreeFunctionDecl {
    /// Function name without its namespace qualification.
    pub name: String,
    /// Namespace containing the function, if any.
    pub enclosing_namespace_id: Option<String>,
    /// Return type.
    pub return_type: Option<TypeDescription>,
    /// UML return multiplicity bracket interior; independent of the return type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_multiplicity: Option<String>,
    /// Function parameters.
    pub parameters: Vec<FunctionArgument>,
    /// Template parameters for generic functions.
    pub template_parameters: Option<Vec<TemplateParameter>>,
    /// Source location in input.
    pub source_location: SourceLocation,
}

impl FreeFunctionDecl {
    /// Returns the function name qualified by its containing namespace.
    pub fn qualified_name(&self) -> String {
        match self.enclosing_namespace_id.as_deref() {
            Some(namespace) if !namespace.is_empty() => format!("{namespace}::{}", self.name),
            _ => self.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_parameters_use_tagged_json_for_all_kinds_and_packs() {
        for is_pack in [false, true] {
            let nested = TemplateParameter::Type {
                name: "T".to_string(),
                is_pack: true,
            };
            let cases = [
                (
                    TemplateParameter::Type {
                        name: "T".to_string(),
                        is_pack,
                    },
                    "T",
                    serde_json::json!({"Type": {"name": "T", "is_pack": is_pack}}),
                ),
                (
                    TemplateParameter::NonType {
                        name: "N".to_string(),
                        value_type: "int".to_string(),
                        is_pack,
                    },
                    "N",
                    serde_json::json!({"NonType": {
                        "name": "N", "value_type": "int", "is_pack": is_pack
                    }}),
                ),
                (
                    TemplateParameter::Template {
                        name: "Container".to_string(),
                        parameters: vec![nested],
                        is_pack,
                    },
                    "Container",
                    serde_json::json!({"Template": {
                        "name": "Container",
                        "parameters": [{"Type": {"name": "T", "is_pack": true}}],
                        "is_pack": is_pack
                    }}),
                ),
            ];

            for (parameter, name, expected) in cases {
                assert_eq!(parameter.name(), name);
                assert_eq!(parameter.is_pack(), is_pack);
                assert_eq!(serde_json::to_value(&parameter).unwrap(), expected);
                assert_eq!(
                    serde_json::from_value::<TemplateParameter>(expected).unwrap(),
                    parameter
                );
            }
        }
    }

    #[test]
    fn test_entity_serialization() {
        let entity = SimpleEntity {
            id: "Core::User".to_string(),
            name: "User".to_string(),
            enclosing_namespace_id: Some("Core".to_string()),
            stereotypes: Vec::new(),
            entity_type: EntityType::Class,
            template_kind: ClassTemplateKind::NonTemplate,
            variables: vec![MemberVariable {
                name: "name".to_string(),
                data_type: Some(TypeDescription {
                    raw_text: "string".to_string(),
                    expression: Some(TypeExpr::NamedType(QualifiedTypeName {
                        segments: vec!["string".to_string()],
                        is_global: false,
                    })),
                }),
                visibility: Visibility::Public,
                is_static: false,
                source_location: SourceLocation::new("test.puml", 1),
            }],
            methods: vec![Method {
                name: "getName".to_string(),
                conversion_target: None,
                return_type: Some(TypeDescription::from("string")),
                return_multiplicity: None,
                visibility: Visibility::Public,
                parameters: vec![],
                template_parameters: None,
                modifiers: vec![MethodModifier::Virtual],
                source_location: SourceLocation::new("test.puml", 1),
            }],
            template_parameters: None,
            enum_literals: vec![],
            source_location: SourceLocation::new("test.puml", 1),
            type_aliases: vec![],
            relationships: vec![],
        };

        let json = serde_json::to_string_pretty(&entity).unwrap();
        let deserialized: SimpleEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(entity, deserialized);
        let mut legacy_json = serde_json::to_value(&entity).unwrap();
        assert!(legacy_json["methods"][0]["return_type"]
            .get("expression")
            .is_none());
        legacy_json["variables"][0]["data_type"] = serde_json::json!("string");
        assert!(serde_json::from_value::<SimpleEntity>(legacy_json).is_err());
    }

    #[test]
    fn entity_template_kind_defaults_to_non_template_and_is_omitted() {
        let entity = SimpleEntity::default();
        let json = serde_json::to_value(&entity).unwrap();

        assert!(json.get("template_kind").is_none());
        assert_eq!(
            serde_json::from_value::<SimpleEntity>(json)
                .unwrap()
                .template_kind,
            ClassTemplateKind::NonTemplate
        );
    }

    #[test]
    fn test_relationship_types() {
        let inheritance = Relationship {
            source: "Derived".to_string(),
            target: "Base".to_string(),
            relation_type: RelationType::Inheritance,
            source_multiplicity: None,
            target_multiplicity: None,
            source_location: SourceLocation::new("test.puml", 1),
        };

        assert_eq!(inheritance.relation_type, RelationType::Inheritance);
    }

    #[test]
    fn free_function_qualified_name_includes_namespace_when_present() {
        let global = FreeFunctionDecl {
            name: "log".to_string(),
            ..Default::default()
        };
        let namespaced = FreeFunctionDecl {
            name: "log".to_string(),
            enclosing_namespace_id: Some("app::internal".to_string()),
            ..Default::default()
        };

        assert_eq!(global.qualified_name(), "log");
        assert_eq!(namespaced.qualified_name(), "app::internal::log");
    }

    #[test]
    fn test_partial_plantuml_entity() {
        // PlantUML often has incomplete information - this should still work
        let entity = SimpleEntity {
            id: "UserService".to_string(),
            name: "UserService".to_string(),
            entity_type: EntityType::Class,
            methods: vec![Method {
                name: "getUser".to_string(),
                conversion_target: None,
                return_type: None,
                return_multiplicity: None,
                visibility: Visibility::Public,
                parameters: vec![],
                template_parameters: None,
                modifiers: vec![],
                source_location: SourceLocation::new("test.puml", 1),
            }],
            ..Default::default()
        };

        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: SimpleEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(entity, deserialized);
        assert!(entity.methods[0].return_type.is_none());
    }
}
