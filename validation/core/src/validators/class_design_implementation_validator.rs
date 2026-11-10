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

//! Class implementation validation: compare unit design class diagrams with
//! C++ implementation produced by the C++ parser.

use super::shared::{
    best_string_suggestion, candidate_graph, nonempty_candidate_rows, unique_covering_matching,
    MatchingOutcome,
};
use crate::models::ClassEntityIndex;
use crate::results::{ErrorBuilder, ErrorCategory};
use crate::ValidationResult;
use class_diagram::{
    ClassTemplateKind, EnumLiteral, FreeFunctionDecl, MemberVariable, Method, MethodModifier,
    Relationship, SimpleEntity, TemplateParameter, TypeAlias,
};
use source_location::SourceLocation;
use std::collections::BTreeMap;

/// Run design-class-vs-implementation-class validation using indexed inputs.
pub fn validate_class_design_implementation(
    design_classes: &ClassEntityIndex,
    implementation_classes: &ClassEntityIndex,
) -> ValidationResult {
    ClassDesignImplementationValidator::new().run(design_classes, implementation_classes)
}

struct ClassDesignImplementationValidator {
    result: ValidationResult,
}

#[derive(Clone, Copy)]
enum FieldReferenceStyle {
    Quoted,
    Verbatim,
}

enum ParameterDifference {
    Count,
    Name,
    Type,
    Variadic,
    PackExpansion,
}

impl ClassDesignImplementationValidator {
    fn new() -> Self {
        Self {
            result: ValidationResult::default(),
        }
    }

    fn run(
        mut self,
        design_classes: &ClassEntityIndex,
        implementation_classes: &ClassEntityIndex,
    ) -> ValidationResult {
        self.check_design_classes_have_implementation(design_classes, implementation_classes);
        self.check_design_free_functions_with_implementation(
            design_classes,
            implementation_classes,
        );
        self.result
    }

    fn check_design_free_functions_with_implementation(
        &mut self,
        design_classes: &ClassEntityIndex,
        implementation_classes: &ClassEntityIndex,
    ) {
        let design_groups = free_function_groups(design_classes);
        let implementation_groups = free_function_groups(implementation_classes);
        let suggestions = free_function_map(implementation_classes);
        for (name, designs) in design_groups {
            let implementations = implementation_groups
                .get(&name)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let candidates =
                candidate_graph(&designs, implementations, |design, implementation| {
                    parameter_types_match(&design.parameters, &implementation.parameters)
                });
            // Preserve legacy diagnostics for empty rows independently of matching
            // the remaining rows. Fields never participate in candidate selection.
            for (row, design) in designs.iter().enumerate() {
                if !candidates[row].is_empty() {
                    continue;
                }
                if implementations.len() == 1 {
                    self.append_free_function_comparison_diagnostics(design, implementations[0]);
                    self.check_free_function_fields(design, implementations[0]);
                } else {
                    self.result.add_failure(Self::format_missing_free_function(
                        design,
                        best_string_suggestion(
                            &free_function_key(design),
                            suggestions.keys().map(String::as_str),
                        )
                        .as_deref(),
                    ));
                }
            }
            let (designs, candidates) = nonempty_candidate_rows(&designs, &candidates);
            match unique_covering_matching(&candidates, implementations.len()) {
                MatchingOutcome::Unique(matching) => {
                    for (row, column) in matching.into_iter().enumerate() {
                        let design = designs[row];
                        let implementation = implementations[column];
                        self.append_free_function_comparison_diagnostics(design, implementation);
                        if !free_functions_match(design, implementation) {
                            self.check_free_function_fields(design, implementation);
                        }
                    }
                }
                outcome => self.result.add_failure(format_overload_matching_failure(
                    &format!("free function {name}"),
                    &outcome,
                    designs.iter().map(|function| {
                        (
                            format_function_declaration(
                                &normalize_qualified_name(&function.qualified_name()),
                                &function.parameters,
                            ),
                            &function.source_location,
                        )
                    }),
                    implementations.iter().map(|function| {
                        (
                            format_function_declaration(
                                &normalize_qualified_name(&function.qualified_name()),
                                &function.parameters,
                            ),
                            &function.source_location,
                        )
                    }),
                )),
            }
        }
    }

    fn check_free_function_fields(
        &mut self,
        design_function: &FreeFunctionDecl,
        implementation_function: &FreeFunctionDecl,
    ) {
        if !optional_types_match(
            &design_function.return_type,
            &implementation_function.return_type,
        ) {
            self.result.add_failure(Self::format_free_function_mismatch(
                design_function,
                implementation_function,
                "return type",
                &render_optional_type(&design_function.return_type),
                &render_optional_type(&implementation_function.return_type),
            ));
        }

        if !template_parameters_match(
            &design_function.template_parameters,
            &implementation_function.template_parameters,
        ) {
            self.result.add_failure(Self::format_free_function_mismatch(
                design_function,
                implementation_function,
                "template parameters",
                &render_template_parameters(&design_function.template_parameters),
                &render_template_parameters(&implementation_function.template_parameters),
            ));
        }

        self.check_free_function_parameter_fields(design_function, implementation_function);
    }

    fn check_free_function_parameter_fields(
        &mut self,
        design_function: &FreeFunctionDecl,
        implementation_function: &FreeFunctionDecl,
    ) {
        for (difference, design_value, implementation_value) in parameter_differences(
            &design_function.parameters,
            &implementation_function.parameters,
        ) {
            self.result.add_failure(Self::format_free_function_mismatch(
                design_function,
                implementation_function,
                &format!("parameter {}", parameter_difference_name(difference)),
                &design_value,
                &implementation_value,
            ));
        }
    }

