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
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use class_diagram::ClassDiagram;
use class_parser::PumlClassParser;
use class_resolver::{ClassPumlResolverError, ClassResolver};

use parser_core::DiagramParser;
use puml_utils::LogLevel;
use resolver_traits::DiagramResolver;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use test_framework::{
    run_case, DefaultExpectationChecker, DiagramProcessor, ExpectationChecker, Expected,
    ResolverGroupRunner,
};

struct ClassExpectationChecker;

fn omit_unchecked_type_expressions(actual: &mut JsonValue, expected: &JsonValue) {
    match (actual, expected) {
        (JsonValue::Object(actual), JsonValue::Object(expected)) => {
            if expected.contains_key("raw_text") && !expected.contains_key("expression") {
                actual.remove("expression");
            }
            for (key, value) in actual.iter_mut() {
                if let Some(expected) = expected.get(key) {
                    omit_unchecked_type_expressions(value, expected);
                }
            }
        }
        (JsonValue::Array(actual), JsonValue::Array(expected)) => {
            for (actual, expected) in actual.iter_mut().zip(expected) {
                omit_unchecked_type_expressions(actual, expected);
            }
        }
        _ => {}
    }
}

impl ExpectationChecker<ClassPumlResolverError, ClassDiagram> for ClassExpectationChecker {
    fn check_ok(&self, actual: &ClassDiagram, expected: &Expected<ClassDiagram>) {
        if let Expected::Ast(expected) = expected {
            let mut actual = serde_json::to_value(actual).unwrap();
            let expected = serde_json::to_value(expected).unwrap();
            omit_unchecked_type_expressions(&mut actual, &expected);
            <DefaultExpectationChecker as ExpectationChecker<ClassPumlResolverError, JsonValue>>::check_ok(
                &DefaultExpectationChecker, &actual, &Expected::Ast(expected),
            );
        } else {
            <DefaultExpectationChecker as ExpectationChecker<
                ClassPumlResolverError,
                ClassDiagram,
            >>::check_ok(&DefaultExpectationChecker, actual, expected);
        }
    }

    fn check_err(&self, error: &ClassPumlResolverError, expected: &YamlValue, base_dir: &Path) {
        <DefaultExpectationChecker as ExpectationChecker<ClassPumlResolverError, ClassDiagram>>::check_err(
            &DefaultExpectationChecker, error, expected, base_dir,
        );
    }
}

// ===== Class Resolver adapter DiagramProcessor =====
struct ClassResolverRunner;
impl DiagramProcessor for ClassResolverRunner {
    type Output = ClassDiagram;
    type Error = ClassPumlResolverError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, ClassDiagram>, ClassPumlResolverError> {
        let mut results = HashMap::new();
        let mut parser = PumlClassParser;
        let mut resolver = ClassResolver::new();

        for path in files {
            let puml_file =
                fs::read_to_string(&**path).expect("Class Resolver: Failed to read test file");
            let parsed_ast = parser
                .parse_file(path, &puml_file, LogLevel::Error)
                .expect("Class Resolver: Failed to parse test file");
            let logic_ast = resolver.resolve(&parsed_ast)?;
            results.insert(Rc::clone(path), logic_ast);
        }

        Ok(results)
    }
}

// Test entry
fn run_class_resolver_case(case_name: &str) {
    run_case(
        "integration_test/class_diagram",
        case_name,
        ClassResolverRunner,
        ClassExpectationChecker,
    );
}

fn run_class_resolver_group_case(case_name: &str) {
    run_case(
        "integration_test/class_diagram",
        case_name,
        ResolverGroupRunner::<_, ClassResolver>::new(PumlClassParser),
        ClassExpectationChecker,
    );
}

#[test]
fn test_group_implicit_parent_across_files() {
    run_class_resolver_group_case("group_implicit_parent_across_files");
}

#[test]
fn test_group_bodyless_stays_top_level() {
    run_class_resolver_group_case("group_bodyless_stays_top_level");
}

#[test]
fn test_group_kind_mismatch_stays_top_level() {
    run_class_resolver_group_case("group_kind_mismatch_stays_top_level");
}

#[test]
fn test_group_ambiguous_implicit_parent() {
    run_class_resolver_group_case("group_ambiguous_implicit_parent");
}

#[test]
fn test_group_implicit_parent_cycle() {
    run_class_resolver_group_case("group_implicit_parent_cycle");
}

#[test]
fn test_group_cross_file_relation() {
    run_class_resolver_group_case("group_cross_file_relation");
}

