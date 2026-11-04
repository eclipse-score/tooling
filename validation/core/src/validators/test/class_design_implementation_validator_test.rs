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

use super::*;
use crate::models::ClassDiagramInputs;
use class_diagram::{
    ClassDiagram, EntityType, FreeFunctionDecl, FunctionArgument, MemberVariable, Method,
    MethodModifier, RelationType, Relationship, SimpleEntity, Visibility,
};
use source_location::SourceLocation;
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
        stereotypes: Vec::new(),
        entity_type: EntityType::Class,
        template_kind: ClassTemplateKind::NonTemplate,
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
        free_functions: Vec::new(),
    }];
    ClassEntityIndex::build_diagram_index(&diagrams, &mut ValidationResult::default())
}

fn index_with_free_functions(free_functions: Vec<FreeFunctionDecl>) -> ClassEntityIndex {
    let diagrams: ClassDiagramInputs = vec![ClassDiagram {
        name: "unit".to_string(),
        entities: Vec::new(),
        free_functions,
    }];
    ClassEntityIndex::build_diagram_index(&diagrams, &mut ValidationResult::default())
}

fn free_function(name: &str, parameter_types: &[&str]) -> FreeFunctionDecl {
    FreeFunctionDecl {
        name: name.to_string(),
        enclosing_namespace_id: None,
        return_type: Some("void".to_string()),
        parameters: parameter_types
            .iter()
            .map(|parameter_type| parameter("", parameter_type, false))
            .collect(),
        template_parameters: None,
        source_location: SourceLocation::new("test.puml", 1),
    }
}

#[test]
fn specialization_base_id_strips_only_the_class_template_arguments() {
    assert_eq!(
        specialization_base_id(
            "amp::Outer<Policy>::move_only_function<ReturnType (ArgTypes...), Capacity>"
        ),
        "amp::Outer<Policy>::move_only_function"
    );
    assert_eq!(
        specialization_base_id("amp::move_only_function"),
        "amp::move_only_function"
    );
    assert_eq!(
        specialization_base_id("amp::Outer<Policy>::move_only_function"),
        "amp::Outer<Policy>::move_only_function"
    );
}

#[test]
fn trailing_template_arguments_start_finds_only_the_final_argument_list() {
    assert_eq!(trailing_template_arguments_start("amp::Box<int>"), Some(8));
    assert_eq!(
        trailing_template_arguments_start("amp::Outer<Policy>::Box<int>"),
        Some("amp::Outer<Policy>::Box".len())
    );
    assert_eq!(trailing_template_arguments_start("amp::Box"), None);
    assert_eq!(
        trailing_template_arguments_start("amp::Outer<Policy>::Box"),
        None
    );
}

#[test]
fn specialization_matching_requires_the_complete_qualified_base_id() {
    assert!(specialization_matches_design(
        "amp::move_only_function<ReturnType (ArgTypes...), Capacity, Alignment>",
        "amp::move_only_function"
    ));
    assert!(!specialization_matches_design(
        "amp::move_only_function_extra<ReturnType (ArgTypes...), Capacity, Alignment>",
        "amp::move_only_function"
    ));
    assert!(!specialization_matches_design(
        "other::move_only_function<ReturnType (ArgTypes...), Capacity, Alignment>",
        "amp::move_only_function"
    ));
}

#[test]
fn qualified_name_normalization_preserves_id_values_but_equates_separators() {
    assert_eq!(
        normalize_qualified_name("amp.move_only_function"),
        normalize_qualified_name("amp::move_only_function")
    );
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

    method.modifiers = vec![MethodModifier::Const, MethodModifier::Noexcept];
    assert_eq!(
        method_key(&method),
        "dispatch(uint8_t, vehicle::Payload...) const"
    );
}

