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

use std::collections::{BTreeMap, BTreeSet};

use source_location::SourceLocation;

use crate::models::{
    BazelArchitecture, ComponentDiagramArchitecture, ComponentRelationType, EndpointRole,
    InternalApiIndex, LogicComponentExt,
};
use crate::results::{Diagnostics, ErrorBuilder, ErrorCategory};
use crate::validators::shared::{format_quoted_list, format_source_file, format_source_line};
use crate::ValidationResult;

#[derive(Clone, Debug)]
pub(crate) struct UnitDesignEntity {
    pub id: String,
    pub entity_type: class_diagram::EntityType,
    pub method_names: Vec<String>,
    pub source_location: SourceLocation,
}

enum UnitDesignDiagnostic<'a> {
    Refinement {
        refinements: &'a [&'a UnitDesignEntity],
    },
    BoundInterface {
        api_interface: &'a crate::models::InternalApiInterface,
        matching_interfaces: &'a [&'a UnitDesignEntity],
    },
}

/// Verify that every architectural unit is refined by an eligible entity whose
/// ID is nested below the full architectural unit ID in the bound unit design.
pub fn validate_architectural_unit_class_design(
    bazel: &BazelArchitecture,
    component: &ComponentDiagramArchitecture,
    design_entities_by_unit_label: &BTreeMap<String, Vec<UnitDesignEntity>>,
    internal_api: Option<&InternalApiIndex>,
) -> ValidationResult {
    let mut result = ValidationResult::default();

    for (unit_key, architectural_unit) in &component.unit_set {
        let Some((unit_label, entities)) =
            unit_design_context(unit_key, &bazel.unit_set, design_entities_by_unit_label)
        else {
            // Bazel-component validation reports this independently.
            continue;
        };

        validate_unit(architectural_unit, unit_label, entities, &mut result);
    }

    if let Some(internal_api) = internal_api {
        validate_bound_interfaces_and_methods(
            bazel,
            component,
            internal_api,
            design_entities_by_unit_label,
            &mut result,
        );
    }

    result
}

fn unit_design_context<'a, K: Ord>(
    unit_key: &K,
    bazel_units: &'a BTreeMap<K, String>,
    design_entities_by_unit_label: &'a BTreeMap<String, Vec<UnitDesignEntity>>,
) -> Option<(&'a str, &'a [UnitDesignEntity])> {
    let unit_label = bazel_units.get(unit_key)?;
    let entities = design_entities_by_unit_label
        .get(unit_label)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    Some((unit_label, entities))
}

fn validate_bound_interfaces_and_methods(
    bazel: &BazelArchitecture,
    component: &ComponentDiagramArchitecture,
    internal_api: &InternalApiIndex,
    design_entities_by_unit_label: &BTreeMap<String, Vec<UnitDesignEntity>>,
    result: &mut ValidationResult,
) {
    let internal_interfaces: BTreeMap<&str, _> = internal_api
        .interfaces()
        .map(|interface| (interface.id.as_str(), interface))
        .collect();
    let component_interfaces: BTreeMap<&str, _> = component
        .entities
        .iter()
        .filter(|entity| entity.is_interface() && entity.parent_id.is_some())
        .map(|interface| (interface.id.as_str(), interface))
        .collect();

    for (unit_key, unit) in &component.unit_set {
        let Some((unit_label, design_entities)) =
            unit_design_context(unit_key, &bazel.unit_set, design_entities_by_unit_label)
        else {
            // The Bazel-component validator reports the missing Bazel mapping.
            continue;
        };

        let bound_interface_targets: BTreeSet<&str> = unit
            .relations
            .iter()
            .filter(|relation| {
                relation.relation_type == ComponentRelationType::InterfaceBinding
                    && relation.source_role == EndpointRole::Required
            })
            .map(|relation| relation.target.as_str())
            .collect();

        for target in bound_interface_targets {
            let Some(component_interface) = component_interfaces.get(target) else {
                continue;
            };
            let Some(api_interface) = internal_interfaces.get(target) else {
                // The component-internal-API validator reports this mismatch.
                continue;
            };

            validate_bound_unit_interface(
                &unit.id,
                unit_label,
                component_interface,
                api_interface,
                design_entities,
                result,
            );
        }
    }
}

