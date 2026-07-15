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
//! implementation class diagrams produced by the C++ parser.

use crate::models::ClassEntityIndex;
use crate::ValidationResult;
use class_diagram::{
    EnumLiteral, MemberVariable, Method, Relationship, SimpleEntity, TemplateParameter, TypeAlias,
};
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
        self.result
    }

    fn check_design_classes_have_implementation(
        &mut self,
        design_classes: &ClassEntityIndex,
        implementation_classes: &ClassEntityIndex,
    ) {
        for design_entity in design_classes.entities() {
            let normalized_design_id = normalize_reference_name(&design_entity.id);
            let Some(implementation_entity) = implementation_classes
                .find_by_id(&design_entity.id)
                .or_else(|| implementation_classes.find_by_id(&normalized_design_id))
            else {
                self.result
                    .add_failure(Self::format_missing_class(design_entity));
                continue;
            };

            self.append_class_comparison_diagnostics(design_entity, implementation_entity);
            self.check_entity(design_entity, implementation_entity);
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

    fn check_entity(&mut self, design_entity: &SimpleEntity, implementation_entity: &SimpleEntity) {
        if design_entity.entity_type != implementation_entity.entity_type {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                "entity_type",
                &format!("{:?}", design_entity.entity_type),
                &format!("{:?}", implementation_entity.entity_type),
            ));
        }

        if design_entity.template_parameters != implementation_entity.template_parameters {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                "template_parameters",
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
                    &format!("type_alias {:?} original_type", design_alias.alias.as_str()),
                    &design_alias.original_type,
                    &implementation_alias.original_type,
                )),
                None => self.result.add_failure(Self::format_missing_member(
                    design_entity,
                    "type_alias",
                    &design_alias.alias,
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
                    "variable",
                    &design_variable.name,
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

        if normalized_optional_type(&design_variable.data_type)
            != normalized_optional_type(&implementation_variable.data_type)
        {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("variable {variable_name:?} data_type"),
                &render_optional_type(&design_variable.data_type),
                &render_optional_type(&implementation_variable.data_type),
            ));
        }

        if design_variable.visibility != implementation_variable.visibility {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("variable {variable_name:?} visibility"),
                &format!("{:?}", design_variable.visibility),
                &format!("{:?}", implementation_variable.visibility),
            ));
        }

        if design_variable.is_static != implementation_variable.is_static {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("variable {variable_name:?} is_static"),
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
        let implementation_methods = method_map(implementation_entity);
        for design_method in &design_entity.methods {
            let key = method_key(design_method);
            match implementation_methods.get(key.as_str()) {
                Some(implementation_method)
                    if methods_match(design_method, implementation_method) => {}
                Some(implementation_method) => self.check_method_fields(
                    design_entity,
                    implementation_entity,
                    design_method,
                    implementation_method,
                ),
                // NOTE: Parameter mismatch diagnostics are emitted only when a same-named
                // implementation method is unique. Overloaded methods are left as missing full
                // signatures because the validator cannot safely choose a candidate yet.
                None => {
                    match unique_method_index_by_name(implementation_entity, &design_method.name)
                        .and_then(|index| implementation_entity.methods.get(index))
                    {
                        Some(implementation_method) => self.check_method_fields(
                            design_entity,
                            implementation_entity,
                            design_method,
                            implementation_method,
                        ),
                        None => self.result.add_failure(Self::format_missing_member(
                            design_entity,
                            "method",
                            &key,
                        )),
                    }
                }
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

        if normalized_optional_type(&design_method.return_type)
            != normalized_optional_type(&implementation_method.return_type)
        {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("method {method_name:?} return_type"),
                &render_optional_type(&design_method.return_type),
                &render_optional_type(&implementation_method.return_type),
            ));
        }

        if design_method.visibility != implementation_method.visibility {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("method {method_name:?} visibility"),
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

        if design_method.template_parameters != implementation_method.template_parameters {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("method {method_name:?} template_parameters"),
                &render_template_parameters(&design_method.template_parameters),
                &render_template_parameters(&implementation_method.template_parameters),
            ));
        }

        if design_method.modifiers != implementation_method.modifiers {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("method {method_name:?} modifiers"),
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

        if design_method.parameters.len() != implementation_method.parameters.len() {
            self.result.add_failure(Self::format_mismatch(
                design_entity,
                implementation_entity,
                &format!("method {method_name:?} parameter_count"),
                &format_parameter_count(&design_method.parameters),
                &format_parameter_count(&implementation_method.parameters),
            ));
            return;
        }

        for (design_parameter, implementation_parameter) in design_method
            .parameters
            .iter()
            .zip(&implementation_method.parameters)
        {
            if design_parameter.name != implementation_parameter.name {
                self.result.add_failure(Self::format_mismatch(
                    design_entity,
                    implementation_entity,
                    &format!("method {method_name:?} parameter name"),
                    &format_parameter(design_parameter),
                    &format_parameter(implementation_parameter),
                ));
            }

            if normalized_optional_type(&design_parameter.param_type)
                != normalized_optional_type(&implementation_parameter.param_type)
            {
                self.result.add_failure(Self::format_mismatch(
                    design_entity,
                    implementation_entity,
                    &format!("method {method_name:?} parameter type"),
                    &format_parameter(design_parameter),
                    &format_parameter(implementation_parameter),
                ));
            }

            if design_parameter.is_pack_expansion != implementation_parameter.is_pack_expansion {
                self.result.add_failure(Self::format_mismatch(
                    design_entity,
                    implementation_entity,
                    &format!("method {method_name:?} parameter list"),
                    &format_method_parameters(&design_method.parameters),
                    &format_method_parameters(&implementation_method.parameters),
                ));
            }
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
                    &enum_literal_value(design_literal),
                    &enum_literal_value(implementation_literal),
                )),
                None => self.result.add_failure(Self::format_missing_member(
                    design_entity,
                    "enum_literal",
                    &design_literal.name,
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
                Some(implementation_relationship)
                    if design_relationship.relation_type
                        == implementation_relationship.relation_type => {}
                Some(implementation_relationship) => {
                    self.result.add_failure(Self::format_mismatch(
                        design_entity,
                        implementation_entity,
                        "relationship type",
                        &display_name,
                        &relationship_display_name(implementation_relationship),
                    ))
                }
                None => self.result.add_failure(Self::format_missing_member(
                    design_entity,
                    "relationship",
                    &display_name,
                )),
            }
        }
    }

    fn format_missing_class(entity: &SimpleEntity) -> String {
        let (design_file, design_line) = entity.source_location.display();
        format!(
            "Missing implementation class for unit design entity:\n\
                Entity ID          : {}\n\
                Design source file : {}\n\
                Design source line : {}\n\
                Required Action    : Add a matching implementation class or update the unit design",
            entity.id, design_file, design_line,
        )
    }

    fn format_missing_member(
        design_entity: &SimpleEntity,
        member_type: &str,
        member_name: &str,
    ) -> String {
        let (design_file, design_line) = design_entity.source_location.display();
        format!(
            "Missing implementation {member_type} for unit design entity:\n\
                Entity ID          : {}\n\
                Member             : {}\n\
                Design source file : {}\n\
                Design source line : {}\n\
                Required Action    : Implement the member or update the unit design",
            design_entity.id, member_name, design_file, design_line,
        )
    }

    fn format_mismatch(
        design_entity: &SimpleEntity,
        implementation_entity: &SimpleEntity,
        field: &str,
        design_value: &str,
        implementation_value: &str,
    ) -> String {
        let (design_file, design_line) = design_entity.source_location.display();
        let (implement_file, implement_line) = implementation_entity.source_location.display();
        format!(
            "Implementation class data differs from unit design entity:\n\
                Entity ID             : {}\n\
                Field                 : {}\n\
                Design value          : {}\n\
                Design source file    : {}\n\
                Design source line    : {}\n\
                Implement value       : {}\n\
                Implement source file : {}\n\
                Implement source line : {}\n\
                Required Action       : Align the implementation with the unit design or update the unit design",
            design_entity.id,
            field,
            design_value,
            design_file,
            design_line,
            implementation_value,
            implement_file,
            implement_line
        )
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
        && normalize_type_name(&design_alias.original_type)
            == normalize_type_name(&implementation_alias.original_type)
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
        && normalized_optional_type(&design_variable.data_type)
            == normalized_optional_type(&implementation_variable.data_type)
        && design_variable.visibility == implementation_variable.visibility
        && design_variable.is_static == implementation_variable.is_static
}