    fn check_design_classes_have_implementation(
        &mut self,
        design_classes: &ClassEntityIndex,
        implementation_classes: &ClassEntityIndex,
    ) {
        for design_entity in design_classes.entities() {
            let normalized_design_id = normalize_qualified_name(&design_entity.id);
            let mut implementation_entities = implementation_classes
                .entities()
                .filter(|entity| {
                    if is_specialization(entity.template_kind) {
                        specialization_matches_design(&entity.id, &normalized_design_id)
                    } else {
                        normalize_qualified_name(&entity.id)
                            .eq_ignore_ascii_case(&normalized_design_id)
                    }
                })
                .collect::<Vec<_>>();

            if implementation_entities
                .iter()
                .any(|entity| is_specialization(entity.template_kind))
            {
                // A defined primary can be a rejection-only fallback rather than the
                // interface described by the design. Keep it only when its parameters
                // match, but continue to check every matching specialization.
                implementation_entities.retain(|entity| {
                    entity.template_kind != ClassTemplateKind::Primary
                        || template_parameters_match(
                            &design_entity.template_parameters,
                            &entity.template_parameters,
                        )
                });
            }

            if implementation_entities.is_empty() {
                let suggested_class = best_string_suggestion(
                    &design_entity.id,
                    implementation_classes
                        .entities()
                        .map(|entity| entity.id.as_str()),
                );
                self.result.add_failure(Self::format_missing_class(
                    design_entity,
                    suggested_class.as_deref(),
                ));
                continue;
            }

            for implementation_entity in implementation_entities {
                self.append_class_comparison_diagnostics(design_entity, implementation_entity);
                self.check_entity(design_entity, implementation_entity);
            }
        }
    }

    fn append_class_comparison_diagnostics(
        &mut self,
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
    ) {
        self.result
            .diagnostics
            .debug(|| format_class_comparison_debug(design_entity, implementation_entity));
        self.result
            .diagnostics
            .trace(|| format_entity_trace("Design entity", design_entity));
        self.result
            .diagnostics
            .trace(|| format_entity_trace("Implementation entity", implementation_entity));
    }

    fn append_free_function_comparison_diagnostics(
        &mut self,
        design_function: &FreeFunctionDecl,
        implementation_function: &FreeFunctionDecl,
    ) {
        self.result.diagnostics.debug(|| {
            format_free_function_comparison_debug(design_function, implementation_function)
        });
        self.result
            .diagnostics
            .trace(|| format_free_function_trace("Design free function", design_function));
        self.result.diagnostics.trace(|| {
            format_free_function_trace("Implementation free function", implementation_function)
        });
    }