fn validate_bound_unit_interface(
    unit_id: &str,
    unit_label: &str,
    component_interface: &crate::models::LogicComponent,
    api_interface: &crate::models::InternalApiInterface,
    design_entities: &[UnitDesignEntity],
    result: &mut ValidationResult,
) {
    let matching_interfaces: Vec<&UnitDesignEntity> = design_entities
        .iter()
        .filter(|entity| {
            entity.entity_type == class_diagram::EntityType::Interface
                && entity.id == api_interface.id
        })
        .collect();

    log_unit_design_diagnostics(
        &mut result.diagnostics,
        unit_id,
        unit_label,
        design_entities,
        UnitDesignDiagnostic::BoundInterface {
            api_interface,
            matching_interfaces: &matching_interfaces,
        },
    );

    let Some(detail_interface) = matching_interfaces.first().copied() else {
        result.add_failure(missing_interface_error(
            unit_id,
            unit_label,
            &component_interface.id,
            &component_interface.source_location,
            api_interface,
            design_entities,
        ));
        return;
    };

    // ClassEntityIndex rejects duplicate canonical IDs before entities reach
    // this validator, so at most one exact interface can match here.
    let detail_methods: BTreeSet<&str> = detail_interface
        .method_names
        .iter()
        .map(String::as_str)
        .collect();
    let missing_methods: BTreeSet<&str> = api_interface
        .method_names
        .iter()
        .map(String::as_str)
        .filter(|method| !detail_methods.contains(method))
        .collect();
    if !missing_methods.is_empty() {
        result.add_failure(missing_methods_error(
            unit_id,
            unit_label,
            &component_interface.id,
            &component_interface.source_location,
            api_interface,
            detail_interface,
            &missing_methods,
        ));
    }
}

fn missing_interface_error(
    unit_id: &str,
    unit_label: &str,
    interface_id: &str,
    component_source: &SourceLocation,
    api_interface: &crate::models::InternalApiInterface,
    design_entities: &[UnitDesignEntity],
) -> String {
    let mut error = ErrorBuilder::new(ErrorCategory::Interface)
        .title(format!(
            "Bound unit \"{unit_id}\" has no corresponding interface \"{interface_id}\" in its unit design"
        ))
        .field("component interface", format!("\"{interface_id}\""))
        .field("bound unit", format!("\"{unit_id}\""))
        .field("bazel unit", unit_label);
    error = add_interface_source_fields(error, component_source, &api_interface.source_location);
    let design_sources: BTreeSet<(String, u32)> = design_entities
        .iter()
        .map(|entity| entity.source_location.display())
        .collect();
    for (design_file, design_line) in design_sources {
        error = error
            .field("design source file", format!("\"{design_file}\""))
            .field("design source line", design_line.to_string());
    }
    error
        .fix(format!(
            "declare interface \"{}\" in the unit design for \"{unit_id}\" using the Internal API namespace",
            interface_id.rsplit('.').next().unwrap_or(interface_id)
        ))
        .build()
}

fn missing_methods_error(
    unit_id: &str,
    unit_label: &str,
    interface_id: &str,
    component_source: &SourceLocation,
    api_interface: &crate::models::InternalApiInterface,
    detail_interface: &UnitDesignEntity,
    missing_methods: &BTreeSet<&str>,
) -> String {
    let methods = format_quoted_list(missing_methods.iter().copied());
    let (design_file, design_line) = detail_interface.source_location.display();
    let mut error = ErrorBuilder::new(ErrorCategory::Method)
        .title(format!(
            "Unit design interface \"{}\" is missing Internal API method(s) {methods}",
            detail_interface.id
        ))
        .field("component interface", format!("\"{interface_id}\""))
        .field("bound unit", format!("\"{unit_id}\""))
        .field("bazel unit", unit_label)
        .field("detail interface", format!("\"{}\"", detail_interface.id));
    error = add_interface_source_fields(error, component_source, &api_interface.source_location);
    error
        .field("design source file", format!("\"{design_file}\""))
        .field("design source line", design_line.to_string())
        .fix(format!(
            "add {methods} to the detail interface, matching the Internal API method names exactly"
        ))
        .build()
}