fn method_map(entity: &SimpleEntity) -> BTreeMap<String, &Method> {
    entity
        .methods
        .iter()
        .map(|method| (method_key(method), method))
        .collect()
}

fn unique_method_index_by_name(entity: &SimpleEntity, method_name: &str) -> Option<usize> {
    let mut matching_methods = entity
        .methods
        .iter()
        .enumerate()
        .filter(|(_, method)| method.name == method_name);
    let (index, _) = matching_methods.next()?;
    matching_methods.next().is_none().then_some(index)
}

fn methods_match(design_method: &Method, implementation_method: &Method) -> bool {
    design_method.name == implementation_method.name
        && normalized_optional_type(&design_method.return_type)
            == normalized_optional_type(&implementation_method.return_type)
        && design_method.visibility == implementation_method.visibility
        && parameters_match(&design_method.parameters, &implementation_method.parameters)
        && design_method.template_parameters == implementation_method.template_parameters
        && design_method.modifiers == implementation_method.modifiers
}

fn parameters_match(
    design_parameters: &[class_diagram::FunctionArgument],
    implementation_parameters: &[class_diagram::FunctionArgument],
) -> bool {
    design_parameters.len() == implementation_parameters.len()
        && design_parameters.iter().zip(implementation_parameters).all(
            |(design_parameter, implementation_parameter)| {
                design_parameter.name == implementation_parameter.name
                    && normalized_optional_type(&design_parameter.param_type)
                        == normalized_optional_type(&implementation_parameter.param_type)
                    // Note: Puml parser not support C-style variadic parameters, design_parameter.is_variadic is always false now.
                    // && design_parameter.is_variadic == implementation_parameter.is_variadic
                    && design_parameter.is_pack_expansion == implementation_parameter.is_pack_expansion
            },
        )
}