#[test]
fn free_function_key_normalizes_qualified_name_and_parameters() {
    let function = FreeFunctionDecl {
        name: "Dispatch".to_string(),
        enclosing_namespace_id: Some("vehicle.core".to_string()),
        return_type: Some("void".to_string()),
        parameters: vec![
            parameter("mode", "std::uint8_t", false),
            parameter("args", "vehicle.Payload", true),
        ],
        template_parameters: None,
        source_location: SourceLocation::new("test.puml", 1),
    };

    assert_eq!(
        free_function_key(&function),
        "vehicle::core::Dispatch(uint8_t, vehicle::Payload...)"
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
fn template_parameter_matching_allows_trailing_declarations_and_omitted_names() {
    let ty = |name: &str, is_pack| TemplateParameter::Type {
        name: name.to_string(),
        is_pack,
    };
    let value = |name: &str, value_type: &str, is_pack| TemplateParameter::NonType {
        name: name.to_string(),
        value_type: value_type.to_string(),
        is_pack,
    };
    let nested = |name: &str, inner| TemplateParameter::Template {
        name: name.to_string(),
        parameters: vec![inner],
        is_pack: false,
    };
    let assert_match = |design, implementation, expected| {
        assert_eq!(
            template_parameters_match(&design, &implementation),
            expected,
            "design: {design:?}, implementation: {implementation:?}",
        );
    };

    // Only nonempty declaration lists allow trailing omission; names stay positional.
    for (design, implementation, expected) in [
        (None, None, true),
        (None, Some(vec![ty("T", false)]), false),
        (Some(vec![]), None, false),
        (Some(vec![]), Some(vec![]), true),
        (Some(vec![]), Some(vec![ty("T", false)]), false),
        (
            Some(vec![ty("T", false)]),
            Some(vec![ty("T", false), ty("U", false)]),
            true,
        ),
        (
            Some(vec![ty("T", false)]),
            Some(vec![ty("U", false), ty("T", false)]),
            false,
        ),
        (Some(vec![ty("", false)]), Some(vec![]), false),
        (
            Some(vec![ty("T", false), ty("", false)]),
            Some(vec![ty("U", false), ty("", false)]),
            false,
        ),
        (
            Some(vec![ty("T", false), ty("", false)]),
            Some(vec![ty("", false), ty("", false)]),
            false,
        ),
        (
            Some(vec![ty("T", false), ty("", false)]),
            Some(vec![ty("", false), ty("T", false)]),
            false,
        ),
        (
            Some(vec![ty("T", false), ty("", false)]),
            Some(vec![ty("T", false)]),
            false,
        ),
        (
            Some(vec![ty("", false), value("", "int", false)]),
            Some(vec![value("N", "int", false), ty("T", false)]),
            false,
        ),
    ] {
        assert_match(design, implementation, expected);
    }

    // Each parameter rule is checked alone and after a matching prefix parameter.
    // An empty name is unnamed; is_pack is explicit in both constructors.
    for (design, implementation, expected) in [
        // Type names: omission is allowed only on the design side.
        (ty("", false), ty("", false), true),
        (ty("", false), ty("T", false), true),
        (ty("T", false), ty("T", false), true),
        (ty("T", false), ty("U", false), false),
        (ty("T", false), ty("", false), false),
        // Packs and parameter kinds remain constrained even without a name.
        (ty("", true), ty("Ts", true), true),
        (ty("", false), ty("Ts", true), false),
        (ty("", true), ty("T", false), false),
        (ty("", false), value("N", "int", false), false),
        // Non-type parameters compare normalized types, names and pack flags.
        (value("", "int", false), value("N", "int", false), true),
        (value("", "int", false), value("N", "long", false), false),
        (
            value("", "unsigned  long", false),
            value("N", "unsigned long", false),
            true,
        ),
        (
            value("N", "unsigned\t long   *", false),
            value("N", "unsigned long*", false),
            true,
        ),
        (
            value("N", "const\t Widget   &", false),
            value("N", "const Widget&", false),
            true,
        ),
        (
            value("N", "size_t", false),
            value("N", " ::std::size_t \t", false),
            true,
        ),
        (
            value("", "vehicle.Payload*", false),
            value("P", " vehicle::Payload * ", false),
            true,
        ),
        (
            value("N", "size_t", false),
            value("M", "std::size_t", false),
            false,
        ),
        (
            value("", "size_t", true),
            value("Ns", "std::size_t", false),
            false,
        ),
        (
            value("", "size_t", false),
            value("Ns", "std::size_t", true),
            false,
        ),
    ] {
        assert_match(
            Some(vec![design.clone()]),
            Some(vec![implementation.clone()]),
            expected,
        );
        // Trailing omission must not weaken any explicitly written entry.
        assert_match(
            Some(vec![design.clone()]),
            Some(vec![implementation.clone(), ty("Extra", false)]),
            expected,
        );
        assert_match(
            Some(vec![ty("T", false), design]),
            Some(vec![ty("T", false), implementation]),
            expected,
        );
    }

    // Template-template parameters apply the same constraints recursively.
    for (design, implementation, expected) in [
        // Template-template parameters allow omitted outer and inner design names.
        (nested("", ty("", false)), nested("C", ty("U", false)), true),
        // An explicit outer template-template parameter name must match.
        (
            nested("C", ty("", false)),
            nested("D", ty("U", false)),
            false,
        ),
        // An explicit nested parameter name must also match recursively.
        (
            nested("C", ty("T", false)),
            nested("C", ty("U", false)),
            false,
        ),
        // Omitting nested names does not allow a different nested parameter kind.
        (
            nested("", ty("", false)),
            nested("C", value("N", "int", false)),
            false,
        ),
        // Value-type normalization also applies recursively.
        (
            nested("", nested("", value("", "size_t", false))),
            nested("C", nested("D", value("N", " std::size_t ", false))),
            true,
        ),
    ] {
        assert_match(Some(vec![design]), Some(vec![implementation]), expected);
    }
}

#[test]
fn template_parameter_formatting_handles_kinds_packs_and_lists() {
    let assert_declaration = |parameter: &TemplateParameter, expected: &str| {
        assert_eq!(
            render_template_parameter_content(parameter),
            expected,
            "parameter: {parameter:?}"
        );
        let nested = TemplateParameter::Template {
            name: "Factory".to_string(),
            parameters: vec![parameter.clone()],
            is_pack: false,
        };
        assert_eq!(
            render_template_parameters(&Some(vec![nested])),
            format!("[\"template <{expected}> typename Factory\"]"),
            "nested parameter: {parameter:?}"
        );
    };

    // Type parameters use declarations when nested and shorthand at the top level.
    for (name, is_pack, declaration, shorthand) in [
        ("", false, "typename", ""),
        ("T", false, "typename T", "T"),
        ("", true, "typename...", "..."),
        ("Ts", true, "typename... Ts", "Ts..."),
    ] {
        let parameter = TemplateParameter::Type {
            name: name.to_string(),
            is_pack,
        };
        assert_declaration(&parameter, declaration);
        assert_eq!(
            render_template_parameter(&parameter),
            format!("\"{shorthand}\""),
            "parameter: {parameter:?}"
        );
    }

    // Non-type parameters trim outer whitespace without normalizing type spelling.
    for (value_type, name, is_pack, expected) in [
        (" \tint\t ", "", false, "int"),
        (" \tint\t ", "N", false, "int N"),
        (" \tint\t ", "", true, "int..."),
        (" \tint\t ", "Ns", true, "int... Ns"),
        (" \t::std::size_t\t ", "Ns", true, "::std::size_t... Ns"),
        (" unsigned  long ", "N", false, "unsigned  long N"),
    ] {
        let parameter = TemplateParameter::NonType {
            name: name.to_string(),
            value_type: value_type.to_string(),
            is_pack,
        };
        assert_declaration(&parameter, expected);
        assert_eq!(
            render_template_parameter(&parameter),
            format!("\"{expected}\""),
            "parameter: {parameter:?}"
        );
    }

    // Template-template parameters use declarations at every nesting level.
    for (name, is_pack, expected) in [
        ("", false, "template <typename> typename"),
        ("C", false, "template <typename> typename C"),
        ("", true, "template <typename> typename..."),
        ("Cs", true, "template <typename> typename... Cs"),
    ] {
        let parameter = TemplateParameter::Template {
            name: name.to_string(),
            parameters: vec![TemplateParameter::Type {
                name: String::new(),
                is_pack: false,
            }],
            is_pack,
        };
        assert_declaration(&parameter, expected);
        assert_eq!(
            render_template_parameter(&parameter),
            format!("\"{expected}\""),
            "parameter: {parameter:?}"
        );
    }

    // Mixed lists preserve order and combine all three parameter kinds.
    let template_parameters = Some(vec![
        TemplateParameter::Type {
            name: "T".to_string(),
            is_pack: true,
        },
        TemplateParameter::NonType {
            name: "Capacity".to_string(),
            value_type: "unsigned long".to_string(),
            is_pack: false,
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
        "[\"T...\", \"unsigned long Capacity\", \"template <int Size> typename Factory\"]"
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
fn non_type_whitespace_normalization_collapses_whitespace() {
    for (type_name, expected) in [
        ("unsigned  long", "unsigned long"),
        (" \tunsigned\t  long  ", "unsigned long"),
        ("unsigned\t long   *", "unsigned long*"),
        ("const\t Widget   &", "const Widget&"),
    ] {
        assert_eq!(
            normalize_non_type_template_parameter_type(type_name),
            expected,
            "type_name: {type_name:?}",
        );
    }
}

#[test]
fn non_type_whitespace_normalization_preserves_literal_behavior() {
    for type_name in [
        "Tag<\"a  b\">",
        "Tag<' '>",
        "Tag<R\"delim(a  b)delim\">",
        "Tag<\"a * b\">",
    ] {
        assert_eq!(
            normalize_non_type_template_parameter_type(type_name),
            // Literal-bearing expressions retain their exact recorded spelling.
            type_name,
        );
    }
    assert_ne!(
        normalize_non_type_template_parameter_type("Tag<\"a  b\">"),
        normalize_non_type_template_parameter_type("Tag<\"a b\">"),
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

    assert!(result.failures.iter().any(|message| message.contains(
        "[Class] Class \"Sample\" from the unit design not found in the C++ class implementation."
    )));
}

#[test]
fn class_implementation_validation_reports_missing_method() {
    let design = index(vec![entity("Sample", vec!["run"])]);
    let implementation = index(vec![entity("Sample", vec!["init"])]);

    let result = validate_class_design_implementation(&design, &implementation);

    assert!(result
        .failures
        .iter()
        .any(|message| message.contains("[Member] Method \"run()\" from entity \"Sample\" in the unit design not found in the C++ class implementation.")));
}

#[test]
fn primary_template_is_filtered_only_with_matching_partial_specializations() {
    for (primary_parameters_match, include_partials, expected_failures) in
        [(false, true, 0), (true, true, 1), (false, false, 2)]
    {
        let mut design_entity = entity("amp.move_only_function", vec!["empty"]);
        design_entity.template_parameters = Some(vec![TemplateParameter::Type {
            name: "ReturnType".to_string(),
            is_pack: false,
        }]);
        let primary = SimpleEntity {
            template_kind: ClassTemplateKind::Primary,
            template_parameters: Some(vec![TemplateParameter::Type {
                name: if primary_parameters_match {
                    "ReturnType"
                } else {
                    "Signature"
                }
                .to_string(),
                is_pack: false,
            }]),
            ..entity("amp::move_only_function", vec![])
        };
        let mut entities = vec![primary];
        if include_partials {
            entities.extend([
                SimpleEntity {
                    template_kind: ClassTemplateKind::PartialSpecialization,
                    ..entity(
                        "amp::move_only_function<R(Args...), Capacity, Alignment>",
                        vec!["empty"],
                    )
                },
                SimpleEntity {
                    template_kind: ClassTemplateKind::PartialSpecialization,
                    ..entity(
                        "amp::move_only_function<R (*)(Args...), Capacity, Alignment>",
                        vec!["empty"],
                    )
                },
            ]);
        }

        let result =
            validate_class_design_implementation(&index(vec![design_entity]), &index(entities));

        assert_eq!(
            result.failures.len(),
            expected_failures,
            "{:?}",
            result.failures,
        );
        if expected_failures > 0 {
            assert!(result
                .failures
                .iter()
                .any(|message| { message.contains("Method \"empty()\"") }));
        }
        assert_eq!(
            result
                .failures
                .iter()
                .any(|message| { message.contains("template_parameters") }),
            !primary_parameters_match && !include_partials,
        );
    }
}

#[test]
fn specializations_match_base_id_and_skip_class_template_parameters() {
    let mut design_entity = entity("demo.Box", vec!["run"]);
    design_entity.template_parameters = Some(vec![TemplateParameter::Type {
        name: "T".to_string(),
        is_pack: false,
    }]);
    let design = index(vec![design_entity]);
    for (template_kind, id) in [
        (ClassTemplateKind::PartialSpecialization, "demo::Box<T*>"),
        (ClassTemplateKind::ExplicitSpecialization, "demo::Box<int>"),
    ] {
        let implementation = index(vec![SimpleEntity {
            template_kind,
            ..entity(id, vec!["run"])
        }]);
        let result = validate_class_design_implementation(&design, &implementation);
        assert!(
            result.is_empty(),
            "{template_kind:?}: {:?}",
            result.failures
        );
    }
}

#[test]
fn class_implementation_validation_reports_full_signature_for_missing_free_function_overloads() {
    let design = index_with_free_functions(vec![
        free_function("Select", &["int"]),
        free_function("Select", &["int", "int"]),
    ]);
    let implementation = index_with_free_functions(Vec::new());

    let result = validate_class_design_implementation(&design, &implementation);

    assert!(result
        .failures
        .iter()
        .any(|message| message.contains("Free function \"Select(int)\"")));
    assert!(result
        .failures
        .iter()
        .any(|message| message.contains("Free function \"Select(int, int)\"")));
}

#[test]
fn class_implementation_validation_normalizes_nested_namespace_in_free_function_mismatch() {
    let mut design_function = free_function("Dispatch", &["int"]);
    design_function.enclosing_namespace_id = Some("vehicle.core".to_string());
    let mut implementation_function = free_function("Dispatch", &["int"]);
    implementation_function.enclosing_namespace_id = Some("vehicle::core".to_string());
    implementation_function.return_type = Some("bool".to_string());

    let design = index_with_free_functions(vec![design_function]);
    let implementation = index_with_free_functions(vec![implementation_function]);

    let result = validate_class_design_implementation(&design, &implementation);

    assert!(result.failures.iter().any(|message| {
        message.contains("free function \"vehicle::core::Dispatch\"")
            && !message.contains("vehicle.core::Dispatch")
    }));
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
        message.contains("[Implementation] Field variable \"value_\" data type on entity \"Sample\" differs between the unit design and the C++ class implementation.")
            && message.contains("variable \"value_\" data type")
    }));
}

#[test]
fn class_implementation_validation_reports_parameter_mismatch_for_unique_same_name_method() {
    let mut design_entity = entity("Sample", vec![]);
    design_entity.methods = vec![method_with_parameter_types("stop", &["int", "int"])];
    let mut implementation_entity = entity("Sample", vec![]);
    implementation_entity.methods = vec![method_with_parameter_types("stop", &["int", "double"])];

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
        .all(|message| !message.contains("[member] method")));
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
fn relationship_candidates_match_each_design_type_independently() {
    use RelationType::{Aggregation, Dependency};

    for design_types in [
        vec![Aggregation],
        vec![Dependency],
        vec![Aggregation, Dependency],
    ] {
        for (implementation_types, missing_dependency) in [
            (vec![Aggregation, Dependency], false),
            (vec![Dependency, Aggregation], false),
            (vec![Aggregation], true),
        ] {
            let mut design_entity = entity("vehicle.Engine", vec![]);
            design_entity.relationships = design_types
                .iter()
                .map(|kind| relationship("vehicle.Engine", "vehicle.Part", *kind))
                .collect();
            let mut implementation_entity = entity("vehicle::Engine", vec![]);
            implementation_entity.relationships = implementation_types
                .iter()
                .map(|kind| relationship("vehicle::Engine", "vehicle::Part", *kind))
                .collect();

            let result = validate_class_design_implementation(
                &index(vec![design_entity]),
                &index(vec![implementation_entity]),
            );
            let expected_failures =
                usize::from(missing_dependency && design_types.contains(&Dependency));
            assert_eq!(
                result.failures.len(),
                expected_failures,
                "design {design_types:?}, implementation {implementation_types:?}: {:?}",
                result.failures,
            );
            if expected_failures == 1 {
                let failure = &result.failures[0];
                assert!(failure.contains("[Implementation] Field \"relationship type\""));
                assert!(failure.contains("vehicle::Engine -> Dependency -> vehicle::Part"));
                assert!(!failure.contains("not found"));
            }
        }
    }
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

    let result = validate_class_design_implementation(
        &index(vec![design_entity]),
        &index(vec![implementation_entity]),
    );

    assert!(result.is_empty(), "{:?}", result.failures);
}

#[test]
fn class_implementation_validation_matches_unique_overloaded_method_prefix() {
    let mut design_entity = entity("Sample", vec![]);
    design_entity.methods = vec![method_with_parameter_types("stop", &["int", "int"])];
    let mut implementation_entity = entity("Sample", vec![]);
    implementation_entity.methods = vec![
        method_with_parameter_types("stop", &["int", "double"]),
        method_with_parameter_types("stop", &["int", "int", "bool"]),
    ];

    let result = validate_class_design_implementation(
        &index(vec![design_entity]),
        &index(vec![implementation_entity]),
    );

    // The old exact-arity missing-signature expectation is now a unique prefix match.
    assert!(result.is_empty(), "{:?}", result.failures);
}

#[test]
fn method_candidates_distinguish_const_overloads_in_either_order() {
    for parameter_types in [vec![], vec!["int"]] {
        for same_return_type in [true, false] {
            let mut nonconst = method_with_parameter_types("Check", &parameter_types);
            nonconst.return_type = Some("int&".to_string());
            let mut const_method = nonconst.clone();
            const_method.modifiers = vec![MethodModifier::Const];
            if !same_return_type {
                const_method.return_type = Some("const int&".to_string());
            }
            for mut implementations in [
                vec![nonconst.clone(), const_method.clone()],
                vec![const_method.clone(), nonconst.clone()],
            ] {
                // Exercise omitted trailing parameters as well as zero-parameter overloads.
                if !parameter_types.is_empty() {
                    for implementation in &mut implementations {
                        implementation.parameters.push(parameter("", "bool", false));
                    }
                }
                for designs in [
                    vec![nonconst.clone()],
                    vec![const_method.clone()],
                    vec![nonconst.clone(), const_method.clone()],
                    vec![const_method.clone(), nonconst.clone()],
                ] {
                    let mut design_entity = entity("Sample", vec![]);
                    design_entity.methods = designs;
                    let mut implementation_entity = entity("Sample", vec![]);
                    implementation_entity.methods = implementations.clone();
                    let result = validate_class_design_implementation(
                        &index(vec![design_entity]),
                        &index(vec![implementation_entity]),
                    );
                    assert!(result.is_empty(), "{:?}", result.failures);
                }
            }
        }
    }
}

#[test]
fn method_const_mismatch_reports_modifiers_in_both_directions() {
    for design_is_const in [false, true] {
        let mut design_method = method("Check");
        let mut implementation_method = method("Check");
        if design_is_const {
            design_method.modifiers.push(MethodModifier::Const);
        } else {
            implementation_method.modifiers.push(MethodModifier::Const);
        }
        let mut design_entity = entity("Sample", vec![]);
        design_entity.methods = vec![design_method];
        let mut implementation_entity = entity("Sample", vec![]);
        implementation_entity.methods = vec![implementation_method];
        let result = validate_class_design_implementation(
            &index(vec![design_entity]),
            &index(vec![implementation_entity]),
        );

        assert_eq!(result.failures.len(), 1, "{:?}", result.failures);
        assert!(result.failures[0].contains("method \"Check\" modifiers"));
        assert!(result.failures[0].contains("[Const]"));
        assert!(result.failures[0].contains("[]"));
    }
}

#[test]
fn method_const_mismatch_does_not_select_an_unrelated_overload() {
    for design_is_const in [false, true] {
        let mut design_method = method("Check");
        let mut implementation_method = method("Check");
        if design_is_const {
            design_method.modifiers.push(MethodModifier::Const);
        } else {
            implementation_method.modifiers.push(MethodModifier::Const);
        }
        let mut design_entity = entity("Sample", vec![]);
        design_entity.methods = vec![design_method.clone()];
        let mut implementation_entity = entity("Sample", vec![]);
        implementation_entity.methods = vec![
            implementation_method,
            method_with_parameter_types("Check", &["double"]),
        ];
        let result = validate_class_design_implementation(
            &index(vec![design_entity]),
            &index(vec![implementation_entity]),
        );

        assert_eq!(result.failures.len(), 1, "{:?}", result.failures);
        assert!(result.failures[0].contains(&format!("Method \"{}\"", method_key(&design_method))));
        assert!(result.failures[0].contains("not found"));
    }
}

/// Run the same overload matrix through both public validation paths.
fn validate_callable_pair(
    design: Vec<Method>,
    implementation: Vec<Method>,
) -> [ValidationResult; 2] {
    let as_free = |methods: &[Method]| {
        methods
            .iter()
            .map(|method| FreeFunctionDecl {
                name: method.name.clone(),
                enclosing_namespace_id: Some("vehicle::core".to_string()),
                return_type: method.return_type.clone(),
                parameters: method.parameters.clone(),
                template_parameters: method.template_parameters.clone(),
                source_location: method.source_location.clone(),
            })
            .collect()
    };
    let free_result = validate_class_design_implementation(
        &index_with_free_functions(as_free(&design)),
        &index_with_free_functions(as_free(&implementation)),
    );
    let mut design_entity = entity("Sample", vec![]);
    design_entity.methods = design;
    let mut implementation_entity = entity("Sample", vec![]);
    implementation_entity.methods = implementation;
    [
        validate_class_design_implementation(
            &index(vec![design_entity]),
            &index(vec![implementation_entity]),
        ),
        free_result,
    ]
}

#[test]
fn callable_group_matrix_is_directional_and_injective() {
    // Expected: 0 = success, 1 = ambiguity, 2 = contention, 3 = legacy mismatch.
    let cases = vec![
        (vec![vec!["int"]], vec![vec!["int", "bool"]], 0),
        (vec![vec!["int"]], vec![vec!["int"], vec!["int", "bool"]], 1),
        (
            vec![vec!["int"], vec!["int", "bool"]],
            vec![vec!["int"], vec!["int", "bool"]],
            0,
        ),
        (
            vec![vec!["Box<int>"], vec!["Box<int>", "bool"]],
            vec![vec!["Box<int>", "bool"]],
            2,
        ),
        (
            vec![vec!["int"]],
            vec![vec!["int", "bool"], vec!["double"]],
            0,
        ),
        (
            vec![vec!["int", "int"]],
            vec![vec!["int", "double"], vec!["int", "int"]],
            0,
        ),
        (
            vec![vec!["int", "int"]],
            vec![vec!["int", "double"], vec!["int", "int", "bool"]],
            0,
        ),
        (vec![vec![]], vec![vec![], vec!["int"]], 0),
        (vec![vec![]], vec![vec!["int"]], 3),
        (vec![vec!["int", "bool"]], vec![vec!["int"]], 3),
        (
            vec![vec!["Outer<Box<int>>"]],
            vec![vec!["Outer<Box<int>, Alloc>", "bool"]],
            3,
        ),
        (
            vec![vec!["Outer<Box<int>, Alloc>"]],
            vec![vec!["Outer<Box<int>, Alloc>", "bool"]],
            0,
        ),
        (
            vec![vec!["Outer<Box<int>>"]],
            vec![vec!["Outer<Box<int, Policy>, Alloc>", "bool"]],
            3,
        ),
        (
            vec![vec!["Outer<Box<int, Policy>, Alloc>"]],
            vec![vec!["Outer<Box<int>>"]],
            3,
        ),
        (vec![vec!["Box<>"]], vec![vec!["Box<int>"]], 3),
    ];
    for (design_types, implementation_types, expected) in cases {
        let design = design_types
            .iter()
            .map(|types| method_with_parameter_types("run", types))
            .collect();
        let implementation = implementation_types
            .iter()
            .map(|types| method_with_parameter_types("run", types))
            .collect();
        for result in validate_callable_pair(design, implementation) {
            assert_eq!(
                result.is_empty(),
                expected == 0,
                "{design_types:?}, {implementation_types:?}: {:?}",
                result.failures
            );
            if expected != 0 {
                assert_eq!(result.failures.len(), 1, "{:?}", result.failures);
                let message = &result.failures[0];
                match expected {
                    1 => assert!(
                        message
                            .to_lowercase()
                            .contains("ambiguous overload matching"),
                        "{message}"
                    ),
                    2 => assert!(
                        message
                            .to_lowercase()
                            .contains("no injective overload matching"),
                        "{message}"
                    ),
                    _ => assert!(
                        message.contains("parameter count") || message.contains("parameter type"),
                        "{message}"
                    ),
                }
            }
        }
    }
}

#[test]
fn documented_template_callable_cases_apply_to_methods_and_free_functions() {
    let type_parameter = |name: &str| TemplateParameter::Type {
        name: name.to_string(),
        is_pack: false,
    };
    let mut design = method_with_parameter_types("f", &["int"]);
    design.template_parameters = Some(vec![type_parameter("T")]);

    let mut trailing = design.clone();
    trailing.template_parameters = Some(vec![type_parameter("T"), type_parameter("U")]);
    let mut different_kind = design.clone();
    different_kind.template_parameters = Some(vec![TemplateParameter::NonType {
        name: "N".to_string(),
        value_type: "int".to_string(),
        is_pack: false,
    }]);
    let ordinary = method_with_parameter_types("f", &["int"]);
    let mut dependent_design = method_with_parameter_types("f", &["T"]);
    dependent_design.template_parameters = Some(vec![type_parameter("T")]);
    let mut renamed = method_with_parameter_types("f", &["U"]);
    renamed.template_parameters = Some(vec![type_parameter("U")]);

    let cases = vec![
        (
            "equal templates",
            design.clone(),
            vec![design.clone()],
            None,
        ),
        (
            "trailing template omission",
            design.clone(),
            vec![trailing],
            None,
        ),
        (
            "template kind mismatch",
            design.clone(),
            vec![different_kind],
            Some("template parameters"),
        ),
        (
            "missing template declaration",
            design.clone(),
            vec![ordinary.clone()],
            Some("template parameters"),
        ),
        (
            "template metadata cannot disambiguate",
            design.clone(),
            vec![ordinary, design],
            Some("ambiguous overload matching"),
        ),
        (
            "no template renaming",
            dependent_design,
            vec![renamed],
            Some("parameter type"),
        ),
    ];
    for (case, design, implementations, expected_failure) in cases {
        for result in validate_callable_pair(vec![design], implementations) {
            if let Some(expected) = expected_failure {
                let expected_count = if case == "no template renaming" { 2 } else { 1 };
                assert_eq!(
                    result.failures.len(),
                    expected_count,
                    "{case}: {:?}",
                    result.failures
                );
                assert!(
                    result
                        .failures
                        .iter()
                        .any(|message| message.to_lowercase().contains(expected)),
                    "{case}: {:?}",
                    result.failures
                );
                if case == "no template renaming" {
                    assert!(result
                        .failures
                        .iter()
                        .any(|message| message.contains("template parameters")));
                }
            } else {
                assert!(result.is_empty(), "{case}: {:?}", result.failures);
            }
        }
    }
}

#[test]
fn callable_fields_are_checked_after_selection_never_for_disambiguation() {
    let mut design = method_with_parameter_types("run", &["int"]);
    design.parameters[0].name = "expected".to_string();
    design.return_type = Some("Box<int>".to_string());
    design.template_parameters = Some(vec![TemplateParameter::Type {
        name: "T".to_string(),
        is_pack: false,
    }]);
    let mut implementation = method_with_parameter_types("run", &["int", "bool"]);
    implementation.parameters[0].name = "actual".to_string();
    implementation.return_type = Some("bool".to_string());
    implementation.visibility = Visibility::Private;
    implementation.modifiers = vec![MethodModifier::Static];
    for (kind, result) in validate_callable_pair(vec![design.clone()], vec![implementation.clone()])
        .into_iter()
        .enumerate()
    {
        for field in ["parameter name", "return type", "template parameters"] {
            assert!(
                result
                    .failures
                    .iter()
                    .any(|message| message.contains(field)),
                "{:?}",
                result.failures
            );
        }
        if kind == 0 {
            for field in ["visibility", "modifiers"] {
                assert!(
                    result
                        .failures
                        .iter()
                        .any(|message| message.contains(field)),
                    "{:?}",
                    result.failures
                );
            }
        }
        assert!(!result
            .failures
            .iter()
            .any(|message| message.contains("parameter count")));
    }
    // Even a completely field-consistent exact candidate cannot take priority.
    for result in validate_callable_pair(vec![design.clone()], vec![design, implementation]) {
        assert_eq!(result.failures.len(), 1, "{:?}", result.failures);
        assert!(result.failures[0]
            .to_lowercase()
            .contains("ambiguous overload matching"));
    }
}

#[test]
fn parameter_flags_are_candidate_constraints_for_both_callable_kinds() {
    for flag in [0, 1] {
        let design = method_with_parameter_types("run", &["Args"]);
        let mut mismatch = design.clone();
        if flag == 0 {
            mismatch.parameters[0].is_variadic = true;
        } else {
            mismatch.parameters[0].is_pack_expansion = true;
        }
        for result in validate_callable_pair(vec![design.clone()], vec![mismatch.clone()]) {
            assert_eq!(result.failures.len(), 1, "{:?}", result.failures);
            assert!(result.failures[0].contains(if flag == 0 {
                "parameter variadic"
            } else {
                "parameter pack expansion"
            }));
        }
        for result in validate_callable_pair(vec![design.clone()], vec![mismatch.clone(), design]) {
            assert!(result.is_empty(), "{:?}", result.failures);
        }
    }
}

#[test]
fn complete_types_are_required_in_all_seven_type_mismatch_scenarios() {
    let assert_single_type_mismatch = |result: &ValidationResult, field: &str| {
        assert_eq!(result.failures.len(), 1, "{field}: {:?}", result.failures);
        let message = &result.failures[0];
        assert!(
            message.contains("[Implementation] Field") && message.contains(field),
            "{field}: {message}"
        );
        assert!(!message.contains("parameter count"), "{field}: {message}");
    };
    let complete_type = "complete::Map<int, float, char, long>";
    // Alias and variable entry points: omitted actual arguments and a changed argument.
    for (is_alias, design_type) in [
        (true, "complete::Map<int, float>"),
        (false, "complete::Map<int, float, char>"),
        (false, "complete::Map<int, float, short, long>"),
    ] {
        let mut design = entity("Consumer", vec![]);
        let mut implementation = entity("Consumer", vec![]);
        let field = if is_alias {
            let alias = |data_type: &str| TypeAlias {
                alias: "Storage".to_string(),
                original_type: data_type.to_string(),
                source_location: SourceLocation::new("design.puml", 7),
            };
            design.type_aliases = vec![alias(design_type)];
            implementation.type_aliases = vec![alias(complete_type)];
            "type alias \"Storage\" original type"
        } else {
            design.variables = vec![variable("values", design_type)];
            implementation.variables = vec![variable("values", complete_type)];
            "variable \"values\" data type"
        };
        let result = validate_class_design_implementation(
            &index(vec![design]),
            &index(vec![implementation]),
        );
        assert_single_type_mismatch(&result, field);
    }
    // Return and parameter types in both callable paths stay complete even
    // when trailing function parameters are omitted in the design.
    for field in ["return type", "parameter type"] {
        let mut design = method_with_parameter_types("run", &["int"]);
        let mut implementation = method_with_parameter_types("run", &["int", "bool"]);
        if field == "return type" {
            design.return_type = Some("const complete::Map<int, float>&".to_string());
            implementation.return_type = Some(format!("const {complete_type}&"));
        } else {
            design.parameters[0].param_type =
                Some("const complete::Map<int, float, char>&".to_string());
            implementation.parameters[0].param_type = Some(format!("const {complete_type}&"));
        }
        for result in validate_callable_pair(vec![design], vec![implementation]) {
            assert_single_type_mismatch(&result, field);
        }
    }
}

#[test]
fn optional_types_require_matching_presence() {
    assert!(optional_types_match(&None, &None));
    assert!(!optional_types_match(&None, &Some("int".to_string())));
    assert!(!optional_types_match(&Some("int".to_string()), &None));
}

#[test]
fn literal_bearing_types_are_compared_raw_without_namespace_rewriting() {
    for (design, implementation) in [
        ("Tag<\"std::a.b\">", "Tag<\"a::b\">"),
        ("Tag<'.'>", "Tag<'::'>"),
        ("Tag<R\"x(std::a.b * &)x\">", "Tag<R\"x(a::b*&)x\">"),
        ("Tag<' '>", "Tag<'\\t'>"),
        ("Tag<\"a * b\">", "Tag<\"a*b\">"),
        ("std::Tag<\"value\">", "Tag<\"value\">"),
        (" Tag<\"value\">", "Tag<\"value\">"),
        ("Tag<\"a  b\">", "Tag<\"a b\">"),
    ] {
        assert!(comparison_types_match(design, design));
        assert!(!comparison_types_match(design, implementation));
        assert_eq!(normalize_non_type_template_parameter_type(design), design);
        assert_ne!(
            normalize_non_type_template_parameter_type(design),
            normalize_non_type_template_parameter_type(implementation)
        );
        assert_eq!(render_optional_type(&Some(design.to_string())), design);
        assert_eq!(render_parameter_type(&Some(design.to_string())), design);
    }
}

#[test]
fn literal_types_are_preserved_in_signatures_and_validation_diagnostics() {
    // Exercise every entry point with one representative raw literal;
    // spelling variations are covered by the comparison table above.
    let design = "Tag<R\"x(std::a.b * &)x\">";
    let implementation = "Tag<R\"x(a::b*&)x\">";
    let design_method = method_with_parameter_types("run", &[design]);
    assert_eq!(parameter_signature(&design_method.parameters), design);
    assert_eq!(method_key(&design_method), format!("run({design})"));
    assert_eq!(
        free_function_key(&free_function("run", &[design])),
        format!("run({design})")
    );
    for result in validate_callable_pair(
        vec![design_method.clone()],
        vec![method_with_parameter_types("run", &[implementation])],
    ) {
        let message = result
            .failures
            .iter()
            .find(|message| message.contains("parameter type"))
            .expect("literal type mismatch must be diagnosed");
        assert!(message.contains(design), "{message}");
        assert!(message.contains(implementation), "{message}");
    }
    // Overloaded missing-signature diagnostics use the same literal-safe key.
    for result in validate_callable_pair(
        vec![design_method],
        vec![
            method_with_parameter_types("run", &[implementation]),
            method_with_parameter_types("run", &["int"]),
        ],
    ) {
        assert_eq!(result.failures.len(), 1, "{:?}", result.failures);
        assert!(
            result.failures[0].contains(&format!("run({design})")),
            "{:?}",
            result.failures
        );
    }
    for result in validate_callable_pair(
        vec![method_with_return_type("get", design)],
        vec![method_with_return_type("get", implementation)],
    ) {
        assert_eq!(result.failures.len(), 1, "{:?}", result.failures);
        let message = &result.failures[0];
        assert!(message.contains("return type"), "{message}");
        assert!(message.contains(design), "{message}");
        assert!(message.contains(implementation), "{message}");
    }
    let mut design_entity = entity("Sample", vec![]);
    design_entity.type_aliases = vec![TypeAlias {
        alias: "Value".to_string(),
        original_type: design.to_string(),
        source_location: SourceLocation::new("design.puml", 7),
    }];
    design_entity.variables = vec![variable("value", design)];
    let mut implementation_entity = design_entity.clone();
    implementation_entity.type_aliases[0].original_type = implementation.to_string();
    implementation_entity.variables[0].data_type = Some(implementation.to_string());
    let result = validate_class_design_implementation(
        &index(vec![design_entity]),
        &index(vec![implementation_entity]),
    );
    assert_eq!(result.failures.len(), 2, "{:?}", result.failures);
    for field in ["original type", "data type"] {
        let message = result
            .failures
            .iter()
            .find(|message| message.contains(field))
            .expect("alias and variable literal mismatches must be diagnosed");
        assert!(message.contains(design), "{message}");
        assert!(message.contains(implementation), "{message}");
    }
}

#[test]
fn template_declaration_value_types_remain_strict_not_partial_arguments() {
    let value = |value_type: &str| TemplateParameter::NonType {
        name: "N".to_string(),
        value_type: value_type.to_string(),
        is_pack: false,
    };
    assert!(!template_parameters_match(
        &Some(vec![value("Box<int>")]),
        &Some(vec![value("Box<int, Policy>")])
    ));
    let nested = |value_type| TemplateParameter::Template {
        name: "C".to_string(),
        parameters: vec![value(value_type)],
        is_pack: false,
    };
    assert!(!template_parameters_match(
        &Some(vec![nested("Box<int>")]),
        &Some(vec![nested("Box<int, Policy>")])
    ));
}

#[test]
fn zero_candidate_rows_keep_missing_signature_diagnostics_for_overloaded_names() {
    for result in validate_callable_pair(
        vec![method_with_parameter_types("run", &["bool"])],
        vec![
            method_with_parameter_types("run", &["int"]),
            method_with_parameter_types("run", &["double"]),
        ],
    ) {
        assert_eq!(result.failures.len(), 1, "{:?}", result.failures);
        assert!(result.failures[0].contains("run(bool)"));
        assert!(result.failures[0].contains("not found"));
        assert!(!result.failures[0].contains("overload matching"));
    }
}

#[test]
fn group_failures_list_all_declarations_and_locations() {
    let mut design = method_with_parameter_types("run", &["int"]);
    design.source_location = SourceLocation::new("design.puml", 7);
    let mut exact = design.clone();
    exact.source_location = SourceLocation::new("implementation.hpp", 11);
    let mut longer = exact.clone();
    longer.parameters.push(parameter("extra", "bool", false));
    longer.source_location = SourceLocation::new("implementation.hpp", 19);
    for result in validate_callable_pair(vec![design], vec![exact, longer]) {
        let message = &result.failures[0];
        for expected in [
            "run(int)",
            "extra: bool",
            "function parameters",
            "additional overload design",
        ] {
            assert!(message.contains(expected), "{expected}: {message}");
        }
        for (label, value) in [
            ("Design source file", "\"design.puml\""),
            ("Design source line", "7"),
            ("Implementation source file 1", "\"implementation.hpp\""),
            ("Implementation source line 1", "11"),
            ("Implementation source file 2", "\"implementation.hpp\""),
            ("Implementation source line 2", "19"),
        ] {
            assert!(
                message.contains(&format!("  {label:28} : {value}")),
                "{message}"
            );
        }
        for label in [
            "Reason",
            "Design value",
            "Implementation value 1",
            "Implementation value 2",
            "Fix",
        ] {
            assert!(message.contains(&format!("\n  {label:28} : ")), "{message}");
        }
        assert!(!message.contains("\n\n"), "{message}");
        assert!(!message.contains(" -> "), "{message}");
        assert!(!message.contains("FunctionArgument"), "{message}");
        assert!(!message.contains("Some("), "{message}");
    }
}

#[test]
fn generic_group_ambiguity_and_contention_have_distinct_guidance() {
    let first = method_with_parameter_types("run", &["int"]);
    let mut second = first.clone();
    second.return_type = Some("bool".to_string());
    second.source_location = SourceLocation::new("second.puml", 8);
    let mut third = first.clone();
    third.parameters.push(parameter("force", "bool", false));
    for result in validate_callable_pair(
        vec![first.clone(), second.clone()],
        vec![first.clone(), second.clone(), third],
    ) {
        let message = &result.failures[0];
        assert!(
            message.contains("Multiple one-to-one matchings cover all design declarations."),
            "{message}"
        );
        assert!(
            message.contains(&format!("  {:28} : ", "Design value 1")),
            "{message}"
        );
        assert!(
            message.contains(&format!("  {:28} : ", "Implementation value 3")),
            "{message}"
        );
        assert!(!message.contains("Both implementation"), "{message}");
        assert!(
            message.contains("additional overload design declarations"),
            "{message}"
        );
    }
    for result in validate_callable_pair(vec![first.clone(), second], vec![first]) {
        let message = &result.failures[0];
        assert!(
            message
                .contains("Design declarations compete for the same implementation declarations."),
            "{message}"
        );
        assert!(
            message.contains("No one-to-one matching can cover all design declarations."),
            "{message}"
        );
        assert!(
            message.contains(
                "Provide distinct matching implementation overloads or correct the design"
            ),
            "{message}"
        );
        assert!(
            !message.contains("additional overload design declarations"),
            "{message}"
        );
    }
}

#[test]
fn simple_callable_fields_preserve_raw_parameters_and_sort_without_deduplication() {
    let raw_type = "Tag<R\"tag(a  b)tag\", ' ', \"c  d\">";
    let parameters = vec![
        parameter("payload", raw_type, false),
        FunctionArgument {
            name: "args".to_string(),
            param_type: Some("Args".to_string()),
            is_pack_expansion: true,
            ..Default::default()
        },
        FunctionArgument {
            name: "unknown".to_string(),
            ..Default::default()
        },
        FunctionArgument {
            is_variadic: true,
            ..Default::default()
        },
    ];
    let declaration = format_function_declaration("vehicle::Sample::run", &parameters);
    assert_eq!(
        declaration,
        format!(
            "vehicle::Sample::run(payload: {raw_type}, args: Args..., unknown: <unknown>, ...)"
        )
    );
    let unknown = SourceLocation::new("", 0);
    let early = SourceLocation::new("a.hpp", 2);
    let late = SourceLocation::new("a.hpp", 10);
    let other_file = SourceLocation::new("b.hpp", 1);
    let message = format_overload_matching_failure(
        "method vehicle::Sample::run",
        &MatchingOutcome::NoSolution,
        [(declaration.clone(), &unknown)].into_iter(),
        [
            ("z()".to_string(), &early),
            (declaration.clone(), &other_file),
            (declaration.clone(), &late),
            (declaration.clone(), &early),
            (declaration.clone(), &early),
        ]
        .into_iter(),
    );
    for (label, value) in [
        ("Design source file", "\"<unknown>\""),
        ("Design source line", "<unknown>"),
        ("Implementation source line 1", "2"),
        ("Implementation source line 2", "2"),
        ("Implementation source line 3", "10"),
        ("Implementation source file 4", "\"b.hpp\""),
        ("Implementation value 5", "z()"),
    ] {
        assert!(
            message.contains(&format!("\n  {label:28} : {value}")),
            "{message}"
        );
    }
    assert_eq!(message.matches(&declaration).count(), 5, "{message}");
    assert!(!message.contains("\n\n"), "{message}");
}

#[test]
fn callable_duplicates_ignore_locations_on_both_sides() {
    let design = method_with_parameter_types("run", &["int"]);
    let exact = design.clone();
    let mut distinct = exact.clone();
    distinct.source_location = SourceLocation::new("other.hpp", 9);
    for (design, implementation) in [
        (vec![design.clone()], vec![exact.clone(), distinct.clone()]),
        (vec![design.clone(), distinct.clone()], vec![exact.clone()]),
        (
            vec![design.clone(), design, distinct.clone()],
            vec![distinct, exact.clone(), exact],
        ),
    ] {
        for result in validate_callable_pair(design, implementation) {
            assert!(result.is_empty(), "{:?}", result.failures);
        }
    }
}

#[test]
fn conflicting_callable_attributes_remain_distinct_on_both_sides() {
    let exact = method_with_parameter_types("run", &["int"]);
    for field in ["return", "name", "template", "visibility", "static"] {
        let mut conflict = exact.clone();
        match field {
            "return" => conflict.return_type = Some("bool".to_string()),
            "name" => conflict.parameters[0].name = "different".to_string(),
            "template" => {
                conflict.template_parameters = Some(vec![TemplateParameter::Type {
                    name: "T".to_string(),
                    is_pack: false,
                }])
            }
            "visibility" => conflict.visibility = Visibility::Private,
            _ => conflict.modifiers = vec![MethodModifier::Static],
        }
        let methods_only = matches!(field, "visibility" | "static");
        for (kind, result) in
            validate_callable_pair(vec![exact.clone()], vec![exact.clone(), conflict.clone()])
                .into_iter()
                .enumerate()
        {
            if methods_only && kind == 1 {
                assert!(result.is_empty(), "{:?}", result.failures);
            } else {
                assert_eq!(result.failures.len(), 1, "{field}: {:?}", result.failures);
                assert!(result.failures[0]
                    .to_lowercase()
                    .contains("ambiguous overload matching"));
            }
        }
        for (kind, result) in
            validate_callable_pair(vec![exact.clone(), conflict], vec![exact.clone()])
                .into_iter()
                .enumerate()
        {
            if methods_only && kind == 1 {
                assert!(result.is_empty(), "{:?}", result.failures);
            } else {
                assert_eq!(result.failures.len(), 1, "{field}: {:?}", result.failures);
                assert!(result.failures[0]
                    .to_lowercase()
                    .contains("no injective overload matching"));
            }
        }
    }
}

#[test]
fn empty_candidate_rows_do_not_hide_contention_or_ambiguity() {
    for (design_types, implementation_types, reason) in [
        (
            vec![vec!["bool"], vec!["Box<int>"], vec!["Box<int>", "int"]],
            vec![vec!["Box<int>", "int"], vec!["double"]],
            "no injective overload matching",
        ),
        (
            vec![vec!["bool"], vec!["int"]],
            vec![vec!["int"], vec!["int", "bool"]],
            "ambiguous overload matching",
        ),
    ] {
        for reverse in [false, true] {
            let mut design: Vec<_> = design_types
                .iter()
                .map(|types| method_with_parameter_types("run", types))
                .collect();
            let mut implementation: Vec<_> = implementation_types
                .iter()
                .map(|types| method_with_parameter_types("run", types))
                .collect();
            if reverse {
                design.reverse();
                implementation.reverse();
            }
            for result in validate_callable_pair(design, implementation) {
                assert_eq!(result.failures.len(), 2, "{:?}", result.failures);
                assert!(result
                    .failures
                    .iter()
                    .any(|message| message.contains("not found")));
                assert!(result
                    .failures
                    .iter()
                    .any(|message| message.to_lowercase().contains(reason)));
            }
        }
    }
}

#[test]
fn free_function_groups_use_fully_qualified_names() {
    let mut design = free_function("run", &["int"]);
    let renamed = free_function("other", &["int"]);
    assert!(!free_functions_match(&design, &renamed));
    design.enclosing_namespace_id = Some("vehicle.core".to_string());
    let mut selected = design.clone();
    selected.enclosing_namespace_id = Some("vehicle::core".to_string());
    selected.parameters.push(parameter("", "bool", false));
    let mut other = selected.clone();
    other.enclosing_namespace_id = Some("other".to_string());
    let result = validate_class_design_implementation(
        &index_with_free_functions(vec![design]),
        &index_with_free_functions(vec![selected, other]),
    );
    assert!(result.is_empty(), "{:?}", result.failures);
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
        .any(|message| message.contains("[Implementation] Field")));
    assert!(diagnostics.contains("Comparing design entity"));
    assert!(!diagnostics.contains("[Implementation] Field"));
}