fn add_interface_source_fields(
    error: ErrorBuilder,
    component_source: &SourceLocation,
    api_source: &SourceLocation,
) -> ErrorBuilder {
    let (component_file, component_line) = component_source.display();
    let (api_file, api_line) = api_source.display();
    error
        .field("component source file", format!("\"{component_file}\""))
        .field("component source line", component_line.to_string())
        .field("internal API source file", format!("\"{api_file}\""))
        .field("internal API source line", api_line.to_string())
}

fn validate_unit(
    architectural_unit: &crate::models::LogicComponent,
    unit_label: &str,
    entities: &[UnitDesignEntity],
    result: &mut ValidationResult,
) {
    let refinements: Vec<&UnitDesignEntity> = entities
        .iter()
        .filter(|entity| refines_unit(&entity.id, &architectural_unit.id))
        .collect();

    log_unit_design_diagnostics(
        &mut result.diagnostics,
        &architectural_unit.id,
        unit_label,
        entities,
        UnitDesignDiagnostic::Refinement {
            refinements: &refinements,
        },
    );

    if refinements.is_empty() {
        result.add_failure(missing_refinement_error(
            &architectural_unit.id,
            unit_label,
            entities,
        ));
    }
}

fn refines_unit(entity_id: &str, unit_id: &str) -> bool {
    entity_id
        .to_lowercase()
        .starts_with(&format!("{}.", unit_id.to_lowercase()))
}

fn missing_refinement_error(
    unit_id: &str,
    unit_label: &str,
    entities: &[UnitDesignEntity],
) -> String {
    let mut error = ErrorBuilder::new(ErrorCategory::Design)
        .title(format!(
            "Architectural unit \"{unit_id}\" has no refining entity in its unit design"
        ))
        .field("architectural unit", unit_id)
        .field("bazel unit", unit_label);

    if entities.is_empty() {
        error = error.field("unit design entities", "<none>");
    } else {
        for entity in entities {
            error = error
                .field("entity", format!("\"{}\"", entity.id))
                .field(
                    "design source file",
                    format!("\"{}\"", format_source_file(&entity.source_location)),
                )
                .field(
                    "design source line",
                    format_source_line(&entity.source_location),
                );
        }
    }

    error
        .fix(format!(
            "add a class, struct, interface, or abstract class whose ID is below the architectural unit ID \"{unit_id}\""
        ))
        .build()
}

fn log_unit_design_diagnostics(
    diagnostics: &mut Diagnostics,
    unit_id: &str,
    unit_label: &str,
    entities: &[UnitDesignEntity],
    context: UnitDesignDiagnostic<'_>,
) {
    diagnostics.debug(|| {
        match context {
            UnitDesignDiagnostic::Refinement { refinements } => format!(
                "Architectural unit: {}\nBazel unit: {}\nEligible unit-design entities: {}\nSelected refinements: {}",
                unit_id,
                unit_label,
                display_entities(entities),
                display_refinement_ids(refinements),
            ),
            UnitDesignDiagnostic::BoundInterface {
                api_interface,
                matching_interfaces,
            } => {
                let detail_interfaces = entities
                    .iter()
                    .filter(|entity| entity.entity_type == class_diagram::EntityType::Interface)
                    .map(|entity| {
                        format!(
                            "{} [{}]",
                            entity.id,
                            display_list(entity.method_names.iter().cloned())
                        )
                    })
                    .collect::<Vec<_>>();
                format!(
                    "Bound interface consistency:\nArchitectural unit: {}\nBazel unit: {}\nInternal API interface: {}\nRequired methods: {}\nDetail-design interfaces: {}\nExact matching interfaces: {}",
                    unit_id,
                    unit_label,
                    api_interface.id,
                    display_list(api_interface.method_names.iter().cloned()),
                    display_list(detail_interfaces),
                    display_entity_refs(matching_interfaces),
                )
            }
        }
    });
}