fn method_key(method: &Method) -> String {
    let parameter_types = method
        .parameters
        .iter()
        .map(|parameter| {
            let variadic = if parameter.is_pack_expansion {
                "..."
            } else {
                ""
            };
            format!(
                "{}{}",
                parameter
                    .param_type
                    .as_deref()
                    .map(normalize_type_name)
                    .unwrap_or_default(),
                variadic
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({})", method.name, parameter_types)
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
    render_optional_value(type_name.as_deref(), "None", normalize_type_name)
}

fn render_parameter_type(type_name: &Option<String>) -> String {
    render_optional_value(type_name.as_deref(), "<unknown>", normalize_type_name)
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
    match parameter {
        TemplateParameter::Type { name, is_pack } => {
            render_quoted_template_parameter(name, *is_pack)
        }
        TemplateParameter::NonType { name, is_pack, .. } => {
            render_quoted_template_parameter(name, *is_pack)
        }
        TemplateParameter::Template {
            name,
            parameters,
            is_pack,
        } => format!(
            "\"template <{}> {}{}\"",
            render_template_parameter_list(parameters),
            name,
            template_pack_marker(*is_pack)
        ),
    }
}

fn render_quoted_template_parameter(name: &str, is_pack: bool) -> String {
    format!("\"{}\"", render_template_parameter_name(name, is_pack))
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
            format!(
                "typename {}",
                render_template_parameter_name(name, *is_pack)
            )
        }
        TemplateParameter::NonType {
            name,
            value_type,
            is_pack,
        } => format!(
            "{} {}",
            value_type,
            render_template_parameter_name(name, *is_pack)
        ),
        TemplateParameter::Template {
            name,
            parameters,
            is_pack,
        } => format!(
            "template <{}> typename {}{}",
            render_template_parameter_list(parameters),
            name,
            template_pack_marker(*is_pack)
        ),
    }
}

fn template_pack_marker(is_pack: bool) -> &'static str {
    if is_pack {
        "..."
    } else {
        ""
    }
}

fn normalized_optional_type(type_name: &Option<String>) -> Option<String> {
    type_name.as_deref().map(normalize_type_name)
}

fn normalize_type_name(type_name: &str) -> String {
    normalize_reference_name(
        &type_name
            .trim()
            .trim_start_matches("::")
            .replace("std::", "")
            .replace(" *", "*")
            .replace(" &", "&"),
    )
}

