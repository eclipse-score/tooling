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

//! Models for class-diagram and internal-API FlatBuffer inputs

use std::collections::{BTreeMap, BTreeSet};

use class_diagram::{
    ClassDiagram as ClassDiagramInput, EntityType, FreeFunctionDecl, MemberVariable, Method,
    Relationship, SimpleEntity, TemplateParameter,
};
use source_location::SourceLocation;

use super::type_names::{method_key, normalized_method, render_type_name};

use crate::{ErrorBuilder, ErrorCategory, ValidationResult};

/// Collection of class diagrams loaded from one or more FlatBuffer files.
pub type ClassDiagramInputs = Vec<ClassDiagramInput>;

#[derive(Clone, Copy)]
enum ClassInputKind {
    Design,
    Implementation,
}

/// Class-like entities from one or more class diagrams, keyed by lower-case id.
pub struct ClassEntityIndex {
    entities: BTreeMap<String, SimpleEntity>,
    free_functions: Vec<FreeFunctionDecl>,
}

/// A declaration without members, e.g. an overview entry. Relationships do not
/// count: an overview may draw them.
fn is_bodyless(entity: &SimpleEntity) -> bool {
    entity.methods.is_empty()
        && entity.variables.is_empty()
        && entity.type_aliases.is_empty()
        && entity.enum_literals.is_empty()
}

/// A disagreement between two declarations of one class, with the locations
/// of the earlier and the later conflicting declaration.
struct MergeConflict {
    field: String,
    first: SourceLocation,
    second: SourceLocation,
}

/// The first entity-level field on which two same-id declarations disagree.
fn conflicting_declaration_field(
    prev: &SimpleEntity,
    entity: &SimpleEntity,
) -> Option<&'static str> {
    if prev.entity_type != entity.entity_type {
        return Some("entity type");
    }
    if prev.stereotypes != entity.stereotypes {
        return Some("stereotype");
    }
    if let (Some(a), Some(b)) = (&prev.template_parameters, &entity.template_parameters) {
        if !is_compatible_template_parameters(a, b) {
            return Some("template parameters");
        }
    }
    None
}

/// Equal lists, or one extended by parameter packs: the resolver appends packs
/// inferred from method parameters to the declared list, so diagrams showing
/// different methods of one template yield different tails.
fn is_compatible_template_parameters(a: &[TemplateParameter], b: &[TemplateParameter]) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    long.starts_with(short)
        && long[short.len()..]
            .iter()
            .all(|parameter| matches!(parameter, TemplateParameter::Type { is_pack: true, .. }))
}

/// Method identity: normalized signature (name, parameter types, const;
/// conversion operators by target).
fn is_same_overload(a: &Method, b: &Method) -> bool {
    method_key(&normalized_method(a)) == method_key(&normalized_method(b))
}

fn comparable_variable(variable: &MemberVariable) -> MemberVariable {
    let mut variable = variable.clone();
    variable.source_location = SourceLocation::default();
    variable.data_type = variable.data_type.as_deref().map(render_type_name);
    variable
}

fn is_same_relationship(a: &Relationship, b: &Relationship) -> bool {
    a.source == b.source
        && a.target == b.target
        && a.relation_type == b.relation_type
        && a.source_multiplicity == b.source_multiplicity
        && a.target_multiplicity == b.target_multiplicity
}