#[test]
fn test_group_local_wins_over_peer() {
    run_class_resolver_group_case("group_local_wins_over_peer");
}

#[test]
fn test_group_cross_file_relation_source() {
    run_class_resolver_group_case("group_cross_file_relation_source");
}

#[test]
fn test_group_ambiguous_peer_reference() {
    run_class_resolver_group_case("group_ambiguous_peer_reference");
}

#[test]
fn test_group_local_alias_name_error() {
    run_class_resolver_group_case("group_local_alias_name_error");
}

#[test]
fn test_class_positive() {
    run_class_resolver_case("class_diagram_positive");
}

#[test]
fn test_class_negative() {
    run_class_resolver_case("class_diagram_negative");
}

#[test]
fn test_cpp_members() {
    run_class_resolver_case("class_diagram_cpp_members");
}

#[test]
fn test_structured_member_types_survive_resolution() {
    let path = Rc::new(PathBuf::from("structured_types.puml"));
    let source = "@startuml
class Registry {
    + using Buffer = std::array<::app::Payload, 8U>
    + values : const app::Payload*[8U]
    + callback : void (* const)(const app::Payload&) noexcept
    + unsupported : decltype(value)
    + Store(data : std::array<app::Payload, 8U>) : const app::Payload*
}
entity Functions <<function>> {
    + Make(value : app::Payload&) : std::vector<app::Payload>
}
@enduml
";
    let parsed = PumlClassParser
        .parse_file(&path, source, LogLevel::Error)
        .unwrap();
    let resolved = ClassResolver::new().resolve(&parsed).unwrap();
    let class_parser::ClassUmlTopLevel::Type(class_parser::TypeDefinition::ClassLikeDef(class)) =
        &parsed.elements[0]
    else {
        panic!("expected a class");
    };
    let entity = &resolved.entities[0];
    let alias = &class.members.type_aliases[0].original_type;
    assert!(alias.expression.is_some());
    assert_eq!(&entity.type_aliases[0].original_type, alias);
    assert_eq!(entity.variables.len(), class.members.attributes.len());
    for (variable, attribute) in entity.variables.iter().zip(&class.members.attributes) {
        assert_eq!(variable.data_type, attribute.r#type);
    }
    let unsupported = entity.variables[2].data_type.as_ref().unwrap();
    assert_eq!(unsupported.raw_text, "decltype(value)");
    assert!(unsupported.expression.is_none());
    let method = &class.members.methods[0];
    assert_eq!(entity.methods[0].return_type, method.r#type);
    assert_eq!(
        entity.methods[0].parameters[0].param_type,
        method.params[0].param_type
    );
    let class_parser::ClassUmlTopLevel::Type(class_parser::TypeDefinition::EntityDef(functions)) =
        &parsed.elements[1]
    else {
        panic!("expected a function entity");
    };
    let function = &functions.methods[0];
    assert_eq!(resolved.free_functions.len(), 1);
    assert_eq!(resolved.free_functions[0].return_type, function.r#type);
    assert_eq!(
        resolved.free_functions[0].parameters[0].param_type,
        function.params[0].param_type
    );
}

#[test]
fn test_file_level_constructs() {
    run_class_resolver_case("class_diagram_file_level_constructs");
}

#[test]
fn test_modifiers() {
    run_class_resolver_case("class_diagram_modifiers");
}

#[test]
fn test_note_alias_relationship() {
    run_class_resolver_case("note_alias_relationship");
}

#[test]
fn test_object_syntax() {
    run_class_resolver_case("class_diagram_object_syntax");
}

#[test]
fn test_relationship_variants() {
    run_class_resolver_case("class_diagram_relationship_variants");
}

#[test]
fn test_syntax_coverage() {
    run_class_resolver_case("class_diagram_syntax_coverage");
}

#[test]
fn test_enum_value_sequence() {
    run_class_resolver_case("class_diagram_enum_value_sequence");
}

#[test]
fn test_method_template_pack() {
    run_class_resolver_case("method_template_pack");
}

#[test]
fn test_c_variadic_method() {
    run_class_resolver_case("c_variadic_method");
}

#[test]
fn test_free_function_entity() {
    run_class_resolver_case("free_function_entity");
}

#[test]
fn test_class_template_pack() {
    run_class_resolver_case("class_template_pack");
}

#[test]
fn test_empty_template_args() {
    run_class_resolver_case("empty_template_args");
}

#[test]
fn test_name_wins_over_alias() {
    run_class_resolver_case("name_wins_over_alias");
}

#[test]
fn test_qualified_label_in_package() {
    run_class_resolver_case("qualified_label_in_package");
}

#[test]
fn test_template_label() {
    run_class_resolver_case("template_label");
}

#[test]
fn test_multiline_label() {
    run_class_resolver_case("multiline_label");
}

#[test]
fn test_reversed_alias_form() {
    run_class_resolver_case("reversed_alias_form");
}

#[test]
fn test_constructor_of_aliased_class() {
    run_class_resolver_case("constructor_of_aliased_class");
}

#[test]
fn test_invalid_prose_label() {
    run_class_resolver_case("invalid_prose_label");
}

#[test]
fn test_invalid_prose_package() {
    run_class_resolver_case("invalid_prose_package");
}

#[test]
fn test_invalid_malformed_label() {
    run_class_resolver_case("invalid_malformed_label");
}

#[test]
fn test_invalid_reference_by_label() {
    run_class_resolver_case("invalid_reference_by_label");
}

#[test]
fn test_invalid_duplicate_label() {
    run_class_resolver_case("invalid_duplicate_label");
}

#[test]
fn test_relation_leaf_of_qualified_declaration() {
    run_class_resolver_case("relation_leaf_of_qualified_declaration");
}

#[test]
fn test_qualified_declaration_in_package() {
    run_class_resolver_case("qualified_declaration_in_package");
}

#[test]
fn test_rooted_declaration_in_package() {
    run_class_resolver_case("rooted_declaration_in_package");
}

#[test]
fn test_rooted_dotted_declaration() {
    run_class_resolver_case("rooted_dotted_declaration");
}

#[test]
fn test_separator_equivalence() {
    run_class_resolver_case("separator_equivalence");
}

#[test]
fn test_qualified_container_in_package() {
    run_class_resolver_case("qualified_container_in_package");
}

#[test]
fn test_invalid_qualified_declaration_duplicate() {
    run_class_resolver_case("qualified_declaration_duplicate");
}

#[test]
fn test_package_relationship_reopened_package() {
    run_class_resolver_case("package_relationship_reopened_package");
}

#[test]
fn test_relation_nearest_scope_shadowing() {
    run_class_resolver_case("relation_nearest_scope_shadowing");
}

#[test]
fn test_extends_root_marker() {
    run_class_resolver_case("extends_root_marker");
}

#[test]
fn test_extends_root_marker_quoted_name() {
    run_class_resolver_case("extends_root_marker_quoted_name");
}

#[test]
fn test_invalid_duplicate_entity() {
    run_class_resolver_case("invalid_duplicate_entity");
}

#[test]
fn test_invalid_duplicate_alias() {
    run_class_resolver_case("invalid_duplicate_alias");
}

#[test]
fn test_invalid_duplicate_entity_separator() {
    run_class_resolver_case("invalid_duplicate_entity_separator");
}

#[test]
fn test_invalid_forward_reference() {
    run_class_resolver_case("invalid_forward_reference");
}

#[test]
fn test_invalid_ambiguous_simple_reference() {
    run_class_resolver_case("invalid_ambiguous_simple_reference");
}

#[test]
fn test_invalid_ambiguous_outer_reference() {
    run_class_resolver_case("invalid_ambiguous_outer_reference");
}

#[test]
fn test_invalid_unresolved_label_reference() {
    run_class_resolver_case("invalid_unresolved_label_reference");
}

#[test]
fn test_invalid_ambiguous_qualified_reference() {
    run_class_resolver_case("invalid_ambiguous_qualified_reference");
}

#[test]
fn test_invalid_unresolved_qualified_reference() {
    run_class_resolver_case("invalid_unresolved_qualified_reference");
}

#[test]
fn test_invalid_unresolved_nested_qualified_reference() {
    run_class_resolver_case("invalid_unresolved_nested_qualified_reference");
}

#[test]
fn test_invalid_unresolved_qualified_base() {
    run_class_resolver_case("invalid_unresolved_qualified_base");
}

#[test]
fn test_relation_colon_qualified_endpoint() {
    run_class_resolver_case("relation_colon_qualified_endpoint");
}

#[test]
fn test_relation_root_marker() {
    run_class_resolver_case("relation_root_marker");
}

#[test]
fn test_namespace_relationship() {
    run_class_resolver_case("namespace_relationship");
}

#[test]
fn test_invalid_namespace_unresolved_relationship() {
    run_class_resolver_case("invalid_namespace_unresolved_relationship");
}
