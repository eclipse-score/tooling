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

use std::collections::BTreeSet;

use crate::models::{ComponentDiagramArchitecture, Errors, InternalApiIndex, LogicComponentExt};

/// Run component-vs-internal-API interface reference validation.
pub fn validate_component_internal_api(
    component_diagram: &ComponentDiagramArchitecture,
    internal_api_diagram: &InternalApiIndex,
    errors: Errors,
) -> Errors {
    ComponentInternalApiValidator::new(component_diagram, internal_api_diagram, errors).run()
}

struct ComponentInternalApiValidator {
    component_interface_ids: BTreeSet<String>,
    internal_api_interface_ids: BTreeSet<String>,
    errors: Errors,
}

impl ComponentInternalApiValidator {
    fn new(
        component_diagram: &ComponentDiagramArchitecture,
        internal_api_diagram: &InternalApiIndex,
        errors: Errors,
    ) -> Self {
        Self {
            component_interface_ids: collect_component_interface_ids(component_diagram),
            internal_api_interface_ids: collect_internal_api_interface_ids(internal_api_diagram),
            errors,
        }
    }

    fn run(mut self) -> Errors {
        self.errors.debug_output = self.build_debug_log();
        self.check_component_interfaces_declared_by_internal_api();
        self.errors
    }

    fn build_debug_log(&self) -> String {
        let mut log = String::new();

        log.push_str("DEBUG: Component interfaces checked against internal API:\n");
        for interface_id in &self.component_interface_ids {
            log.push_str(&format!("  {interface_id}\n"));
        }

        log.push_str("DEBUG: Internal API interfaces available for component interfaces:\n");
        for interface_id in &self.internal_api_interface_ids {
            log.push_str(&format!("  {interface_id}\n"));
        }

        log
    }

    fn check_component_interfaces_declared_by_internal_api(&mut self) {
        let missing_interfaces: BTreeSet<String> = self
            .component_interface_ids
            .difference(&self.internal_api_interface_ids)
            .cloned()
            .collect();

        if !missing_interfaces.is_empty() {
            self.errors
                .push(format_missing_internal_api_interface_error(
                    &missing_interfaces,
                ));
        }
    }
}

fn collect_component_interface_ids(
    component_diagram: &ComponentDiagramArchitecture,
) -> BTreeSet<String> {
    component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_interface())
        .map(|entity| entity.id.clone())
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
) -> String {
    format!(
        "Internal API consistency violation: Missing internal API interface:\n\
          Missing interfaces  : {missing_interfaces}\n\
          Action              : Add each component interface to the internal API diagram or remove it from the component diagram",
        missing_interfaces = format_name_list(missing_internal_api_interfaces),
    )
}

fn format_name_list(names: &BTreeSet<String>) -> String {
    if names.is_empty() {
        return "<none>".to_string();
    }

    names
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[path = "test/component_internal_api_validator_test.rs"]
mod tests;