/// Merges the members and relationships of `other` (the later declaration)
/// into `merged`. A member redeclared identically is kept once; a member
/// sharing its identity (name, plus parameter types and const for methods) but
/// differing in any other attribute is a conflict. Enum literal lists must
/// agree when both are present.
fn merge_members(merged: &mut SimpleEntity, other: SimpleEntity) -> Result<(), MergeConflict> {
    match (&mut merged.template_parameters, other.template_parameters) {
        (None, parameters) => merged.template_parameters = parameters,
        (Some(existing), Some(parameters)) if parameters.len() > existing.len() => {
            *existing = parameters;
        }
        _ => {}
    }

    for method in other.methods {
        match merged.methods.iter().find(|m| is_same_overload(m, &method)) {
            None => merged.methods.push(method),
            Some(existing) if normalized_method(existing) == normalized_method(&method) => {}
            Some(existing) => {
                return Err(MergeConflict {
                    field: format!("definition of method \"{}\"", method.name),
                    first: existing.source_location.clone(),
                    second: method.source_location,
                });
            }
        }
    }
    for variable in other.variables {
        match merged.variables.iter().find(|v| v.name == variable.name) {
            None => merged.variables.push(variable),
            Some(existing) if comparable_variable(existing) == comparable_variable(&variable) => {}
            Some(existing) => {
                return Err(MergeConflict {
                    field: format!("definition of attribute \"{}\"", variable.name),
                    first: existing.source_location.clone(),
                    second: variable.source_location,
                });
            }
        }
    }
    for alias in other.type_aliases {
        match merged.type_aliases.iter().find(|t| t.alias == alias.alias) {
            None => merged.type_aliases.push(alias),
            Some(existing)
                if render_type_name(&existing.original_type)
                    == render_type_name(&alias.original_type) => {}
            Some(existing) => {
                return Err(MergeConflict {
                    field: format!("definition of type alias \"{}\"", alias.alias),
                    first: existing.source_location.clone(),
                    second: alias.source_location,
                });
            }
        }
    }
    if merged.enum_literals.is_empty() {
        merged.enum_literals = other.enum_literals;
    } else if let Some(first_literal) = other.enum_literals.first() {
        let same_literals = merged.enum_literals.len() == other.enum_literals.len()
            && merged
                .enum_literals
                .iter()
                .zip(&other.enum_literals)
                .all(|(x, y)| x.name == y.name && x.value == y.value);
        if !same_literals {
            return Err(MergeConflict {
                field: "list of enum literals".to_string(),
                first: merged.enum_literals[0].source_location.clone(),
                second: first_literal.source_location.clone(),
            });
        }
    }
    for relationship in other.relationships {
        if !merged
            .relationships
            .iter()
            .any(|existing| is_same_relationship(existing, &relationship))
        {
            merged.relationships.push(relationship);
        }
    }
    Ok(())
}

impl ClassEntityIndex {
    /// Builds an index from class design diagrams.
    ///
    /// Declarations of one id merge: members, type aliases and relationships
    /// are united. Entity type, stereotype, template parameter, enum literal
    /// or member-definition disagreements, and ids differing in case, are
    /// errors.
    pub fn build_diagram_index(
        diagrams: &[ClassDiagramInput],
        result: &mut ValidationResult,
    ) -> Self {
        Self::build_index(diagrams, result, ClassInputKind::Design)
    }

    /// Builds an index from C++ implementation diagrams.
    pub fn build_implementation_index(
        diagrams: &[ClassDiagramInput],
        result: &mut ValidationResult,
    ) -> Self {
        Self::build_index(diagrams, result, ClassInputKind::Implementation)
    }