    fn check_entity(&mut self, design_entity: &SimpleEntity, implementation_entity: &SimpleEntity) {
        if design_entity.entity_type != implementation_entity.entity_type {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                "entity_type",
                FieldReferenceStyle::Quoted,
                &format!("{:?}", design_entity.entity_type),
                &format!("{:?}", implementation_entity.entity_type),
            ));
        }

        if is_specialization(implementation_entity.template_kind) {
            self.result.diagnostics.debug(|| {
                format!(
                    "Skipping class template-parameter check for design entity {:?} and implementation specialization {:?}: matched by fully qualified base name only; specialization arguments are not validated",
                    design_entity.id, implementation_entity.id,
                )
            });
        } else if !template_parameters_match(
            &design_entity.template_parameters,
            &implementation_entity.template_parameters,
        ) {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                "template_parameters",
                FieldReferenceStyle::Quoted,
                &render_template_parameters(&design_entity.template_parameters),
                &render_template_parameters(&implementation_entity.template_parameters),
            ));
        }

        self.check_type_aliases(design_entity, implementation_entity);
        self.check_variables(design_entity, implementation_entity);
        self.check_methods(design_entity, implementation_entity);
        self.check_enum_literals(design_entity, implementation_entity);
        self.check_relationships(design_entity, implementation_entity);
    }

    fn check_type_aliases(
        &mut self,
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
    ) {
        let implementation_aliases = type_alias_map(implementation_entity);
        for design_alias in &design_entity.type_aliases {
            match implementation_aliases.get(design_alias.alias.as_str()) {
                Some(implementation_alias)
                    if type_aliases_match(design_alias, implementation_alias) => {}
                Some(implementation_alias) => self.result.add_failure(Self::format_mismatch(
                    design_entity,
                    implementation_entity,
                    &format!(
                        "type alias \"{}\" original type",
                        design_alias.alias.as_str()
                    ),
                    FieldReferenceStyle::Verbatim,
                    &design_alias.original_type,
                    &implementation_alias.original_type,
                )),
                None => self.result.add_failure(Self::format_missing_member(
                    design_entity,
                    implementation_entity,
                    "type_alias",
                    &design_alias.alias,
                    best_string_suggestion(
                        &design_alias.alias,
                        implementation_aliases.keys().copied(),
                    )
                    .as_deref(),
                )),
            }
        }
    }

    fn check_variables(
        &mut self,
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
    ) {
        let implementation_variables = variable_map(implementation_entity);
        for design_variable in &design_entity.variables {
            match implementation_variables.get(design_variable.name.as_str()) {
                Some(implementation_variable)
                    if member_variables_match(design_variable, implementation_variable) => {}
                Some(implementation_variable) => self.check_variable_fields(
                    design_entity,
                    implementation_entity,
                    design_variable,
                    implementation_variable,
                ),
                None => self.result.add_failure(Self::format_missing_member(
                    design_entity,
                    implementation_entity,
                    "variable",
                    &design_variable.name,
                    best_string_suggestion(
                        &design_variable.name,
                        implementation_variables.keys().copied(),
                    )
                    .as_deref(),
                )),
            }
        }
    }

    fn check_variable_fields(
        &mut self,
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
        design_variable: &MemberVariable,
        implementation_variable: &MemberVariable,
    ) {
        let variable_name = &design_variable.name;

        if !optional_types_match(
            &design_variable.data_type,
            &implementation_variable.data_type,
        ) {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("variable \"{variable_name}\" data type"),
                FieldReferenceStyle::Verbatim,
                &render_optional_type(&design_variable.data_type),
                &render_optional_type(&implementation_variable.data_type),
            ));
        }

        if design_variable.visibility != implementation_variable.visibility {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("variable \"{variable_name}\" visibility"),
                FieldReferenceStyle::Verbatim,
                &format!("{:?}", design_variable.visibility),
                &format!("{:?}", implementation_variable.visibility),
            ));
        }

        if design_variable.is_static != implementation_variable.is_static {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("variable \"{variable_name}\" is static"),
                FieldReferenceStyle::Verbatim,
                &format!("{:?}", design_variable.is_static),
                &format!("{:?}", implementation_variable.is_static),
            ));
        }
    }

    fn check_methods(
        &mut self,
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
    ) {
        let design_groups = method_groups(design_entity);
        let implementation_groups = method_groups(implementation_entity);
        let suggestions = method_map(implementation_entity);
        for (name, designs) in design_groups {
            let implementations = implementation_groups
                .get(name)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let candidates =
                candidate_graph(&designs, implementations, |design, implementation| {
                    parameter_types_match(&design.parameters, &implementation.parameters)
                        && design.modifiers.contains(&MethodModifier::Const)
                            == implementation.modifiers.contains(&MethodModifier::Const)
                });
            // An unmatched declaration must not hide independent mismatches or
            // contention among declarations that do have candidates.
            for (row, design) in designs.iter().enumerate() {
                if !candidates[row].is_empty() {
                    continue;
                }
                if implementations.len() == 1 {
                    self.check_method_fields(
                        design_entity,
                        implementation_entity,
                        design,
                        implementations[0],
                    );
                } else {
                    self.result.add_failure(Self::format_missing_member(
                        design_entity,
                        implementation_entity,
                        "method",
                        &method_key(design),
                        best_string_suggestion(
                            &method_key(design),
                            suggestions.keys().map(String::as_str),
                        )
                        .as_deref(),
                    ));
                }
            }
            let (designs, candidates) = nonempty_candidate_rows(&designs, &candidates);
            match unique_covering_matching(&candidates, implementations.len()) {
                MatchingOutcome::Unique(matching) => {
                    for (row, column) in matching.into_iter().enumerate() {
                        if !methods_match(designs[row], implementations[column]) {
                            self.check_method_fields(
                                design_entity,
                                implementation_entity,
                                designs[row],
                                implementations[column],
                            );
                        }
                    }
                }
                outcome => self.result.add_failure(format_overload_matching_failure(
                    &format!("method {}::{name}", design_entity.id),
                    &outcome,
                    designs.iter().map(|method| {
                        (
                            format_function_declaration(
                                &format!("{}::{name}", design_entity.id),
                                &method.parameters,
                            ),
                            &method.source_location,
                        )
                    }),
                    implementations.iter().map(|method| {
                        (
                            format_function_declaration(
                                &format!("{}::{name}", implementation_entity.id),
                                &method.parameters,
                            ),
                            &method.source_location,
                        )
                    }),
                )),
            }
        }
    }

    fn check_method_fields(
        &mut self,
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
        design_method: &Method,
        implementation_method: &Method,
    ) {
        let method_name = &design_method.name;

        if !optional_types_match(
            &design_method.return_type,
            &implementation_method.return_type,
        ) {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("method \"{method_name}\" return type"),
                FieldReferenceStyle::Verbatim,
                &render_optional_type(&design_method.return_type),
                &render_optional_type(&implementation_method.return_type),
            ));
        }

        if design_method.visibility != implementation_method.visibility {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("method \"{method_name}\" visibility"),
                FieldReferenceStyle::Verbatim,
                &format!("{:?}", design_method.visibility),
                &format!("{:?}", implementation_method.visibility),
            ));
        }

        self.check_method_parameter_fields(
            design_entity,
            implementation_entity,
            design_method,
            implementation_method,
        );

        if !template_parameters_match(
            &design_method.template_parameters,
            &implementation_method.template_parameters,
        ) {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("method \"{method_name}\" template parameters"),
                FieldReferenceStyle::Verbatim,
                &render_template_parameters(&design_method.template_parameters),
                &render_template_parameters(&implementation_method.template_parameters),
            ));
        }

        if design_method.modifiers != implementation_method.modifiers {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("method \"{method_name}\" modifiers"),
                FieldReferenceStyle::Verbatim,
                &format!("{:?}", design_method.modifiers),
                &format!("{:?}", implementation_method.modifiers),
            ));
        }
    }

    fn check_method_parameter_fields(
        &mut self,
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
        design_method: &Method,
        implementation_method: &Method,
    ) {
        let method_name = &design_method.name;
        for (difference, design_value, implementation_value) in
            parameter_differences(&design_method.parameters, &implementation_method.parameters)
        {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!(
                    "method \"{method_name}\" parameter {}",
                    parameter_difference_name(difference)
                ),
                FieldReferenceStyle::Verbatim,
                &design_value,
                &implementation_value,
            ));
        }
    }

    fn check_enum_literals(
        &mut self,
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
    ) {
        let implementation_literals = enum_literal_map(implementation_entity);
        for design_literal in &design_entity.enum_literals {
            match implementation_literals.get(design_literal.name.as_str()) {
                Some(implementation_literal)
                    if enum_literals_match(design_literal, implementation_literal) => {}
                Some(implementation_literal) => self.result.add_failure(Self::format_mismatch(
                    design_entity,
                    implementation_entity,
                    &format!("{:?}", design_literal.name.as_str()),
                    FieldReferenceStyle::Verbatim,
                    &enum_literal_value(design_literal),
                    &enum_literal_value(implementation_literal),
                )),
                None => self.result.add_failure(Self::format_missing_member(
                    design_entity,
                    implementation_entity,
                    "enum_literal",
                    &design_literal.name,
                    best_string_suggestion(
                        &design_literal.name,
                        implementation_literals.keys().copied(),
                    )
                    .as_deref(),
                )),
            }
        }
    }

    fn check_relationships(
        &mut self,
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
    ) {
        let implementation_relationships = relationship_map(implementation_entity);
        for design_relationship in &design_entity.relationships {
            let key = relationship_key(design_relationship);
            let display_name = relationship_display_name(design_relationship);
            match implementation_relationships.get(key.as_str()) {
                Some(candidates)
                    if candidates.iter().any(|candidate| {
                        candidate.relation_type == design_relationship.relation_type
                    }) => {}
                Some(candidates) => {
                    let mut candidate_names = candidates
                        .iter()
                        .map(|candidate| relationship_display_name(candidate))
                        .collect::<Vec<_>>();
                    candidate_names.sort();
                    candidate_names.dedup();
                    self.result.add_failure(Self::format_mismatch(
                        design_entity,
                        implementation_entity,
                        "relationship type",
                        FieldReferenceStyle::Quoted,
                        &display_name,
                        &candidate_names.join("; "),
                    ))
                }
                None => {
                    let implementation_relationship_names = implementation_entity
                        .relationships
                        .iter()
                        .map(relationship_display_name)
                        .collect::<Vec<_>>();
                    self.result.add_failure(Self::format_missing_member(
                        design_entity,
                        implementation_entity,
                        "relationship",
                        &display_name,
                        best_string_suggestion(
                            &display_name,
                            implementation_relationship_names.iter().map(String::as_str),
                        )
                        .as_deref(),
                    ))
                }
            }
        }
    }

    fn format_missing_class(entity: &SimpleEntity, suggested_class: Option<&str>) -> String {
        let error = ErrorBuilder::new(ErrorCategory::Class)
            .title(format!(
                "class \"{}\" from the unit design not found in the C++ class implementation",
                entity.id
            ))
            .field("class", format!("\"{}\"", entity.id))
            .field(
                "design source file",
                format!("\"{}\"", source_file(&entity.source_location)),
            )
            .field("design source line", source_line(&entity.source_location))
            .fix(format!(
                "add implementation class \"{}\" in the C++ class implementation, or remove it from the unit design",
                entity.id
            ));

        let error = if let Some(suggested_class) = suggested_class {
            error.suggest(&entity.id, Some("class"), suggested_class)
        } else {
            error
        };

        error.build()
    }

    fn format_missing_free_function(
        function: &FreeFunctionDecl,
        suggested_function: Option<&str>,
    ) -> String {
        let function_name = free_function_key(function);
        let error = ErrorBuilder::new(ErrorCategory::Class)
            .title(format!(
                "free function \"{function_name}\" from the unit design not found in the C++ implementation"
            ))
            .field("free function", format!("\"{function_name}\""))
            .field(
                "design source file",
                format!("\"{}\"", source_file(&function.source_location)),
            )
            .field("design source line", source_line(&function.source_location))
            .fix(format!(
                "add free function \"{function_name}\" to the C++ implementation, or remove it from the unit design"
            ));

        let error = if let Some(suggested_function) = suggested_function {
            error.suggest(&function_name, Some("free function"), suggested_function)
        } else {
            error
        };

        error.build()
    }

    fn format_free_function_mismatch(
        design_function: &FreeFunctionDecl,
        implementation_function: &FreeFunctionDecl,
        field: &str,
        design_value: &str,
        implementation_value: &str,
    ) -> String {
        let function_name = normalize_qualified_name(&design_function.qualified_name());
        ErrorBuilder::new(ErrorCategory::Implementation)
            .title(format!(
                "field {field} on free function \"{function_name}\" differs between the unit design and the C++ implementation"
            ))
            .field("free function", format!("\"{function_name}\""))
            .field("field", field)
            .field("design value", design_value)
            .field(
                "design source file",
                format!("\"{}\"", source_file(&design_function.source_location)),
            )
            .field(
                "design source line",
                source_line(&design_function.source_location),
            )
            .field("implementation value", implementation_value)
            .field(
                "implementation source file",
                format!(
                    "\"{}\"",
                    source_file(&implementation_function.source_location)
                ),
            )
            .field(
                "implementation source line",
                source_line(&implementation_function.source_location),
            )
            .fix(format!(
                "make {field} on free function \"{function_name}\" consistent between the unit design and the C++ implementation"
            ))
            .build()
    }

    fn format_missing_member(
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
        member_type: &str,
        member_name: &str,
        suggested_member: Option<&str>,
    ) -> String {
        let error = ErrorBuilder::new(ErrorCategory::Member)
            .title(format!(
                "{member_type} \"{member_name}\" from entity \"{}\" in the unit design not found in the C++ class implementation",
                design_entity.id
            ))
            .field("entity", format!("\"{}\"", design_entity.id))
            .field("member", format!("{member_type} \"{member_name}\""))
            .field(
                "design source file",
                format!("\"{}\"", source_file(&design_entity.source_location)),
            )
            .field("design source line", source_line(&design_entity.source_location))
            .fix(format!(
                "add {member_type} \"{member_name}\" to entity \"{}\" in the C++ class implementation, or remove it from the unit design",
                design_entity.id
            ));

        let error = if is_specialization(implementation_entity.template_kind) {
            error
                .field(
                    "implementation entity",
                    format!("\"{}\"", implementation_entity.id),
                )
                .field(
                    "implementation source file",
                    format!(
                        "\"{}\"",
                        source_file(&implementation_entity.source_location)
                    ),
                )
                .field(
                    "implementation source line",
                    source_line(&implementation_entity.source_location),
                )
        } else {
            error
        };

        let error = if let Some(suggested_member) = suggested_member {
            error.suggest(member_name, Some(member_type), suggested_member)
        } else {
            error
        };

        error.build()
    }

    fn format_mismatch(
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
        field: &str,
        field_reference_style: FieldReferenceStyle,
        design_value: &str,
        implementation_value: &str,
    ) -> String {
        let field_reference = format_field_reference(field, field_reference_style);

        let error = ErrorBuilder::new(ErrorCategory::Implementation)
            .title(format!(
                "field {field_reference} on entity \"{}\" differs between the unit design and the C++ class implementation",
                design_entity.id,
            ))
            .field("entity", format!("\"{}\"", design_entity.id))
            .field("field", field)
            .field("design value", design_value)
            .field(
                "design source file",
                format!("\"{}\"", source_file(&design_entity.source_location)),
            )
            .field("design source line", source_line(&design_entity.source_location));

        let error = if is_specialization(implementation_entity.template_kind) {
            error.field(
                "implementation entity",
                format!("\"{}\"", implementation_entity.id),
            )
        } else {
            error
        };

        error
            .field("implementation value", implementation_value)
            .field(
                "implementation source file",
                format!("\"{}\"", source_file(&implementation_entity.source_location)),
            )
            .field(
                "implementation source line",
                source_line(&implementation_entity.source_location),
            )
            .fix(format!(
                "make {field_reference} in entity \"{}\" consistent between the unit design and the C++ class implementation",
                design_entity.id
            ))
            .build()
    }
}

