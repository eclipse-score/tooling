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
use std::path::PathBuf;
use std::rc::Rc;

use component_diagram::LogicComponent;
use component_parser::PumlComponentParser;
use component_resolver::{ComponentResolver, ComponentResolverError};
use parser_core::DiagramParser;
use puml_utils::LogLevel;
use resolver_traits::DiagramResolver;
use test_framework::{run_case, DefaultExpectationChecker, DiagramProcessor};

// ===== Component Resolver adapter DiagramProcessor =====
struct ComponentResolverRunner;
impl DiagramProcessor for ComponentResolverRunner {
    type Output = HashMap<String, LogicComponent>;
    type Error = ComponentResolverError;

    fn run(
        &self,
        files: &HashSet<Rc<PathBuf>>,
    ) -> Result<HashMap<Rc<PathBuf>, HashMap<String, LogicComponent>>, ComponentResolverError> {
        let mut results = HashMap::new();
        let mut parser = PumlComponentParser;
        let mut resolver = ComponentResolver::new();

        for path in files {
            let puml_file = fs::read_to_string(&**path).expect("Failed to read test file");
            let parsed_ast = parser
                .parse_file(path, &puml_file, LogLevel::Error)
                .expect("Failed to parse test file");
            let logic_ast = resolver.resolve(&parsed_ast)?;

            results.insert(Rc::clone(path), logic_ast);
        }
        Ok(results)
    }
}

// Test entry
fn run_component_resolver_case(case_name: &str) {
    run_case(
        "integration_test/component_diagram",
        case_name,
        ComponentResolverRunner,
        DefaultExpectationChecker,
    );
}

fn run_deployment_resolver_case(case_name: &str) {
    run_case(
        "integration_test/deployment_diagram",
        case_name,
        ComponentResolverRunner,
        DefaultExpectationChecker,
    );
}

#[test]
fn test_relation_simple_name() {
    run_component_resolver_case("relation_simple_name");
}

#[test]
fn test_relation_reverse_simple_name() {
    run_component_resolver_case("relation_reverse_simple_name");
}

#[test]
fn test_relation_fqn() {
    run_component_resolver_case("relation_fqn");
}

#[test]
fn test_relation_relative_name() {
    run_component_resolver_case("relation_relative_name");
}

#[test]
fn test_relation_simple_name_alias() {
    run_component_resolver_case("relation_simple_name_alias");
}

#[test]
fn test_relation_invalid_arrow_parsed_as_association() {
    run_component_resolver_case("relation_invalid_arrow_parsed_as_association");
}

#[test]
fn test_relation_absolute_fqn() {
    run_component_resolver_case("relation_absolute_fqn");
}

#[test]
fn test_invalid_unresolved_reference() {
    run_component_resolver_case("invalid_unresolved_reference");
}

#[test]
fn test_invalid_ambiguous_reference() {
    run_component_resolver_case("invalid_ambiguous_reference");
}

#[test]
fn test_invalid_ambiguous_reference_element_alias() {
    run_component_resolver_case("invalid_ambiguous_reference_element_alias");
}

#[test]
fn test_invalid_duplicate_component() {
    run_component_resolver_case("invalid_duplicate_component");
}

#[test]
fn test_invalid_interface_decor_between_components() {
    run_component_resolver_case("invalid_interface_decor_between_components");
}

#[test]
fn test_invalid_interface_decor_between_interfaces() {
    run_component_resolver_case("invalid_interface_decor_between_interfaces");
}

#[test]
fn test_invalid_interface_left_decorator() {
    run_component_resolver_case("invalid_interface_left_decorator");
}

#[test]
fn test_invalid_interface_binding_non_component() {
    run_component_resolver_case("invalid_interface_binding_non_component");
}

#[test]
fn test_invalid_source_endpoint_role_mismatch() {
    run_component_resolver_case("invalid_source_endpoint_role_mismatch");
}

#[test]
fn test_invalid_mixed_interface_decor() {
    run_component_resolver_case("invalid_mixed_interface_decor");
}