    fn build_index(
        diagrams: &[ClassDiagramInput],
        result: &mut ValidationResult,
        input_kind: ClassInputKind,
    ) -> Self {
        let mut entities: BTreeMap<String, SimpleEntity> = BTreeMap::new();
        let mut free_functions = Vec::new();
        let (source_kind, ignore_same_source_location) = match input_kind {
            ClassInputKind::Design => ("design", false),
            ClassInputKind::Implementation => ("implementation", true),
        };

        for diagram in diagrams {
            for entity in &diagram.entities {
                let indexed_entity = entity.clone();

                let key = indexed_entity.id.to_lowercase();
                if let Some(prev) = entities.get(&key) {
                    if ignore_same_source_location
                        && prev.source_location == indexed_entity.source_location
                    {
                        continue;
                    }

                    let (first_source_file, _) = prev.source_location.display();
                    let (second_source_file, _) = indexed_entity.source_location.display();

                    let merges_declarations = matches!(input_kind, ClassInputKind::Design);
                    // The index and the implementation matching ignore case, so ids differing only in case are duplicates.
                    let same_id = prev.id == indexed_entity.id;
                    let mut conflict = if merges_declarations && same_id {
                        conflicting_declaration_field(prev, &indexed_entity).map(|field| {
                            MergeConflict {
                                field: field.to_string(),
                                first: prev.source_location.clone(),
                                second: indexed_entity.source_location.clone(),
                            }
                        })
                    } else {
                        None
                    };

                    let mut merged_entity = None;
                    if conflict.is_none() && merges_declarations && same_id {
                        let (mut merged, other) = if is_bodyless(prev) {
                            (indexed_entity.clone(), prev.clone())
                        } else {
                            (prev.clone(), indexed_entity.clone())
                        };
                        match merge_members(&mut merged, other) {
                            Ok(()) => merged_entity = Some(merged),
                            Err(member_conflict) => conflict = Some(member_conflict),
                        }
                    }

                    if let Some(MergeConflict {
                        field,
                        first,
                        second,
                    }) = conflict
                    {
                        let (first_source_file, _) = first.display();
                        let (second_source_file, _) = second.display();
                        result.add_failure(
                            ErrorBuilder::new(ErrorCategory::Class)
                                .title(format!(
                                    "class \"{}\" is re-declared with a conflicting {field} in another class diagram file.",
                                    prev.id,
                                ))
                                .field("class", format!("\"{}\"", prev.id))
                                .field("conflicting field", field.as_str())
                                .field(
                                    "design source file",
                                    format!("\"{}\"", first_source_file),
                                )
                                .field("design source line", first.line.to_string())
                                .field(
                                    "conflicting source file",
                                    format!("\"{}\"", second_source_file),
                                )
                                .field("conflicting source line", second.line.to_string())
                                .fix(format!(
                                    "make every declaration of class \"{}\" agree on {field}, or rename one of the conflicting classes",
                                    prev.id,
                                ))
                                .build(),
                        );
                    } else if let Some(merged) = merged_entity {
                        entities.insert(key, merged);
                    } else {
                        result.add_failure(
                            ErrorBuilder::new(ErrorCategory::Class)
                                .title(format!(
                                    "class \"{}\" is defined more than once in the class {}.",
                                    prev.id,
                                    source_kind,
                                ))
                                .field("class", format!("\"{}\"", prev.id))
                                .field(
                                    format!("{source_kind} source file"),
                                    format!("\"{}\"", first_source_file),
                                )
                                .field(
                                    format!("{source_kind} source line"),
                                    prev.source_location.line.to_string(),
                                )
                                .field(
                                    "duplicate source file",
                                    format!("\"{}\"", second_source_file),
                                )
                                .field(
                                    "duplicate source line",
                                    indexed_entity.source_location.line.to_string(),
                                )
                                .fix(format!(
                                    "remove or rename one of the duplicate class \"{}\" declarations in the class {}{}",
                                    prev.id,
                                    source_kind,
                                    if merges_declarations && !same_id {
                                        format!("; ids \"{}\" and \"{}\" differ only in case", prev.id, indexed_entity.id)
                                    } else {
                                        String::new()
                                    },
                                ))
                                .build(),
                        );
                    }
                } else {
                    entities.insert(key, indexed_entity);
                }
            }

            free_functions.extend(diagram.free_functions.iter().cloned());
        }

        Self {
            entities,
            free_functions,
        }
    }

    pub fn entities(&self) -> impl Iterator<Item = &SimpleEntity> + '_ {
        self.entities.values()
    }

    pub fn find_by_id(&self, id: &str) -> Option<&SimpleEntity> {
        self.entities.get(&id.to_lowercase())
    }

    pub fn free_functions(&self) -> impl Iterator<Item = &FreeFunctionDecl> + '_ {
        self.free_functions.iter()
    }
}

/// Indexed internal-API data prepared for interface and method validators.
pub struct InternalApiInterface {
    pub id: String,
    pub method_names: BTreeSet<String>,
    pub source_location: SourceLocation,
}

/// Indexed internal-API data prepared for validators.
pub struct InternalApiIndex {
    interfaces: Vec<InternalApiInterface>,
}

impl InternalApiIndex {
    /// Build an [`InternalApiIndex`] from internal-API diagram inputs.
    ///
    /// An interface declared in several files is one interface holding the
    /// union of the methods; its source location is the first declaration
    /// with methods.
    pub fn build_index(diagrams: &[ClassDiagramInput]) -> Self {
        let mut interfaces: Vec<InternalApiInterface> = Vec::new();
        let mut positions: BTreeMap<String, usize> = BTreeMap::new();

        for diagram in diagrams {
            for entity in &diagram.entities {
                if entity.entity_type != EntityType::Interface {
                    continue;
                }

                let method_names: BTreeSet<String> = entity
                    .methods
                    .iter()
                    .map(|method| method.name.clone())
                    .filter(|name| !name.is_empty())
                    .collect();

                if let Some(position) = positions.get(&entity.id) {
                    let interface = &mut interfaces[*position];
                    if interface.method_names.is_empty() && !method_names.is_empty() {
                        interface.source_location = entity.source_location.clone();
                    }
                    interface.method_names.extend(method_names);
                    continue;
                }

                positions.insert(entity.id.clone(), interfaces.len());
                interfaces.push(InternalApiInterface {
                    id: entity.id.clone(),
                    method_names,
                    source_location: entity.source_location.clone(),
                });
            }
        }

        Self { interfaces }
    }

