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

//! Validation: compare component-diagram interfaces with interfaces declared
//! by the internal API diagram.

use std::collections::{BTreeMap, BTreeSet};

use super::shared::{best_string_suggestion, format_name_list};
use crate::models::{ComponentDiagramArchitecture, InternalApiIndex, LogicComponentExt};
use crate::results::{ErrorBuilder, ErrorCategory};
use crate::{Diagnostics, ValidationResult};
use source_location::SourceLocation;

/// Run component-vs-internal-API interface reference validation.
pub fn validate_component_internal_api(
    component_diagram: &ComponentDiagramArchitecture,
    internal_api_diagram: &InternalApiIndex,
) -> ValidationResult {
    ComponentInternalApiValidator::new(component_diagram, internal_api_diagram).run()
}

struct ComponentInternalApiValidator {
    component_interface_sources: BTreeMap<String, SourceLocation>,
    internal_api_interface_ids: BTreeSet<String>,
    result: ValidationResult,
}

impl ComponentInternalApiValidator {
    fn new(
        component_diagram: &ComponentDiagramArchitecture,
        internal_api_diagram: &InternalApiIndex,
    ) -> Self {
        let component_interface_sources =
            collect_component_internal_interface_sources(component_diagram);

        Self {
            component_interface_sources,
            internal_api_interface_ids: collect_internal_api_interface_ids(internal_api_diagram),
            result: ValidationResult::default(),
        }
    }

    fn run(mut self) -> ValidationResult {
        let component_interface_ids: BTreeSet<String> =
            self.component_interface_sources.keys().cloned().collect();

        append_debug_log(
            &mut self.result.diagnostics,
            &component_interface_ids,
            &self.internal_api_interface_ids,
        );
        self.check_component_interfaces_declared_by_internal_api();
        self.result
    }

    fn check_component_interfaces_declared_by_internal_api(&mut self) {
        let component_interface_ids: BTreeSet<String> =
            self.component_interface_sources.keys().cloned().collect();
        let missing_interfaces: BTreeSet<String> = component_interface_ids
            .difference(&self.internal_api_interface_ids)
            .cloned()
            .collect();

        if !missing_interfaces.is_empty() {
            self.result
                .add_failure(format_missing_internal_api_interface_error(
                    &missing_interfaces,
                    &self.component_interface_sources,
                    &self.internal_api_interface_ids,
                ));
        }
    }
}

fn append_debug_log(
    diagnostics: &mut Diagnostics,
    component_interface_ids: &BTreeSet<String>,
    internal_api_interface_ids: &BTreeSet<String>,
) {
    diagnostics.debug(|| "Component interfaces checked against internal API:".to_string());
    for interface_id in component_interface_ids {
        diagnostics.debug(|| format!("  {interface_id}"));
    }

    diagnostics.debug(|| "Internal API interfaces available for component interfaces:".to_string());
    for interface_id in internal_api_interface_ids {
        diagnostics.debug(|| format!("  {interface_id}"));
    }
}

fn collect_component_internal_interface_sources(
    component_diagram: &ComponentDiagramArchitecture,
) -> BTreeMap<String, SourceLocation> {
    component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_interface() && entity.parent_id.is_some())
        .map(|entity| (entity.id.clone(), entity.source_location.clone()))
        .collect()
}

fn collect_internal_api_interface_ids(internal_api_diagram: &InternalApiIndex) -> BTreeSet<String> {
    internal_api_diagram
        .interfaces()
        .map(|interface| interface.id.clone())
        .collect()
}

fn format_missing_internal_api_interface_error(
    missing_internal_api_interfaces: &BTreeSet<String>,
    component_interface_sources: &BTreeMap<String, SourceLocation>,
    internal_api_interface_ids: &BTreeSet<String>,
) -> String {
    let missing_interfaces = format_name_list(missing_internal_api_interfaces);

    let mut error = ErrorBuilder::new(ErrorCategory::Interface)
        .title(format!(
            "component interface(s) {missing_interfaces} from the component diagram not found in the internal API diagram"
        ))
        .field("missing interfaces", missing_interfaces.clone());

    for interface_id in missing_internal_api_interfaces {
        if let Some(source_location) = component_interface_sources.get(interface_id) {
            let (source_file, source_line) = source_location.display();
            error = error
                .field(
                    format!("component source file for \"{interface_id}\""),
                    format!("\"{source_file}\""),
                )
                .field(
                    format!("component source line for \"{interface_id}\""),
                    source_line.to_string(),
                );
        }

        if let Some(suggested_interface) = best_string_suggestion(
            interface_id,
            internal_api_interface_ids.iter().map(String::as_str),
        ) {
            error = error.suggest(Some("interface"), &suggested_interface);
        }
    }

    error
        .fix(format!(
            "add interface declaration(s) {missing_interfaces} in the internal API diagram, or remove those interface declarations from the component diagram"
        ))
        .build()
}

#[cfg(test)]
#[path = "test/component_internal_api_validator_test.rs"]
mod tests;