fn normalize_reference_name(reference: &str) -> String {
    // TODO: Remove this workaround once class diagram and implementation parser UIDs use the same namespace separator.
    reference.trim().replace('.', "::")
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

fn relationship_map(entity: &SimpleEntity) -> BTreeMap<String, &Relationship> {
    entity
        .relationships
        .iter()
        .map(|relationship| (relationship_key(relationship), relationship))
        .collect()
}

fn relationship_key(relationship: &Relationship) -> String {
    format!(
        "{} -> {}",
        normalize_reference_name(&relationship.source),
        normalize_reference_name(&relationship.target)
    )
}

fn relationship_display_name(relationship: &Relationship) -> String {
    format!(
        "{} -> {:?} -> {}",
        normalize_reference_name(&relationship.source),
        relationship.relation_type,
        normalize_reference_name(&relationship.target)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ClassDiagramInputs;
    use class_diagram::{
        ClassDiagram, EntityType, FunctionArgument, MemberVariable, Method, RelationType,
        Relationship, SimpleEntity, SourceLocation, Visibility,
    };
    use std::sync::Once;

    struct TestLogger;

    impl log::Log for TestLogger {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, _record: &log::Record<'_>) {}

        fn flush(&self) {}
    }

    static LOGGER: TestLogger = TestLogger;
    static INIT_LOGGER: Once = Once::new();

    fn enable_trace_diagnostics() {
        INIT_LOGGER.call_once(|| {
            let _ = log::set_logger(&LOGGER);
        });
        log::set_max_level(log::LevelFilter::Trace);
    }

    fn method(name: &str) -> Method {
        Method {
            name: name.to_string(),
            return_type: None,
            source_location: SourceLocation::new("test.puml", 1),
            visibility: Visibility::Public,
            parameters: Vec::new(),
            template_parameters: None,
            modifiers: Vec::new(),
        }
    }

    fn method_with_return_type(name: &str, return_type: &str) -> Method {
        Method {
            return_type: Some(return_type.to_string()),
            ..method(name)
        }
    }

    fn method_with_parameter_types(name: &str, parameter_types: &[&str]) -> Method {
        Method {
            parameters: parameter_types
                .iter()
                .map(|parameter_type| FunctionArgument {
                    name: String::new(),
                    param_type: Some((*parameter_type).to_string()),
                    is_variadic: false,
                    is_pack_expansion: false,
                })
                .collect(),
            ..method(name)
        }
    }

    fn parameter(name: &str, param_type: &str, is_pack_expansion: bool) -> FunctionArgument {
        FunctionArgument {
            name: name.to_string(),
            param_type: Some(param_type.to_string()),
            is_variadic: false,
            is_pack_expansion,
        }
    }

    fn entity(id: &str, methods: Vec<&str>) -> SimpleEntity {
        entity_in_namespace(id, None, methods)
    }

    fn variable(name: &str, data_type: &str) -> MemberVariable {
        MemberVariable {
            name: name.to_string(),
            data_type: Some(data_type.to_string()),
            visibility: Visibility::Private,
            is_static: false,
            source_location: SourceLocation::new("test.puml", 1),
        }
    }

    fn relationship(source: &str, target: &str, relation_type: RelationType) -> Relationship {
        Relationship {
            source: source.to_string(),
            target: target.to_string(),
            relation_type,
            source_multiplicity: None,
            target_multiplicity: None,
            source_location: SourceLocation::new("test.puml", 1),
        }
    }

    fn entity_in_namespace(
        id: &str,
        enclosing_namespace_id: Option<&str>,
        methods: Vec<&str>,
    ) -> SimpleEntity {
        SimpleEntity {
            id: id.to_string(),
            name: id.rsplit('.').next().unwrap_or(id).to_string(),
            enclosing_namespace_id: enclosing_namespace_id.map(str::to_string),
            entity_type: EntityType::Class,
            type_aliases: Vec::new(),
            variables: Vec::new(),
            methods: methods.into_iter().map(method).collect(),
            template_parameters: None,
            enum_literals: Vec::new(),
            relationships: Vec::new(),
            source_location: SourceLocation::new("test.puml", 1),
        }
    }

    fn index(entities: Vec<SimpleEntity>) -> ClassEntityIndex {
        let diagrams: ClassDiagramInputs = vec![ClassDiagram {
            name: "unit".to_string(),
            entities,
            relationships: Vec::new(),
            source_files: Vec::new(),
            version: None,
        }];
        ClassEntityIndex::build_index(&diagrams, &mut ValidationResult::default())
    }

    #[test]
    fn method_key_normalizes_parameter_types_and_marks_variadic_parameters() {
        let mut method = method("dispatch");
        method.parameters = vec![
            parameter("mode", "std::uint8_t", false),
            parameter("args", "vehicle.Payload", true),
        ];

        assert_eq!(
            method_key(&method),
            "dispatch(uint8_t, vehicle::Payload...)"
        );
    }

    #[test]
    fn method_parameter_formatting_includes_count_names_types_and_variadic_marker() {
        let parameters = vec![
            parameter("mode", "int", false),
            parameter("args", "IngArg&&", true),
        ];

        assert_eq!(
            format_parameter_count(&parameters),
            "2 parameters (mode: int, args: IngArg&&...)"
        );
    }

    #[test]
    fn template_parameter_formatting_includes_pack_and_nested_parameters() {
        let template_parameters = Some(vec![
            TemplateParameter::Type {
                name: "T".to_string(),
                is_pack: true,
            },
            TemplateParameter::Template {
                name: "Factory".to_string(),
                parameters: vec![TemplateParameter::NonType {
                    name: "Size".to_string(),
                    value_type: "int".to_string(),
                    is_pack: false,
                }],
                is_pack: false,
            },
        ]);

        assert_eq!(
            render_template_parameters(&template_parameters),
            "[\"T...\", \"template <int Size> Factory\"]"
        );
    }

    #[test]
    fn relationship_helpers_normalize_namespace_separators() {
        let relationship = relationship(
            "vehicle.Engine",
            "vehicle.Manufacturer",
            RelationType::Composition,
        );

        assert_eq!(
            relationship_key(&relationship),
            "vehicle::Engine -> vehicle::Manufacturer"
        );
        assert_eq!(
            relationship_display_name(&relationship),
            "vehicle::Engine -> Composition -> vehicle::Manufacturer"
        );
    }

    #[test]
    fn type_name_normalization_ignores_pointer_and_reference_spacing() {
        assert_eq!(normalize_type_name("std::uint8_t *"), "uint8_t*");
        assert_eq!(
            normalize_type_name("vehicle.Payload &"),
            "vehicle::Payload&"
        );
    }

    #[test]
    fn type_name_normalization_strips_leading_global_namespace() {
        assert_eq!(normalize_type_name("::std::uint8_t"), "uint8_t");
        assert_eq!(
            normalize_type_name(" ::vehicle::Payload "),
            "vehicle::Payload"
        );
    }

    #[test]
    fn class_implementation_validation_passes_when_design_class_and_methods_exist() {
        let design = index(vec![entity("Sample", vec!["run"])]);
        let implementation = index(vec![entity("sample", vec!["run", "helper"])]);

        let result = validate_class_design_implementation(&design, &implementation);

        assert!(
            result.is_empty(),
            "Expected pass, got: {:?}",
            result.failures
        );
    }

    #[test]
    fn class_implementation_validation_reports_missing_class() {
        let design = index(vec![entity("Sample", vec!["run"])]);
        let implementation = index(vec![]);

        let result = validate_class_design_implementation(&design, &implementation);

        assert!(result
            .failures
            .iter()
            .any(|message| message.contains("Missing implementation class")));
    }

    #[test]
    fn class_implementation_validation_reports_missing_method() {
        let design = index(vec![entity("Sample", vec!["run"])]);
        let implementation = index(vec![entity("Sample", vec!["init"])]);

        let result = validate_class_design_implementation(&design, &implementation);

        assert!(result
            .failures
            .iter()
            .any(|message| message.contains("Missing implementation method")));
    }

    #[test]
    fn class_implementation_validation_ignores_std_prefix_for_method_return_type() {
        let mut design_entity = entity("Sample", vec![]);
        design_entity.methods = vec![method_with_return_type("GetNumber", "uint8_t")];
        let mut implementation_entity = entity("Sample", vec![]);
        implementation_entity.methods = vec![method_with_return_type("GetNumber", "std::uint8_t")];

        let design = index(vec![design_entity]);
        let implementation = index(vec![implementation_entity]);

        let result = validate_class_design_implementation(&design, &implementation);

        assert!(
            result.is_empty(),
            "Expected pass, got: {:?}",
            result.failures
        );
    }

    #[test]
    fn class_implementation_validation_reports_variable_mismatch() {
        let mut design_entity = entity("Sample", vec![]);
        design_entity.variables = vec![variable("value_", "uint8_t")];
        let mut implementation_entity = entity("Sample", vec![]);
        implementation_entity.variables = vec![variable("value_", "uint16_t")];

        let design = index(vec![design_entity]);
        let implementation = index(vec![implementation_entity]);

        let result = validate_class_design_implementation(&design, &implementation);

        assert!(result.failures.iter().any(|message| {
            message.contains("Implementation class data differs")
                && message.contains("variable \"value_\" data_type")
        }));
    }

    #[test]
    fn class_implementation_validation_reports_parameter_mismatch_for_unique_same_name_method() {
        let mut design_entity = entity("Sample", vec![]);
        design_entity.methods = vec![method_with_parameter_types("stop", &["int", "int"])];
        let mut implementation_entity = entity("Sample", vec![]);
        implementation_entity.methods =
            vec![method_with_parameter_types("stop", &["int", "double"])];

        let design = index(vec![design_entity]);
        let implementation = index(vec![implementation_entity]);

        let result = validate_class_design_implementation(&design, &implementation);

        assert!(result.failures.iter().any(|message| {
            message.contains("method \"stop\" parameter type")
                && message.contains("int")
                && message.contains("double")
        }));
        assert!(result
            .failures
            .iter()
            .all(|message| !message.contains("Missing implementation method")));
    }

    #[test]
    fn class_implementation_validation_normalizes_namespace_separator_for_relationships() {
        let mut design_entity = entity("vehicle.Engine", vec![]);
        design_entity.relationships = vec![relationship(
            "vehicle.Engine",
            "vehicle.Manufacturer",
            RelationType::Composition,
        )];
        let mut implementation_entity = entity("vehicle::Engine", vec![]);
        implementation_entity.relationships = vec![relationship(
            "vehicle::Engine",
            "vehicle::Manufacturer",
            RelationType::Composition,
        )];

        let design = index(vec![design_entity]);
        let implementation = index(vec![implementation_entity]);

        let result = validate_class_design_implementation(&design, &implementation);

        assert!(
            result.is_empty(),
            "Expected pass, got: {:?}",
            result.failures
        );
    }

    #[test]
    fn class_implementation_validation_matches_exact_overloaded_method_signature() {
        let mut design_entity = entity("Sample", vec![]);
        design_entity.methods = vec![method_with_parameter_types("stop", &["int", "int"])];
        let mut implementation_entity = entity("Sample", vec![]);
        implementation_entity.methods = vec![
            method_with_parameter_types("stop", &["int", "double"]),
            method_with_parameter_types("stop", &["int", "int"]),
        ];

        let design = index(vec![design_entity]);
        let implementation = index(vec![implementation_entity]);

        let result = validate_class_design_implementation(&design, &implementation);

        assert!(
            result.is_empty(),
            "Expected pass, got: {:?}",
            result.failures
        );
    }

    #[test]
    fn class_implementation_validation_does_not_guess_between_overloaded_methods() {
        let mut design_entity = entity("Sample", vec![]);
        design_entity.methods = vec![method_with_parameter_types("stop", &["int", "int"])];
        let mut implementation_entity = entity("Sample", vec![]);
        implementation_entity.methods = vec![
            method_with_parameter_types("stop", &["int", "double"]),
            method_with_parameter_types("stop", &["int", "int", "bool"]),
        ];

        let design = index(vec![design_entity]);
        let implementation = index(vec![implementation_entity]);

        let result = validate_class_design_implementation(&design, &implementation);

        assert!(result.failures.iter().any(|message| {
            message.contains("Missing implementation method") && message.contains("stop(int, int)")
        }));
        assert!(result
            .failures
            .iter()
            .all(|message| !message.contains("parameter type")
                && !message.contains("parameter_count")));
    }

    #[test]
    fn class_implementation_validation_keeps_failures_and_diagnostics_separate() {
        enable_trace_diagnostics();

        let mut design_entity = entity("Sample", vec![]);
        design_entity.variables = vec![variable("value_", "uint8_t")];
        let mut implementation_entity = entity("Sample", vec![]);
        implementation_entity.variables = vec![variable("value_", "uint16_t")];

        let design = index(vec![design_entity]);
        let implementation = index(vec![implementation_entity]);

        let result = validate_class_design_implementation(&design, &implementation);
        let diagnostics = result.diagnostics.render();

        assert!(result
            .failures
            .iter()
            .any(|message| message.contains("Implementation class data differs")));
        assert!(diagnostics.contains("Comparing design entity"));
        assert!(!diagnostics.contains("Implementation class data differs"));
    }
}