    pub fn interfaces(&self) -> impl Iterator<Item = &InternalApiInterface> + '_ {
        self.interfaces.iter()
    }
}

/// Last dotted segment of an interface id.
fn id_leaf(id: &str) -> &str {
    id.rsplit('.').next().unwrap_or(id)
}

/// Indexed public-API data prepared for component/public-API validators.
pub struct PublicApiIndex {
    api_names: BTreeSet<String>, // interface id leaves
    /// Methods per interface id leaf.
    methods: BTreeMap<String, BTreeSet<String>>,
}

impl PublicApiIndex {
    /// Build a [`PublicApiIndex`] from public-API class diagram inputs.
    pub fn build_index(diagrams: &[ClassDiagramInput]) -> Self {
        let mut api_names: BTreeSet<String> = BTreeSet::new();
        let mut methods: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for diagram in diagrams {
            for entity in &diagram.entities {
                if entity.entity_type != EntityType::Interface {
                    continue;
                }

                let leaf = id_leaf(&entity.id);
                api_names.insert(leaf.to_string());
                methods.entry(leaf.to_string()).or_default().extend(
                    entity
                        .methods
                        .iter()
                        .map(|method| method.name.clone())
                        .filter(|name| !name.is_empty()),
                );
            }
        }

        Self { api_names, methods }
    }