fn method_groups(entity: &SimpleEntity) -> BTreeMap<&str, Vec<&Method>> {
    let mut groups: BTreeMap<&str, Vec<&Method>> = BTreeMap::new();
    for method in &entity.methods {
        let group = groups.entry(&method.name).or_default();
        // Location is provenance, not callable identity. Compare every other
        // field exactly: directional type matching would erase conflicts.
        if !group.iter().any(|existing| {
            existing.name == method.name
                && existing.return_type == method.return_type
                && existing.visibility == method.visibility
                && existing.parameters == method.parameters
                && existing.template_parameters == method.template_parameters
                && existing.modifiers == method.modifiers
        }) {
            group.push(method);
        }
    }
    groups
}

fn free_function_groups(index: &ClassEntityIndex) -> BTreeMap<String, Vec<&FreeFunctionDecl>> {
    let mut groups: BTreeMap<String, Vec<&FreeFunctionDecl>> = BTreeMap::new();
    for function in index.free_functions() {
        let group = groups
            .entry(normalize_qualified_name(&function.qualified_name()))
            .or_default();
        if !group.iter().any(|existing| {
            existing.name == function.name
                && existing.enclosing_namespace_id == function.enclosing_namespace_id
                && existing.return_type == function.return_type
                && existing.parameters == function.parameters
                && existing.template_parameters == function.template_parameters
        }) {
            group.push(function);
        }
    }
    groups
}

