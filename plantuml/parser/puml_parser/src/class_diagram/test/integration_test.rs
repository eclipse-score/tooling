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

use class_parser::{ClassError, ClassUmlFile, PumlClassParser};
use parser_core::{BaseParseError, DiagramParser};
use puml_utils::LogLevel;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use test_framework::{
    run_case, DefaultExpectationChecker, DiagramProcessor, ExpectationChecker, Expected,
};

struct ClassExpectationChecker;

fn omit_unchecked_expressions(actual: &mut JsonValue, expected: &JsonValue) {
    match (actual, expected) {
        (JsonValue::Object(actual), JsonValue::Object(expected)) => {
            if expected.contains_key("raw_text") && !expected.contains_key("expression") {
                actual.remove("expression");
            }
            for (key, value) in actual.iter_mut() {
                if let Some(expected) = expected.get(key) {
                    omit_unchecked_expressions(value, expected);
                }
            }
        }
        (JsonValue::Array(actual), JsonValue::Array(expected)) => {
            for (actual, expected) in actual.iter_mut().zip(expected) {
                omit_unchecked_expressions(actual, expected);
            }
        }
        _ => {}
    }
}

impl ExpectationChecker<ClassError, ClassUmlFile> for ClassExpectationChecker {
    fn check_ok(&self, actual: &ClassUmlFile, expected: &Expected<ClassUmlFile>) {
        if let Expected::Ast(expected) = expected {
            let mut actual = serde_json::to_value(actual).unwrap();
            let expected = serde_json::to_value(expected).unwrap();
            omit_unchecked_expressions(&mut actual, &expected);
            <DefaultExpectationChecker as ExpectationChecker<ClassError, JsonValue>>::check_ok(
                &DefaultExpectationChecker,
                &actual,
                &Expected::Ast(expected),
            );
        } else {
            <DefaultExpectationChecker as ExpectationChecker<ClassError, ClassUmlFile>>::check_ok(
                &DefaultExpectationChecker,
                actual,
                expected,
            );
        }
    }

    fn check_err(&self, error: &ClassError, expected: &YamlValue, base_dir: &Path) {
        <DefaultExpectationChecker as ExpectationChecker<ClassError, ClassUmlFile>>::check_err(
            &DefaultExpectationChecker,
            error,
            expected,
            base_dir,
        );
    }
}

#[test]
fn test_only_omitted_type_expressions_are_ignored() {
    let actual = serde_json::json!({
        "type": {"raw_text": "int", "expression": {"Builtin": "int"}},
        "name": "entry",
    });
    let expected = serde_json::json!({"type": {"raw_text": "int"}, "name": "entry"});
    let mut unchecked = actual.clone();
    omit_unchecked_expressions(&mut unchecked, &expected);
    assert_eq!(unchecked, expected);

    for expected in [
        serde_json::json!({
            "type": {"raw_text": "int", "expression": {"Builtin": "void"}},
            "name": "entry",
        }),
        serde_json::json!({"type": {"raw_text": "void"}, "name": "entry"}),
        serde_json::json!({"type": {"raw_text": "int"}, "name": "other"}),
    ] {
        let mut checked = actual.clone();
        omit_unchecked_expressions(&mut checked, &expected);
        assert_ne!(checked, expected);
    }
}

struct ClassRunner;
impl DiagramProcessor for ClassRunner {
    type Output = ClassUmlFile;
    type Error = ClassError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, ClassUmlFile>, ClassError> {
        let mut results: HashMap<Rc<PathBuf>, ClassUmlFile> = HashMap::new();
        let mut parser = PumlClassParser;

        for puml_path in files {
            let uml_content = fs::read_to_string(&**puml_path).map_err(|e| {
                ClassError::Base(BaseParseError::IoError {
                    path: puml_path.as_ref().to_path_buf(),
                    error: Box::new(e),
                })
            })?;

            let class_ast = parser.parse_file(puml_path, &uml_content, LogLevel::Error)?;

            results.insert(Rc::clone(puml_path), class_ast);
        }

        Ok(results)
    }
}

// Test entry
fn run_class_diagram_parser_case(case_name: &str) {
    run_case(
        "puml_parser/tests/class_diagram",
        case_name,
        ClassRunner,
        ClassExpectationChecker,
    );
}

// --------- test for include ---------
#[test]
fn test_alias_class() {
    run_class_diagram_parser_case("alias_class");
}

#[test]
fn test_alias_package() {
    run_class_diagram_parser_case("alias_package");
}

#[test]
fn test_attr_method() {
    run_class_diagram_parser_case("attr_method");
}

#[test]
fn test_conversion_operators() {
    run_class_diagram_parser_case("conversion_operators");
}

#[test]
fn test_class_body_separators() {
    run_class_diagram_parser_case("class_body_separators");
}

#[test]
fn test_class_merge() {
    run_class_diagram_parser_case("class_merge");
}