    pub fn api_names(&self) -> impl Iterator<Item = &String> + '_ {
        self.api_names.iter()
    }

    /// `name` is an interface name or id; only its last segment is compared.
    pub fn declares_interface(&self, name: &str) -> bool {
        self.api_names.contains(id_leaf(name))
    }

    /// `interface` is an interface id and `name` an interface name or id; only
    /// their last segments are compared.
    pub fn is_interface_named(&self, interface: &str, name: &str) -> bool {
        id_leaf(interface) == id_leaf(name) && self.declares_interface(interface)
    }

    pub fn declares_method(&self, name: &str) -> bool {
        self.methods.values().any(|methods| methods.contains(name))
    }

    /// `interface` is an interface id; only its last segment is compared.
    pub fn interface_declares_method(&self, interface: &str, method: &str) -> bool {
        self.methods
            .get(id_leaf(interface))
            .is_some_and(|methods| methods.contains(method))
    }

    /// Methods declared on `interface` (an interface id), if it is public API.
    pub fn interface_method_names(&self, interface: &str) -> impl Iterator<Item = &str> + '_ {
        self.methods
            .get(id_leaf(interface))
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    /// All method names declared on any public API interface.
    pub fn method_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.methods.values().flatten().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use class_diagram::{
        ClassDiagram, ClassTemplateKind, Method, MethodModifier, SimpleEntity, Visibility,
    };
    use source_location::SourceLocation;

    fn method(name: &str) -> Method {
        Method {
            name: name.to_string(),
            conversion_target: None,
            return_type: None,
            return_multiplicity: None,
            source_location: SourceLocation::new("test.puml", 1),
            visibility: Visibility::Public,
            parameters: Vec::new(),
            template_parameters: None,
            modifiers: Vec::new(),
        }
    }

    fn entity(id: &str, source_file: &str, source_line: u32) -> SimpleEntity {
        SimpleEntity {
            id: id.to_string(),
            name: id.rsplit('.').next().unwrap_or(id).to_string(),
            enclosing_namespace_id: None,
            stereotypes: Vec::new(),
            entity_type: EntityType::Class,
            template_kind: ClassTemplateKind::NonTemplate,
            type_aliases: Vec::new(),
            variables: Vec::new(),
            methods: Vec::new(),
            template_parameters: None,
            enum_literals: Vec::new(),
            relationships: Vec::new(),
            source_location: SourceLocation::new(source_file, source_line),
        }
    }

    fn with_method(mut entity: SimpleEntity, name: &str) -> SimpleEntity {
        entity.methods.push(method(name));
        entity
    }

    #[test]
    fn class_entity_index_reports_duplicate_source_locations() {
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "classes".to_string(),
            entities: vec![
                with_method(entity("Unit.Sample", "design_a.puml", 12), "Run"),
                with_method(entity("unit.sample", "design_b.puml", 34), "Stop"),
            ],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let _index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains(
            "[Class] Class \"Unit.Sample\" is defined more than once in the class design."
        ));
    }

    #[test]
    fn class_entity_index_merges_declarations_with_bodies_across_diagrams() {
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "classes".to_string(),
            entities: vec![
                with_method(entity("Unit.Sample", "design_a.puml", 12), "Run"),
                with_method(entity("Unit.Sample", "design_b.puml", 34), "Run"),
                with_method(entity("Unit.Sample", "design_b.puml", 35), "Stop"),
            ],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
        let merged = index.find_by_id("Unit.Sample").expect("merged entity");
        let names: Vec<&str> = merged.methods.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["Run", "Stop"]);
    }

    #[test]
    fn class_entity_index_merges_const_overloads() {
        let mut const_overload = with_method(entity("Unit.Sample", "design_b.puml", 34), "data");
        const_overload.methods[0].modifiers = vec![MethodModifier::Const];
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "classes".to_string(),
            entities: vec![
                with_method(entity("Unit.Sample", "design_a.puml", 12), "data"),
                const_overload,
            ],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
        assert_eq!(index.find_by_id("Unit.Sample").unwrap().methods.len(), 2);
    }

    #[test]
    fn class_entity_index_merges_same_id_declared_under_different_names() {
        let mut qualified = with_method(entity("a.B", "design_b.puml", 34), "Stop");
        qualified.name = "E".to_string(); // alias, as in `class B as E`
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "classes".to_string(),
            entities: vec![
                with_method(entity("a.B", "design_a.puml", 12), "Run"),
                qualified,
            ],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
        assert_eq!(index.find_by_id("a.B").unwrap().methods.len(), 2);
    }

    #[test]
    fn class_entity_index_reports_ids_differing_in_case_even_when_bodyless() {
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "classes".to_string(),
            entities: vec![
                with_method(entity("Engine", "design_a.puml", 12), "start"),
                entity("engine", "design_b.puml", 34),
            ],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let _index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("is defined more than once"));
    }

    #[test]
    fn class_entity_index_reports_partial_enum_literal_lists() {
        let literal = |name: &str, value: i128, line: u32| class_diagram::EnumLiteral {
            name: name.to_string(),
            value: Some(value),
            source_location: SourceLocation::new("design.puml", line),
        };
        let mut full = entity("Unit.State", "design_a.puml", 1);
        full.entity_type = EntityType::Enum;
        full.enum_literals = vec![literal("Idle", 0, 2), literal("Running", 1, 3)];
        let mut partial = entity("Unit.State", "design_b.puml", 1);
        partial.entity_type = EntityType::Enum;
        partial.enum_literals = vec![literal("Running", 0, 2)];
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "classes".to_string(),
            entities: vec![full, partial],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let _index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("conflicting list of enum literals"));
    }

    #[test]
    fn class_entity_index_reports_conflicting_member_definitions() {
        let mut conflicting = with_method(entity("Unit.Sample", "design_b.puml", 34), "Run");
        conflicting.methods[0].return_type = Some("int".to_string());
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "classes".to_string(),
            entities: vec![
                with_method(entity("Unit.Sample", "design_a.puml", 12), "Run"),
                conflicting,
            ],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let _index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains(
            "is re-declared with a conflicting definition of method \"Run\" in another class diagram file"
        ));
    }

    #[test]
    fn implementation_class_entity_index_ignores_same_source_location() {
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "implementation".to_string(),
            entities: vec![
                entity("Unit.Sample", "implementation.h", 12),
                entity("unit.sample", "implementation.h", 12),
            ],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let index = ClassEntityIndex::build_implementation_index(&diagrams, &mut result);

        assert!(result.failures.is_empty());
        assert_eq!(index.entities().count(), 1);
    }

    #[test]
    fn class_entity_index_merges_bodyless_declaration_into_detail() {
        for entities in [
            vec![
                entity("Unit.Sample", "overview.puml", 3),
                with_method(entity("Unit.Sample", "detail.puml", 5), "Run"),
            ],
            vec![
                with_method(entity("Unit.Sample", "detail.puml", 5), "Run"),
                entity("Unit.Sample", "overview.puml", 3),
            ],
        ] {
            let diagrams = vec![ClassDiagram {
                source: None,
                name: "classes".to_string(),
                entities,
                free_functions: Vec::new(),
            }];

            let mut result = ValidationResult::default();
            let index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

            assert!(
                result.is_empty(),
                "unexpected failures: {:?}",
                result.failures
            );
            let merged = index.find_by_id("Unit.Sample").expect("merged entity");
            assert_eq!(merged.methods.len(), 1);
        }
    }

    #[test]
    fn class_entity_index_unites_relationships_of_overview_and_detail() {
        let relationship = |target: &str| class_diagram::Relationship {
            source: "Unit.Sample".to_string(),
            target: target.to_string(),
            ..Default::default()
        };
        let mut overview = entity("Unit.Sample", "overview.puml", 3);
        overview.relationships = vec![relationship("Unit.A"), relationship("Unit.B")];
        let mut detail = with_method(entity("Unit.Sample", "detail.puml", 5), "Run");
        detail.relationships = vec![relationship("Unit.B"), relationship("Unit.C")];

        let diagrams = vec![ClassDiagram {
            source: None,
            name: "classes".to_string(),
            entities: vec![overview, detail],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
        let merged = index.find_by_id("Unit.Sample").expect("merged entity");
        assert_eq!(merged.methods.len(), 1);
        let targets: Vec<&str> = merged
            .relationships
            .iter()
            .map(|r| r.target.as_str())
            .collect();
        assert_eq!(targets, ["Unit.B", "Unit.C", "Unit.A"]);
    }

    #[test]
    fn class_entity_index_reports_conflicting_entity_type() {
        let mut interface = entity("Unit.Sample", "detail.puml", 5);
        interface.entity_type = EntityType::Interface;
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "classes".to_string(),
            entities: vec![entity("Unit.Sample", "overview.puml", 3), interface],
            free_functions: Vec::new(),
        }];

        let mut result = ValidationResult::default();
        let _index = ClassEntityIndex::build_diagram_index(&diagrams, &mut result);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains(
            "is re-declared with a conflicting entity type in another class diagram file"
        ));
    }

    fn diagram_of(entities: Vec<SimpleEntity>) -> Vec<ClassDiagram> {
        vec![ClassDiagram {
            source: None,
            name: "classes".to_string(),
            entities,
            free_functions: Vec::new(),
        }]
    }

    fn build(entities: Vec<SimpleEntity>) -> (ClassEntityIndex, ValidationResult) {
        let mut result = ValidationResult::default();
        let index = ClassEntityIndex::build_diagram_index(&diagram_of(entities), &mut result);
        (index, result)
    }

    fn at(mut method: Method, file: &str, line: u32) -> Method {
        method.source_location = SourceLocation::new(file, line);
        method
    }

    fn type_parameter(name: &str, is_pack: bool) -> class_diagram::TemplateParameter {
        class_diagram::TemplateParameter::Type {
            name: name.to_string(),
            is_pack,
        }
    }

    #[test]
    fn class_entity_index_merges_differently_spelled_types() {
        let mut a = with_method(entity("Unit.Sample", "design_a.puml", 1), "data");
        a.methods[0].return_type = Some("std::uint8_t*".to_string());
        a.methods[0].parameters = vec![class_diagram::FunctionArgument {
            param_type: Some("vehicle.Payload".to_string()),
            ..Default::default()
        }];
        let mut b = with_method(entity("Unit.Sample", "design_b.puml", 1), "data");
        b.methods[0].return_type = Some("uint8_t *".to_string());
        b.methods[0].parameters = vec![class_diagram::FunctionArgument {
            param_type: Some("vehicle::Payload".to_string()),
            ..Default::default()
        }];

        let (index, result) = build(vec![a, b]);

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
        assert_eq!(index.find_by_id("Unit.Sample").unwrap().methods.len(), 1);
    }

    #[test]
    fn class_entity_index_reports_conflict_between_later_declarations() {
        let mut run_int = with_method(entity("Unit.Sample", "design_c.puml", 1), "Stop");
        run_int.methods[0].return_type = Some("int".to_string());
        run_int.methods[0] = at(run_int.methods[0].clone(), "design_c.puml", 7);
        let (_, result) = build(vec![
            with_method(entity("Unit.Sample", "design_a.puml", 1), "Run"),
            {
                let mut b = with_method(entity("Unit.Sample", "design_b.puml", 1), "Stop");
                b.methods[0] = at(b.methods[0].clone(), "design_b.puml", 5);
                b
            },
            run_int,
        ]);

        assert_eq!(result.failures.len(), 1);
        let failure = &result.failures[0];
        assert!(failure.contains("design_b.puml"), "{failure}");
        assert!(failure.contains("design_c.puml"), "{failure}");
        assert!(!failure.contains("design_a.puml"), "{failure}");
        assert!(failure.contains("Design source line      : 5"), "{failure}");
        assert!(failure.contains("Conflicting source line : 7"), "{failure}");
    }

    #[test]
    fn class_entity_index_reports_conflicting_attributes_and_type_aliases() {
        let variable = |data_type: &str, file: &str| class_diagram::MemberVariable {
            name: "count".to_string(),
            data_type: Some(data_type.to_string()),
            source_location: SourceLocation::new(file, 2),
            ..Default::default()
        };
        let mut a = entity("Unit.Sample", "design_a.puml", 1);
        a.variables = vec![variable("int", "design_a.puml")];
        let mut b = entity("Unit.Sample", "design_b.puml", 1);
        b.variables = vec![variable("long", "design_b.puml")];
        let (_, result) = build(vec![a, b]);
        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("conflicting definition of attribute \"count\""));

        let alias = |original: &str, file: &str| class_diagram::TypeAlias {
            alias: "Id".to_string(),
            original_type: original.to_string(),
            source_location: SourceLocation::new(file, 2),
        };
        let mut a = entity("Unit.Sample", "design_a.puml", 1);
        a.type_aliases = vec![alias("int", "design_a.puml")];
        let mut b = entity("Unit.Sample", "design_b.puml", 1);
        b.type_aliases = vec![alias("long", "design_b.puml")];
        let (_, result) = build(vec![a, b]);
        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("conflicting definition of type alias \"Id\""));
    }

    #[test]
    fn class_entity_index_merges_identical_enum_literals() {
        let literals = |file: &str| {
            let mut declared = entity("Unit.State", file, 1);
            declared.entity_type = EntityType::Enum;
            declared.enum_literals = ["Idle", "Running"]
                .iter()
                .enumerate()
                .map(|(value, name)| class_diagram::EnumLiteral {
                    name: name.to_string(),
                    value: Some(value as i128),
                    source_location: SourceLocation::new(file, 2),
                })
                .collect();
            declared
        };

        let (index, result) = build(vec![literals("design_a.puml"), literals("design_b.puml")]);

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
        assert_eq!(
            index.find_by_id("Unit.State").unwrap().enum_literals.len(),
            2
        );
    }

    #[test]
    fn class_entity_index_reports_conflicting_template_parameters() {
        let mut a = entity("Unit.Box", "design_a.puml", 1);
        a.template_parameters = Some(vec![type_parameter("T", false)]);
        let mut b = entity("Unit.Box", "design_b.puml", 1);
        b.template_parameters = Some(vec![type_parameter("U", false)]);

        let (_, result) = build(vec![a, b]);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("conflicting template parameters"));
    }

    #[test]
    fn class_entity_index_merges_template_parameters_extended_by_inferred_packs() {
        let mut a = with_method(entity("Unit.Box", "design_a.puml", 1), "Run");
        a.template_parameters = Some(vec![type_parameter("T", false)]);
        let mut b = with_method(entity("Unit.Box", "design_b.puml", 1), "Emplace");
        b.template_parameters = Some(vec![
            type_parameter("T", false),
            type_parameter("Args", true),
        ]);

        let (index, result) = build(vec![a, b]);

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
        let merged = index.find_by_id("Unit.Box").unwrap();
        assert_eq!(merged.template_parameters.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn class_entity_index_keeps_one_copy_of_a_relationship_drawn_twice() {
        let relationship = |file: &str, line: u32| class_diagram::Relationship {
            source: "Unit.Sample".to_string(),
            target: "Unit.Wheel".to_string(),
            source_location: SourceLocation::new(file, line),
            ..Default::default()
        };
        let mut a = with_method(entity("Unit.Sample", "design_a.puml", 1), "Run");
        a.relationships = vec![relationship("design_a.puml", 4)];
        let mut b = with_method(entity("Unit.Sample", "design_b.puml", 1), "Stop");
        b.relationships = vec![relationship("design_b.puml", 9)];

        let (index, result) = build(vec![a, b]);

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
        assert_eq!(
            index.find_by_id("Unit.Sample").unwrap().relationships.len(),
            1
        );
    }

    #[test]
    fn class_entity_index_names_case_only_id_difference() {
        let (_, result) = build(vec![
            with_method(entity("Engine", "design_a.puml", 1), "start"),
            entity("engine", "design_b.puml", 1),
        ]);

        assert!(result.failures[0].contains("differ only in case"));
    }

    #[test]
    fn internal_api_index_merges_interface_declared_in_several_files() {
        let interface = |file: &str, line: u32, methods: &[&str]| {
            let mut declared = entity("InternalAPI.InternalInterface", file, line);
            declared.entity_type = EntityType::Interface;
            for name in methods {
                declared.methods.push(method(name));
            }
            ClassDiagram {
                source: None,
                name: file.to_string(),
                entities: vec![declared],
                free_functions: Vec::new(),
            }
        };
        let diagrams = vec![
            interface("overview.puml", 3, &[]),
            interface("detail_a.puml", 5, &["GetData"]),
            interface("detail_b.puml", 7, &["SetData", "GetData"]),
        ];

        let index = InternalApiIndex::build_index(&diagrams);

        let interfaces: Vec<_> = index.interfaces().collect();
        assert_eq!(interfaces.len(), 1);
        let names: Vec<&str> = interfaces[0]
            .method_names
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(names, ["GetData", "SetData"]);
        assert_eq!(
            interfaces[0].source_location,
            SourceLocation::new("detail_a.puml", 5)
        );
    }

    #[test]
    fn internal_api_index_collects_interfaces_and_methods() {
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "internal_api".to_string(),
            entities: vec![
                SimpleEntity {
                    id: "InternalAPI.InternalInterface".to_string(),
                    name: "InternalInterface".to_string(),
                    enclosing_namespace_id: Some("InternalAPI".to_string()),
                    stereotypes: Vec::new(),
                    entity_type: EntityType::Interface,
                    template_kind: ClassTemplateKind::NonTemplate,
                    type_aliases: Vec::new(),
                    variables: Vec::new(),
                    methods: vec![method("GetData")],
                    template_parameters: None,
                    enum_literals: Vec::new(),
                    relationships: Vec::new(),
                    source_location: SourceLocation::new("test.puml", 1),
                },
                SimpleEntity {
                    id: "InternalAPI.Helper".to_string(),
                    name: "Helper".to_string(),
                    enclosing_namespace_id: Some("InternalAPI".to_string()),
                    stereotypes: Vec::new(),
                    entity_type: EntityType::Class,
                    template_kind: ClassTemplateKind::NonTemplate,
                    type_aliases: Vec::new(),
                    variables: Vec::new(),
                    methods: vec![method("IgnoreMe")],
                    template_parameters: None,
                    enum_literals: Vec::new(),
                    relationships: Vec::new(),
                    source_location: SourceLocation::new("test.puml", 1),
                },
            ],
            free_functions: Vec::new(),
        }];

        let index = InternalApiIndex::build_index(&diagrams);

        assert!(index
            .interfaces()
            .find(|interface| interface.id == "InternalAPI.InternalInterface")
            .expect("expected interface entry")
            .method_names
            .contains("GetData"));
        assert!(index
            .interfaces()
            .all(|interface| interface.id != "InternalAPI.Helper"));
    }

    #[test]
    fn internal_api_index_keeps_distinct_interface_ids() {
        let diagrams = vec![ClassDiagram {
            source: None,
            name: "internal_api".to_string(),
            entities: vec![
                SimpleEntity {
                    id: "InternalAPI.InternalInterfaceA".to_string(),
                    name: "InternalInterface".to_string(),
                    enclosing_namespace_id: Some("InternalAPI".to_string()),
                    stereotypes: Vec::new(),
                    entity_type: EntityType::Interface,
                    template_kind: ClassTemplateKind::NonTemplate,
                    type_aliases: Vec::new(),
                    variables: Vec::new(),
                    methods: vec![method("GetData")],
                    template_parameters: None,
                    enum_literals: Vec::new(),
                    relationships: Vec::new(),
                    source_location: SourceLocation::new("test.puml", 1),
                },
                SimpleEntity {
                    id: "InternalAPI.InternalInterfaceB".to_string(),
                    name: "InternalInterface".to_string(),
                    enclosing_namespace_id: Some("InternalAPI".to_string()),
                    stereotypes: Vec::new(),
                    entity_type: EntityType::Interface,
                    template_kind: ClassTemplateKind::NonTemplate,
                    type_aliases: Vec::new(),
                    variables: Vec::new(),
                    methods: vec![method("GetData1")],
                    template_parameters: None,
                    enum_literals: Vec::new(),
                    relationships: Vec::new(),
                    source_location: SourceLocation::new("test.puml", 1),
                },
            ],
            free_functions: Vec::new(),
        }];

        let index = InternalApiIndex::build_index(&diagrams);

        let interface_ids: BTreeSet<&str> = index
            .interfaces()
            .map(|interface| interface.id.as_str())
            .collect();

        assert_eq!(interface_ids.len(), 2);
        assert!(interface_ids.contains("InternalAPI.InternalInterfaceA"));
        assert!(interface_ids.contains("InternalAPI.InternalInterfaceB"));
    }
}