fn display_entities(entities: &[UnitDesignEntity]) -> String {
    display_list(entities.iter().map(display_entity))
}

fn display_entity_refs(entities: &[&UnitDesignEntity]) -> String {
    display_list(entities.iter().map(|entity| display_entity(entity)))
}

fn display_entity(entity: &UnitDesignEntity) -> String {
    let (file, line) = entity.source_location.display();
    format!(
        "{} {:?} methods [{}] ({}:{})",
        entity.id,
        entity.entity_type,
        display_list(entity.method_names.iter().cloned()),
        file,
        line,
    )
}

fn display_refinement_ids(entities: &[&UnitDesignEntity]) -> String {
    display_list(entities.iter().map(|entity| entity.id.clone()))
}

fn display_list(items: impl IntoIterator<Item = String>) -> String {
    let mut output = String::new();
    let mut is_first = true;
    for item in items {
        if !is_first {
            output.push_str(", ");
        }
        output.push_str(&item);
        is_first = false;
    }
    if is_first {
        "<none>".to_string()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        BazelEntityRef, BazelInput, BazelInputEntry, ComponentDiagramInputs, ComponentRelationType,
        ComponentType, EndpointRole, InternalApiIndex, LogicComponent, LogicRelation,
    };
    use class_diagram::{ClassDiagram, EntityType, Method, SimpleEntity, Visibility};
    use std::collections::BTreeMap;

    fn source_location() -> source_location::SourceLocation {
        source_location::SourceLocation::new("test.puml", 1)
    }

    fn component_entity(id: &str, parent_id: Option<&str>, stereotype: &str) -> LogicComponent {
        LogicComponent {
            id: id.to_string(),
            name: id.rsplit('.').next().map(str::to_string),
            alias: None,
            parent_id: parent_id.map(str::to_string),
            element_type: if stereotype == "SEooC" {
                ComponentType::Package
            } else {
                ComponentType::Component
            },
            stereotype: Some(stereotype.to_string()),
            relations: Vec::new(),
            source_location: source_location(),
        }
    }

    fn bazel_ref(label: &str, design_name: &str) -> BazelEntityRef {
        BazelEntityRef {
            label: label.to_string(),
            design_name: design_name.to_string(),
        }
    }

    fn inputs(
        class_ids: &[&str],
    ) -> (
        BazelArchitecture,
        ComponentDiagramArchitecture,
        BTreeMap<String, Vec<UnitDesignEntity>>,
    ) {
        let mut components = BTreeMap::new();
        components.insert(
            "@//:system".to_string(),
            BazelInputEntry {
                units: Vec::new(),
                components: vec![bazel_ref("@//:control", "control")],
            },
        );
        components.insert(
            "@//:control".to_string(),
            BazelInputEntry {
                units: vec![bazel_ref("@//:filter", "Filter")],
                components: Vec::new(),
            },
        );

        let mut result = ValidationResult::default();
        let bazel = BazelInput { components }.to_bazel_architecture(&mut result);
        let component = ComponentDiagramInputs {
            entities: vec![
                component_entity("system", None, "SEooC"),
                component_entity("system.control", Some("system"), "component"),
                component_entity("system.control.Filter", Some("system.control"), "unit"),
            ],
        }
        .to_diagram_architecture(&mut result);
        assert!(
            result.is_empty(),
            "fixture setup failed: {:?}",
            result.failures
        );

        let classes = BTreeMap::from([(
            "@//:filter".to_string(),
            class_ids
                .iter()
                .map(|id| UnitDesignEntity {
                    id: (*id).to_string(),
                    entity_type: class_diagram::EntityType::Class,
                    method_names: Vec::new(),
                    source_location: source_location(),
                })
                .collect(),
        )]);

        (bazel, component, classes)
    }

    #[test]
    fn accepts_a_class_in_the_bound_unit_design_namespace() {
        let (bazel, component, classes) = inputs(&["system.control.Filter.FilterService"]);

        let result = validate_architectural_unit_class_design(&bazel, &component, &classes, None);

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
    }

    #[test]
    fn rejects_a_class_from_another_namespace() {
        let (bazel, component, classes) = inputs(&["system.control.OtherUnit.Service"]);

        let result = validate_architectural_unit_class_design(&bazel, &component, &classes, None);

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains(
            "Architectural unit \"system.control.Filter\" has no refining entity in its unit design."
        ));
        assert!(result.failures[0].contains("Design source file : \"test.puml\""));
        assert!(result.failures[0].contains("Design source line : 1"));
    }

    #[test]
    fn rejects_a_class_with_the_same_unit_name_under_another_parent() {
        let (bazel, component, classes) = inputs(&["system.other.Filter.FilterService"]);

        let result = validate_architectural_unit_class_design(&bazel, &component, &classes, None);

        assert_eq!(result.failures.len(), 1);
    }

    #[test]
    fn rejects_a_lexical_but_not_segment_prefix() {
        let (bazel, component, classes) = inputs(&["system.control.Filtering.FilterService"]);

        let result = validate_architectural_unit_class_design(&bazel, &component, &classes, None);

        assert_eq!(result.failures.len(), 1);
    }

    const INTERNAL_INTERFACE_ID: &str =
        "safety_software_seooc_example.component_example.InternalInterface";

    fn api_index() -> InternalApiIndex {
        api_index_with_methods(&["GetData"])
    }

    fn api_index_with_methods(method_names: &[&str]) -> InternalApiIndex {
        let source_location = source_location();
        InternalApiIndex::build_index(&[ClassDiagram {
            name: "internal_api".to_string(),
            entities: vec![SimpleEntity {
                id: INTERNAL_INTERFACE_ID.to_string(),
                name: "InternalInterface".to_string(),
                enclosing_namespace_id: Some(
                    "safety_software_seooc_example.component_example".to_string(),
                ),
                stereotypes: Vec::new(),
                entity_type: EntityType::Interface,
                type_aliases: Vec::new(),
                variables: Vec::new(),
                methods: method_names
                    .iter()
                    .map(|name| Method {
                        name: (*name).to_string(),
                        return_type: None,
                        source_location: source_location.clone(),
                        visibility: Visibility::Public,
                        parameters: Vec::new(),
                        template_parameters: None,
                        modifiers: Vec::new(),
                    })
                    .collect(),
                template_parameters: None,
                enum_literals: Vec::new(),
                relationships: Vec::new(),
                source_location,
            }],
            free_functions: Vec::new(),
        }])
    }

    fn unit_label(name: &str) -> String {
        format!("@//bazel/rules/rules_score/examples/seooc/{name}:{name}")
    }

    fn bound_interface_inputs(
        bindings: &[(&str, EndpointRole)],
    ) -> (
        BazelArchitecture,
        ComponentDiagramArchitecture,
        BTreeMap<String, Vec<UnitDesignEntity>>,
    ) {
        let mut bazel_units = BTreeMap::new();
        let mut component_units = BTreeMap::new();
        let mut designs = BTreeMap::new();

        for (name, role) in bindings {
            let unit_id = format!("safety_software_seooc_example.component_example.{name}");
            let key = (name.to_lowercase(), Some("component_example".to_string()));
            let label = unit_label(name);
            bazel_units.insert(key.clone(), label.clone());
            component_units.insert(
                key,
                LogicComponent {
                    id: unit_id,
                    name: Some((*name).to_string()),
                    alias: None,
                    parent_id: Some("safety_software_seooc_example.component_example".to_string()),
                    element_type: ComponentType::Component,
                    stereotype: Some("unit".to_string()),
                    relations: vec![LogicRelation {
                        target: INTERNAL_INTERFACE_ID.to_string(),
                        annotation: None,
                        relation_type: ComponentRelationType::InterfaceBinding,
                        source_role: *role,
                        source_location: source_location(),
                    }],
                    source_location: source_location(),
                },
            );
            designs.insert(
                label,
                vec![
                    UnitDesignEntity {
                        id: INTERNAL_INTERFACE_ID.to_string(),
                        entity_type: EntityType::Interface,
                        method_names: vec!["GetData".to_string()],
                        source_location: source_location(),
                    },
                    UnitDesignEntity {
                        id: format!(
                            "safety_software_seooc_example.component_example.{name}.UnitClass"
                        ),
                        entity_type: EntityType::Class,
                        method_names: Vec::new(),
                        source_location: source_location(),
                    },
                ],
            );
        }

        let component = ComponentDiagramArchitecture {
            seooc_set: BTreeMap::new(),
            comp_set: BTreeMap::new(),
            unit_set: component_units,
            entities: vec![LogicComponent {
                id: INTERNAL_INTERFACE_ID.to_string(),
                name: Some("InternalInterface".to_string()),
                alias: None,
                parent_id: Some("safety_software_seooc_example.component_example".to_string()),
                element_type: ComponentType::Interface,
                stereotype: None,
                relations: Vec::new(),
                source_location: source_location(),
            }],
        };
        let bazel = BazelArchitecture {
            seooc_set: BTreeMap::new(),
            comp_set: BTreeMap::new(),
            unit_set: bazel_units,
        };
        (bazel, component, designs)
    }

    #[test]
    fn accepts_required_and_provided_units_when_the_required_design_is_complete() {
        let (bazel, component, designs) = bound_interface_inputs(&[
            ("unit_1", EndpointRole::Required),
            ("unit_2", EndpointRole::Provided),
        ]);

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
    }

    #[test]
    fn does_not_require_provided_unit_design_to_declare_the_interface() {
        let (bazel, component, mut designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Provided)]);
        designs
            .get_mut(&unit_label("unit_1"))
            .unwrap()
            .retain(|entity| entity.entity_type != EntityType::Interface);

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        assert!(
            result.is_empty(),
            "provided-side interface declaration must not be checked: {:?}",
            result.failures
        );
    }

    #[test]
    fn reports_when_any_bound_unit_lacks_the_interface() {
        let (bazel, component, mut designs) = bound_interface_inputs(&[
            ("unit_1", EndpointRole::Provided),
            ("unit_2", EndpointRole::Required),
        ]);
        designs
            .get_mut(&unit_label("unit_2"))
            .unwrap()
            .retain(|entity| entity.entity_type != EntityType::Interface);

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        assert_eq!(result.failures.len(), 1);
        assert!(
            result.failures[0].contains("safety_software_seooc_example.component_example.unit_2")
        );
        assert!(result.failures[0].contains("no corresponding interface"));
    }

    #[test]
    fn reports_internal_api_method_missing_from_bound_unit_interface() {
        let (bazel, component, mut designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Required)]);
        designs.get_mut(&unit_label("unit_1")).unwrap()[0].method_names =
            vec!["SetData".to_string()];

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("GetData"));
        assert!(
            result.failures[0].contains("safety_software_seooc_example.component_example.unit_1")
        );
    }

    #[test]
    fn skips_internal_api_interface_without_unit_bindings() {
        let (bazel, mut component, designs) = bound_interface_inputs(&[]);
        component.entities.push(LogicComponent {
            id: INTERNAL_INTERFACE_ID.to_string(),
            name: Some("InternalInterface".to_string()),
            alias: None,
            parent_id: Some("example.service".to_string()),
            element_type: ComponentType::Interface,
            stereotype: None,
            relations: Vec::new(),
            source_location: source_location(),
        });

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
    }

    #[test]
    fn requires_the_exact_canonical_interface_id() {
        let (bazel, component, mut designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Required)]);
        designs.get_mut(&unit_label("unit_1")).unwrap()[0].id =
            "safety_software_seooc_example.component_example.other.InternalInterface".to_string();

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("no corresponding interface"));
    }

    #[test]
    fn accepts_additional_detail_interface_methods() {
        let (bazel, component, mut designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Required)]);
        designs.get_mut(&unit_label("unit_1")).unwrap()[0]
            .method_names
            .push("LocalHelper".to_string());

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
    }

    #[test]
    fn compares_method_names_case_sensitively() {
        let (bazel, component, mut designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Required)]);
        designs.get_mut(&unit_label("unit_1")).unwrap()[0].method_names =
            vec!["getdata".to_string()];

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("GetData"));
    }

    #[test]
    fn reports_all_missing_api_methods_together() {
        let (bazel, component, mut designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Required)]);
        designs.get_mut(&unit_label("unit_1")).unwrap()[0]
            .method_names
            .clear();

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index_with_methods(&["GetData", "SetData"])),
        );

        assert_eq!(result.failures.len(), 1);
        assert!(result.failures[0].contains("GetData"));
        assert!(result.failures[0].contains("SetData"));
    }

    #[test]
    fn ignores_units_without_interface_bindings() {
        let (bazel, mut component, mut designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Required)]);
        component
            .unit_set
            .values_mut()
            .next()
            .unwrap()
            .relations
            .clear();
        designs
            .get_mut(&unit_label("unit_1"))
            .unwrap()
            .retain(|entity| entity.entity_type != EntityType::Interface);

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
    }

    #[test]
    fn ignores_non_interface_binding_relations() {
        let (bazel, mut component, mut designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Required)]);
        component.unit_set.values_mut().next().unwrap().relations[0].relation_type =
            ComponentRelationType::Dependency;
        designs
            .get_mut(&unit_label("unit_1"))
            .unwrap()
            .retain(|entity| entity.entity_type != EntityType::Interface);

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
    }

    #[test]
    fn skips_component_interface_missing_from_internal_api() {
        let (bazel, component, designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Required)]);
        let empty_internal_api = InternalApiIndex::build_index(&[]);

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&empty_internal_api),
        );

        assert!(
            result.is_empty(),
            "unexpected failures: {:?}",
            result.failures
        );
    }

    #[test]
    fn deduplicates_multiple_bindings_to_the_same_interface() {
        let (bazel, mut component, mut designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Required)]);
        // Add a second interface binding relation to the same interface
        let unit = component.unit_set.values_mut().next().unwrap();
        unit.relations.push(LogicRelation {
            target: INTERNAL_INTERFACE_ID.to_string(),
            annotation: None,
            relation_type: ComponentRelationType::InterfaceBinding,
            source_role: EndpointRole::Provided,
            source_location: source_location(),
        });
        // Remove the interface from unit design to trigger failure
        designs
            .get_mut(&unit_label("unit_1"))
            .unwrap()
            .retain(|entity| entity.entity_type != EntityType::Interface);

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&api_index()),
        );

        // Expect exactly 1 failure despite having 2 relations to the same interface
        assert_eq!(result.failures.len(), 1);
    }

    #[test]
    fn deduplicates_overloaded_method_names_in_missing_methods_error() {
        let (bazel, component, mut designs) =
            bound_interface_inputs(&[("unit_1", EndpointRole::Required)]);
        designs.get_mut(&unit_label("unit_1")).unwrap()[0]
            .method_names
            .clear();

        // Internal API with overloaded methods (same name)
        let overloaded_api = api_index_with_methods(&["GetData", "GetData"]);

        let result = validate_architectural_unit_class_design(
            &bazel,
            &component,
            &designs,
            Some(&overloaded_api),
        );

        assert_eq!(result.failures.len(), 1);
        let error = &result.failures[0];
        // Ensure "GetData" appears only once in the error title and fix message
        assert_eq!(error.matches("\"GetData\"").count(), 2); // 1 in title, 1 in fix
    }
}