#[test]
fn test_constexpr_attribute() {
    run_class_diagram_parser_case("constexpr_attribute");
}

#[test]
fn test_color() {
    run_class_diagram_parser_case("color");
}

#[test]
fn test_cpp_style() {
    run_class_diagram_parser_case("cpp_style");
}

#[test]
fn test_friend_method() {
    run_class_diagram_parser_case("friend_method");
}

#[test]
fn test_pointer_trailing_const() {
    run_class_diagram_parser_case("pointer_trailing_const");
}

#[test]
fn test_named_const_void_pointer_const() {
    run_class_diagram_parser_case("named_const_void_pointer_const");
}

#[test]
fn test_function_pointer() {
    run_class_diagram_parser_case("function_pointer");
}

#[test]
fn test_ignored_visibility_statements() {
    run_class_diagram_parser_case("ignored_visibility_statements");
}

#[test]
fn test_free_function_entity() {
    run_class_diagram_parser_case("free_function_entity");
}

#[test]
fn test_keyword_prefixed_relationship() {
    run_class_diagram_parser_case("keyword_prefixed_relationship");
}

#[test]
fn test_method_modifier_placement() {
    run_class_diagram_parser_case("method_modifier_placement");
}

#[test]
fn test_mock_method_variants_ignored() {
    run_class_diagram_parser_case("mock_method_variants_ignored");
}

#[test]
fn test_mock_method_interleaved_members_survive() {
    run_class_diagram_parser_case("mock_method_interleaved_members_survive");
}

#[test]
fn test_multiline_note() {
    run_class_diagram_parser_case("multiline_note");
}

#[test]
fn test_namespace_multiline_note() {
    run_class_diagram_parser_case("namespace_multiline_note");
}

#[test]
fn test_package_multiline_note() {
    run_class_diagram_parser_case("package_multiline_note");
}

#[test]
fn test_together_multiline_note() {
    run_class_diagram_parser_case("together_multiline_note");
}

#[test]
fn test_qualified_method_name() {
    run_class_diagram_parser_case("qualified_method_name");
}

#[test]
fn test_varargs_method() {
    run_class_diagram_parser_case("varargs_method");
}

#[test]
fn test_ctrl_instruct() {
    run_class_diagram_parser_case("ctrl_instruct");
}

#[test]
fn test_empty() {
    run_class_diagram_parser_case("empty");
}

#[test]
fn test_enum() {
    run_class_diagram_parser_case("enum");
}

#[test]
fn test_interface() {
    run_class_diagram_parser_case("interface");
}

#[test]
fn test_implements() {
    run_class_diagram_parser_case("implements");
}

#[test]
fn test_namespace_1() {
    run_class_diagram_parser_case("namespace_1");
}

#[test]
fn test_namespace_2() {
    run_class_diagram_parser_case("namespace_2");
}

#[test]
fn test_namespace_3() {
    run_class_diagram_parser_case("namespace_3");
}

#[test]
fn test_negative_pkg_comp() {
    run_class_diagram_parser_case("negative_pkg_comp");
}

#[test]
fn test_one_class() {
    run_class_diagram_parser_case("one_class");
}

#[test]
fn test_only_attribute() {
    run_class_diagram_parser_case("only_attribute");
}

#[test]
fn test_only_method() {
    run_class_diagram_parser_case("only_method");
}

#[test]
fn test_package() {
    run_class_diagram_parser_case("package");
}

#[test]
fn test_param() {
    run_class_diagram_parser_case("param");
}

#[test]
fn test_parameter_directions() {
    run_class_diagram_parser_case("parameter_directions");
}

#[test]
fn test_parameter_multiplicities() {
    run_class_diagram_parser_case("parameter_multiplicities");
}

#[test]
fn test_return_multiplicities() {
    run_class_diagram_parser_case("return_multiplicities");
}

#[test]
fn test_param_templete() {
    run_class_diagram_parser_case("param_templete");
}

#[test]
fn test_empty_template_args() {
    run_class_diagram_parser_case("empty_template_args");
}

#[test]
fn test_relationship_arrows() {
    run_class_diagram_parser_case("relationship_arrows");
}

#[test]
fn test_relationship_inheritance() {
    run_class_diagram_parser_case("relationship_inheritance");
}

#[test]
fn test_relationship_mix_inher() {
    run_class_diagram_parser_case("relationship_mix_inher");
}

#[test]
fn test_relationship_normal() {
    run_class_diagram_parser_case("relationship_normal");
}

#[test]
fn test_relationship_qualified_id() {
    run_class_diagram_parser_case("relationship_qualified_id");
}

#[test]
fn test_stereotype_definition() {
    run_class_diagram_parser_case("stereotype_definition");
}

#[test]
fn test_stereotype_relationship() {
    run_class_diagram_parser_case("stereotype_relationship");
}

#[test]
fn test_struct() {
    run_class_diagram_parser_case("struct");
}

#[test]
fn test_full_features() {
    run_class_diagram_parser_case("full_features");
}