fn format_overload_matching_failure<'a>(
    name: &str,
    outcome: &MatchingOutcome,
    design: impl Iterator<Item = (String, &'a SourceLocation)>,
    implementation: impl Iterator<Item = (String, &'a SourceLocation)>,
) -> String {
    let design = design.collect::<Vec<_>>();
    let implementation = implementation.collect::<Vec<_>>();
    let (title, reason, fix) = match outcome {
        MatchingOutcome::Ambiguous => (
            "ambiguous overload matching",
            if design.len() == 1 && implementation.len() == 2 {
                "Both implementation declarations match the design's parameter prefix. A unique implementation declaration cannot be selected."
            } else {
                "Multiple one-to-one matchings cover all design declarations. A unique assignment of implementation declarations cannot be selected."
            },
            "Add function parameters or additional overload design declarations so that the overload group has a unique one-to-one matching.",
        ),
        MatchingOutcome::NoSolution => (
            "no injective overload matching covers all design declarations",
            "Design declarations compete for the same implementation declarations. No one-to-one matching can cover all design declarations.",
            "Provide distinct matching implementation overloads or correct the design so that each design declaration can select a different implementation declaration.",
        ),
        MatchingOutcome::Unique(_) => unreachable!("unique matchings are not group failures"),
    };
    let error = ErrorBuilder::new(ErrorCategory::Implementation)
        .title(format!("{title} for {name}"))
        .field("Reason", reason);
    let error = append_declaration_fields(error, "Design", design);
    append_declaration_fields(error, "Implementation", implementation)
        .fix(fix)
        .build()
}

fn append_declaration_fields<'a>(
    mut error: ErrorBuilder,
    label: &str,
    items: impl IntoIterator<Item = (String, &'a SourceLocation)>,
) -> ErrorBuilder {
    // Sort only diagnostic values, never candidates or matching priorities.
    // Keep identical rendered declarations, including identical locations.
    let mut items = items.into_iter().collect::<Vec<_>>();
    items.sort_by_key(|(declaration, location)| {
        (
            declaration.clone(),
            source_file(location),
            location.display().1,
        )
    });
    let numbered = items.len() != 1;
    for (index, (declaration, location)) in items.into_iter().enumerate() {
        let suffix = if numbered {
            format!(" {}", index + 1)
        } else {
            String::new()
        };
        error = error
            .field(format!("{label} value{suffix}"), declaration)
            .field(
                format!("{label} source file{suffix}"),
                format!("\"{}\"", source_file(location)),
            )
            .field(
                format!("{label} source line{suffix}"),
                source_line(location),
            );
    }
    error
}

fn format_function_declaration(
    name: &str,
    parameters: &[class_diagram::FunctionArgument],
) -> String {
    // Diagnostic spelling is raw: do not normalize types or literal whitespace.
    let parameters = parameters.iter().map(|parameter| {
        if parameter.is_variadic {
            return "...".to_string();
        }

        let mut rendered = match (parameter.name.is_empty(), parameter.param_type.as_deref()) {
            (true, data_type) => data_type.unwrap_or("<unknown>").to_string(),
            (false, data_type) => {
                format!("{}: {}", parameter.name, data_type.unwrap_or("<unknown>"))
            }
        };
        if parameter.is_pack_expansion {
            rendered.push_str("...");
        }
        rendered
    });
    format!("{name}({})", render_joined(parameters))
}

fn format_field_reference(field: &str, style: FieldReferenceStyle) -> String {
    match style {
        FieldReferenceStyle::Quoted => format!("\"{field}\""),
        FieldReferenceStyle::Verbatim => field.to_string(),
    }
}

fn source_file(source_location: &SourceLocation) -> String {
    let (file, _) = source_location.display();

    if file.is_empty() {
        "<unknown>".to_string()
    } else {
        file
    }
}

fn source_line(source_location: &SourceLocation) -> String {
    let (_, line) = source_location.display();

    if line == 0 {
        "<unknown>".to_string()
    } else {
        line.to_string()
    }
}

fn format_class_comparison_debug(
    design_entity: &SimpleEntity,
    implementation_entity: &SimpleEntity,
) -> String {
    format!(
        "Comparing design entity {:?} with implementation entity {:?}",
        design_entity.id, implementation_entity.id
    )
}

fn format_free_function_comparison_debug(
    design_function: &FreeFunctionDecl,
    implementation_function: &FreeFunctionDecl,
) -> String {
    format!(
        "Comparing design free function {:?} with implementation free function {:?}",
        design_function.qualified_name(),
        implementation_function.qualified_name()
    )
}

fn format_free_function_trace(label: &str, function: &FreeFunctionDecl) -> String {
    format!(
        "{label} {:?} details:\n\
         namespace={:?}\n\
         return_type={:?}\n\
         parameters={:?}\n\
         template_parameters={:?}",
        function.qualified_name(),
        function.enclosing_namespace_id,
        function.return_type,
        function.parameters,
        function.template_parameters,
    )
}

fn format_entity_trace(label: &str, entity: &SimpleEntity) -> String {
    format!(
        "{label} {:?} details:\n\
         name={:?}\n\
         namespace={:?}\n\
         type={:?}\n\
         type_aliases={:?}\n\
         variables={:?}\n\
         methods={:?}\n\
         template_parameters={:?}\n\
         enum_literals={:?}\n\
         relationships={:?}",
        entity.id,
        entity.name,
        entity.enclosing_namespace_id,
        entity.entity_type,
        entity.type_aliases,
        entity.variables,
        entity.methods,
        entity.template_parameters,
        entity.enum_literals,
        entity.relationships
    )
}

fn type_alias_map(entity: &SimpleEntity) -> BTreeMap<&str, &TypeAlias> {
    entity
        .type_aliases
        .iter()
        .map(|type_alias| (type_alias.alias.as_str(), type_alias))
        .collect()
}

fn type_aliases_match(design_alias: &TypeAlias, implementation_alias: &TypeAlias) -> bool {
    design_alias.alias == implementation_alias.alias
        && comparison_types_match(
            &design_alias.original_type,
            &implementation_alias.original_type,
        )
}

fn variable_map(entity: &SimpleEntity) -> BTreeMap<&str, &MemberVariable> {
    entity
        .variables
        .iter()
        .map(|variable| (variable.name.as_str(), variable))
        .collect()
}

fn member_variables_match(
    design_variable: &MemberVariable,
    implementation_variable: &MemberVariable,
) -> bool {
    design_variable.name == implementation_variable.name
        && optional_types_match(
            &design_variable.data_type,
            &implementation_variable.data_type,
        )
        && design_variable.visibility == implementation_variable.visibility
        && design_variable.is_static == implementation_variable.is_static
}

fn free_function_map(index: &ClassEntityIndex) -> BTreeMap<String, &FreeFunctionDecl> {
    index
        .free_functions()
        .map(|function| (free_function_key(function), function))
        .collect()
}

fn method_map(entity: &SimpleEntity) -> BTreeMap<String, &Method> {
    entity
        .methods
        .iter()
        .map(|method| (method_key(method), method))
        .collect()
}

fn methods_match(design_method: &Method, implementation_method: &Method) -> bool {
    design_method.name == implementation_method.name
        && optional_types_match(
            &design_method.return_type,
            &implementation_method.return_type,
        )
        && design_method.visibility == implementation_method.visibility
        && parameters_match(&design_method.parameters, &implementation_method.parameters)
        && template_parameters_match(
            &design_method.template_parameters,
            &implementation_method.template_parameters,
        )
        && design_method.modifiers == implementation_method.modifiers
}

fn template_parameters_match(
    design: &Option<Vec<TemplateParameter>>,
    implementation: &Option<Vec<TemplateParameter>>,
) -> bool {
    match (design, implementation) {
        (None, None) => true,
        (Some(design), Some(implementation)) => {
            template_parameter_lists_match(design, implementation)
        }
        _ => false,
    }
}

fn template_parameter_lists_match(
    design: &[TemplateParameter],
    implementation: &[TemplateParameter],
) -> bool {
    parameter_count_matches(design.len(), implementation.len())
        && design
            .iter()
            .zip(implementation)
            .all(|(design, implementation)| {
                // Omitting a name in the design leaves only that name unconstrained.
                if !design.name().is_empty() && design.name() != implementation.name() {
                    return false;
                }

                match (design, implementation) {
                    (
                        TemplateParameter::Type {
                            is_pack: design_pack,
                            ..
                        },
                        TemplateParameter::Type {
                            is_pack: implementation_pack,
                            ..
                        },
                    ) => design_pack == implementation_pack,
                    (
                        TemplateParameter::NonType {
                            value_type: design_type,
                            is_pack: design_pack,
                            ..
                        },
                        TemplateParameter::NonType {
                            value_type: implementation_type,
                            is_pack: implementation_pack,
                            ..
                        },
                    ) => {
                        design_pack == implementation_pack
                            && normalize_non_type_template_parameter_type(design_type)
                                == normalize_non_type_template_parameter_type(implementation_type)
                    }
                    (
                        TemplateParameter::Template {
                            parameters: design_parameters,
                            is_pack: design_pack,
                            ..
                        },
                        TemplateParameter::Template {
                            parameters: implementation_parameters,
                            is_pack: implementation_pack,
                            ..
                        },
                    ) => {
                        design_pack == implementation_pack
                            && template_parameter_lists_match(
                                design_parameters,
                                implementation_parameters,
                            )
                    }
                    _ => false,
                }
            })
}

fn parameters_match(
    design_parameters: &[class_diagram::FunctionArgument],
    implementation_parameters: &[class_diagram::FunctionArgument],
) -> bool {
    parameter_types_match(design_parameters, implementation_parameters)
        && design_parameters
            .iter()
            .zip(implementation_parameters)
            .all(|(design, implementation)| design.name == implementation.name)
}

/// Parameter compatibility excludes names, return types, visibility, template
/// declarations and modifiers. Method candidates additionally require equal const
/// qualification; remaining fields are checked only after selection.
fn parameter_types_match(
    design: &[class_diagram::FunctionArgument],
    implementation: &[class_diagram::FunctionArgument],
) -> bool {
    parameter_count_matches(design.len(), implementation.len())
        && design
            .iter()
            .zip(implementation)
            .all(|(design, implementation)| {
                optional_types_match(&design.param_type, &implementation.param_type)
                    && design.is_variadic == implementation.is_variadic
                    && design.is_pack_expansion == implementation.is_pack_expansion
            })
}

fn parameter_count_matches(design: usize, implementation: usize) -> bool {
    if design == 0 {
        implementation == 0
    } else {
        design <= implementation
    }
}

fn free_function_key(function: &FreeFunctionDecl) -> String {
    let parameter_types = parameter_signature(&function.parameters);
    format!(
        "{}({parameter_types})",
        normalize_qualified_name(&function.qualified_name())
    )
}

fn method_key(method: &Method) -> String {
    let parameter_types = parameter_signature(&method.parameters);
    let const_qualifier = if method.modifiers.contains(&MethodModifier::Const) {
        " const"
    } else {
        ""
    };
    format!("{}({}){}", method.name, parameter_types, const_qualifier)
}

fn parameter_signature(parameters: &[class_diagram::FunctionArgument]) -> String {
    parameters
        .iter()
        .map(|parameter| {
            let variadic = if parameter.is_variadic || parameter.is_pack_expansion {
                "..."
            } else {
                ""
            };
            format!(
                "{}{}",
                parameter
                    .param_type
                    .as_deref()
                    .map(render_type_name)
                    .unwrap_or_default(),
                variadic
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn parameter_differences(
    design_parameters: &[class_diagram::FunctionArgument],
    implementation_parameters: &[class_diagram::FunctionArgument],
) -> Vec<(ParameterDifference, String, String)> {
    if !parameter_count_matches(design_parameters.len(), implementation_parameters.len()) {
        return vec![(
            ParameterDifference::Count,
            format_parameter_count(design_parameters),
            format_parameter_count(implementation_parameters),
        )];
    }

    let mut differences = Vec::new();
    for (design_parameter, implementation_parameter) in
        design_parameters.iter().zip(implementation_parameters)
    {
        if design_parameter.name != implementation_parameter.name {
            differences.push((
                ParameterDifference::Name,
                format_parameter(design_parameter),
                format_parameter(implementation_parameter),
            ));
        }

        if !optional_types_match(
            &design_parameter.param_type,
            &implementation_parameter.param_type,
        ) {
            differences.push((
                ParameterDifference::Type,
                format_parameter(design_parameter),
                format_parameter(implementation_parameter),
            ));
        }

        if design_parameter.is_variadic != implementation_parameter.is_variadic {
            differences.push((
                ParameterDifference::Variadic,
                format_method_parameters(design_parameters),
                format_method_parameters(implementation_parameters),
            ));
        }

        if design_parameter.is_pack_expansion != implementation_parameter.is_pack_expansion {
            differences.push((
                ParameterDifference::PackExpansion,
                format_method_parameters(design_parameters),
                format_method_parameters(implementation_parameters),
            ));
        }
    }

    differences
}

fn parameter_difference_name(difference: ParameterDifference) -> &'static str {
    match difference {
        ParameterDifference::Count => "count",
        ParameterDifference::Name => "name",
        ParameterDifference::Type => "type",
        ParameterDifference::Variadic => "variadic",
        ParameterDifference::PackExpansion => "pack expansion",
    }
}

fn free_functions_match(
    design_function: &FreeFunctionDecl,
    implementation_function: &FreeFunctionDecl,
) -> bool {
    normalize_qualified_name(&design_function.qualified_name())
        == normalize_qualified_name(&implementation_function.qualified_name())
        && optional_types_match(
            &design_function.return_type,
            &implementation_function.return_type,
        )
        && parameters_match(
            &design_function.parameters,
            &implementation_function.parameters,
        )
        && template_parameters_match(
            &design_function.template_parameters,
            &implementation_function.template_parameters,
        )
}

fn render_optional_value<T, F>(value: Option<T>, default: &str, format_value: F) -> String
where
    F: FnOnce(T) -> String,
{
    value
        .map(format_value)
        .unwrap_or_else(|| default.to_string())
}

fn render_list(items: impl Iterator<Item = String>) -> String {
    format!("[{}]", render_joined(items))
}

fn render_joined(items: impl Iterator<Item = String>) -> String {
    items.collect::<Vec<_>>().join(", ")
}

fn render_optional_type(type_name: &Option<String>) -> String {
    render_optional_value(type_name.as_deref(), "None", render_type_name)
}

fn render_parameter_type(type_name: &Option<String>) -> String {
    render_optional_value(type_name.as_deref(), "<unknown>", render_type_name)
}

fn render_type_name(type_name: &str) -> String {
    if contains_literal(type_name) {
        type_name.to_string()
    } else {
        normalize_type_name(type_name)
    }
}

fn format_parameter(parameter: &class_diagram::FunctionArgument) -> String {
    if parameter.name.is_empty() {
        render_parameter_type(&parameter.param_type)
    } else {
        format!(
            "{}: {}",
            parameter.name,
            render_parameter_type(&parameter.param_type),
        )
    }
}

fn format_parameter_list(parameters: &[class_diagram::FunctionArgument]) -> String {
    parameters
        .iter()
        .map(|parameter| {
            if parameter.is_variadic {
                return "...".to_string();
            }

            let mut rendered = format_parameter(parameter);
            if parameter.is_pack_expansion {
                rendered.push_str("...");
            }
            rendered
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_method_parameters(parameters: &[class_diagram::FunctionArgument]) -> String {
    format!("({})", format_parameter_list(parameters))
}

fn format_parameter_count(parameters: &[class_diagram::FunctionArgument]) -> String {
    let count = parameters.len();
    let label = if count == 1 {
        "parameter"
    } else {
        "parameters"
    };
    format!(
        "{} {} {}",
        count,
        label,
        format_method_parameters(parameters)
    )
}

fn render_template_parameters(template_parameters: &Option<Vec<TemplateParameter>>) -> String {
    render_optional_value(template_parameters.as_deref(), "None", |parameters| {
        render_list(parameters.iter().map(render_template_parameter))
    })
}

fn render_template_parameter(parameter: &TemplateParameter) -> String {
    let content = match parameter {
        TemplateParameter::Type { name, is_pack } => render_template_parameter_name(name, *is_pack),
        TemplateParameter::NonType { .. } | TemplateParameter::Template { .. } => {
            render_template_parameter_content(parameter)
        }
    };
    format!("\"{content}\"")
}

fn render_template_parameter_name(name: &str, is_pack: bool) -> String {
    format!("{}{}", name, template_pack_marker(is_pack))
}

fn render_template_parameter_list(parameters: &[TemplateParameter]) -> String {
    render_joined(parameters.iter().map(render_template_parameter_content))
}

fn render_template_parameter_content(parameter: &TemplateParameter) -> String {
    match parameter {
        TemplateParameter::Type { name, is_pack } => {
            render_template_parameter_declaration("typename", name, *is_pack)
        }
        TemplateParameter::NonType {
            name,
            value_type,
            is_pack,
        } => render_template_parameter_declaration(value_type, name, *is_pack),
        TemplateParameter::Template {
            name,
            parameters,
            is_pack,
        } => render_template_parameter_declaration(
            &format!(
                "template <{}> typename",
                render_template_parameter_list(parameters)
            ),
            name,
            *is_pack,
        ),
    }
}

fn render_template_parameter_declaration(prefix: &str, name: &str, is_pack: bool) -> String {
    let mut declaration = format!("{}{}", prefix.trim(), template_pack_marker(is_pack));
    if !name.is_empty() {
        declaration.push(' ');
        declaration.push_str(name);
    }
    declaration
}

fn template_pack_marker(is_pack: bool) -> &'static str {
    if is_pack {
        "..."
    } else {
        ""
    }
}

fn optional_types_match(design: &Option<String>, implementation: &Option<String>) -> bool {
    match (design.as_deref(), implementation.as_deref()) {
        (None, None) => true,
        (Some(design), Some(implementation)) => comparison_types_match(design, implementation),
        _ => false,
    }
}

fn comparison_types_match(design: &str, implementation: &str) -> bool {
    // Conservative literal detection also covers raw and character literals.
    // Never rewrite namespaces, punctuation or whitespace inside their contents.
    if contains_literal(design) || contains_literal(implementation) {
        design == implementation
    } else {
        normalize_type_name(design) == normalize_type_name(implementation)
    }
}

fn contains_literal(type_name: &str) -> bool {
    type_name.contains('"') || type_name.contains('\'')
}

fn normalize_non_type_template_parameter_type(type_name: &str) -> String {
    // Preserve existing behavior for potentially literal-bearing expressions.
    // Whitespace within literals is significant and must not be collapsed.
    if contains_literal(type_name) {
        return type_name.to_string();
    }

    normalize_type_name(&type_name.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn normalize_type_name(type_name: &str) -> String {
    normalize_qualified_name(
        &type_name
            .trim()
            .trim_start_matches("::")
            .replace("std::", "")
            .replace(" *", "*")
            .replace(" &", "&"),
    )
}

fn normalize_qualified_name(reference: &str) -> String {
    // TODO: Remove this workaround once class diagram and implementation parser UIDs use the same namespace separator.
    reference.trim().replace('.', "::")
}

fn is_specialization(kind: ClassTemplateKind) -> bool {
    matches!(
        kind,
        ClassTemplateKind::PartialSpecialization | ClassTemplateKind::ExplicitSpecialization
    )
}

fn specialization_base_id(id: &str) -> &str {
    trailing_template_arguments_start(id).map_or(id, |start| id[..start].trim())
}

fn trailing_template_arguments_start(id: &str) -> Option<usize> {
    let mut angle_depth = 0usize;
    let mut specialization_arguments_start = None;

    for (index, character) in id.char_indices() {
        match character {
            '<' if angle_depth == 0 => {
                specialization_arguments_start = Some(index);
                angle_depth = 1;
            }
            '<' => angle_depth += 1,
            '>' if angle_depth > 0 => {
                angle_depth -= 1;
                if angle_depth == 0 {
                    let suffix = id[index + character.len_utf8()..].trim_start();
                    if suffix.is_empty() {
                        return specialization_arguments_start;
                    }
                    if suffix.starts_with("::") {
                        specialization_arguments_start = None;
                    }
                }
            }
            _ => {}
        }
    }

    None
}

fn specialization_matches_design(implementation_id: &str, normalized_design_id: &str) -> bool {
    normalize_qualified_name(specialization_base_id(implementation_id))
        .eq_ignore_ascii_case(normalized_design_id)
}

fn enum_literal_map(entity: &SimpleEntity) -> BTreeMap<&str, &EnumLiteral> {
    entity
        .enum_literals
        .iter()
        .map(|literal| (literal.name.as_str(), literal))
        .collect()
}

fn enum_literals_match(design_literal: &EnumLiteral, implementation_literal: &EnumLiteral) -> bool {
    design_literal.name == implementation_literal.name
        && design_literal.value == implementation_literal.value
}

fn enum_literal_value(literal: &EnumLiteral) -> String {
    match &literal.value {
        Some(value) => format!("{:?}", value),
        None => "None".to_string(),
    }
}

fn relationship_map(entity: &SimpleEntity) -> BTreeMap<String, Vec<&Relationship>> {
    let mut relationships: BTreeMap<String, Vec<&Relationship>> = BTreeMap::new();
    for relationship in &entity.relationships {
        relationships
            .entry(relationship_key(relationship))
            .or_default()
            .push(relationship);
    }
    relationships
}

fn relationship_key(relationship: &Relationship) -> String {
    format!(
        "{} -> {}",
        normalize_qualified_name(&relationship.source),
        normalize_qualified_name(&relationship.target)
    )
}

fn relationship_display_name(relationship: &Relationship) -> String {
    format!(
        "{} -> {:?} -> {}",
        normalize_qualified_name(&relationship.source),
        relationship.relation_type,
        normalize_qualified_name(&relationship.target)
    )
}

#[cfg(test)]
#[path = "test/class_design_implementation_validator_test.rs"]
mod tests;