#[test]
fn test_port_basic() {
    run_component_resolver_case("port_basic");
}

#[test]
fn test_port_relation_lifting() {
    run_component_resolver_case("port_relation_lifting");
}

#[test]
fn test_port_two_ports() {
    run_component_resolver_case("port_two_ports");
}

#[test]
fn test_together_basic() {
    run_component_resolver_case("together_basic");
}

#[test]
fn test_arrow_lollipop() {
    run_component_resolver_case("arrow_lollipop");
}

#[test]
fn test_relation_interface_required_to_component() {
    run_component_resolver_case("relation_interface_required_to_component");
}

#[test]
fn test_relation_interface_provided_to_component() {
    run_component_resolver_case("relation_interface_provided_to_component");
}

#[test]
fn test_port_directional_interface_binding() {
    run_component_resolver_case("port_directional_interface_binding");
}

#[test]
fn test_port_alias() {
    run_component_resolver_case("port_alias");
}

#[test]
fn test_together_with_relation() {
    run_component_resolver_case("together_with_relation");
}

#[test]
fn test_port_global_name_resolution() {
    run_component_resolver_case("port_global_name_resolution");
}

#[test]
fn test_top_level_port() {
    run_component_resolver_case("top_level_port");
}

#[test]
fn test_port_deep_nesting() {
    run_component_resolver_case("port_deep_nesting");
}

#[test]
fn test_port_target_no_decor_no_mismatch() {
    run_component_resolver_case("port_target_no_decor_no_mismatch");
}

#[test]
fn test_package_seooc_interface_binding() {
    run_component_resolver_case("package_seooc_interface_binding");
}

#[test]
fn test_package_component_interface_binding() {
    run_component_resolver_case("package_component_interface_binding");
}

#[test]
fn test_invalid_package_no_stereotype_binding() {
    run_component_resolver_case("invalid_package_no_stereotype_binding");
}

#[test]
fn test_deployment_diagram() {
    run_deployment_resolver_case("deployment_diagram_it");
}

#[test]
fn test_declare_elements() {
    run_deployment_resolver_case("declare_elements");
}

#[test]
fn test_arrows_link() {
    run_deployment_resolver_case("arrows_link");
}

#[test]
fn test_nested_elements() {
    run_deployment_resolver_case("nested_elements");
}

#[test]
fn test_source_locations_are_preserved() {
    use component_diagram::SourceLocation;
    use component_parser::{CompPumlDocument, Element, Relation, Statement};
    use parser_core::common_ast::ElementIdentity;
    use parser_core::common_ast::{Arrow, ArrowLine};

    let component_location = SourceLocation::new("input.puml", 2);
    let relation_location = SourceLocation::new("input.puml", 4);

    let document = CompPumlDocument {
        name: None,
        statements: vec![
            Statement::Element(Element {
                identity: ElementIdentity {
                    name: Some("A".to_string()),
                    alias: None,
                    stereotype: None,
                    element_kind: "component".to_string(),
                    source_location: component_location.clone(),
                },
                style: None,
                statements: Vec::new(),
            }),
            Statement::Element(Element {
                identity: ElementIdentity {
                    name: Some("B".to_string()),
                    alias: None,
                    stereotype: None,
                    element_kind: "component".to_string(),
                    source_location: SourceLocation::new("input.puml", 3),
                },
                style: None,
                statements: Vec::new(),
            }),
            Statement::Relation(Relation {
                lhs: "A".to_string(),
                arrow: Arrow {
                    left: None,
                    line: ArrowLine {
                        raw: "--".to_string(),
                    },
                    middle: None,
                    right: None,
                },
                rhs: "B".to_string(),
                style: None,
                description: None,
                source_location: relation_location.clone(),
            }),
        ],
    };

    let mut resolver = ComponentResolver::new();
    let logic = resolver.resolve(&document).expect("document must resolve");

    let component = logic.get("A").expect("component A must exist");
    assert_eq!(component.source_location, component_location);

    let relation = component
        .relations
        .first()
        .expect("A must have one relation");
    assert_eq!(relation.source_location, relation_location);
}
