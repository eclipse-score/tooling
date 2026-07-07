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

use super::shared::format_name_list;
use crate::models::{ComponentDiagramArchitecture, InternalApiIndex, LogicComponentExt};
use crate::{Diagnostics, ValidationResult};

/// Run component-vs-internal-API interface reference validation.
pub fn validate_component_internal_api(
    component_diagram: &ComponentDiagramArchitecture,
    internal_api_diagram: &InternalApiIndex,
) -> ValidationResult {
    ComponentInternalApiValidator::new(component_diagram, internal_api_diagram).run()
}

struct ComponentInternalApiValidator {
    component_interface_ids: BTreeSet<String>,
    internal_api_interface_ids: BTreeSet<String>,
    result: ValidationResult,
}

impl ComponentInternalApiValidator {
    fn new(
        component_diagram: &ComponentDiagramArchitecture,
        internal_api_diagram: &InternalApiIndex,
    ) -> Self {
        Self {
            component_interface_ids: collect_component_interface_ids(component_diagram),
            internal_api_interface_ids: collect_internal_api_interface_ids(internal_api_diagram),
            result: ValidationResult::default(),
        }
    }

    fn run(mut self) -> ValidationResult {
        append_debug_log(
            &mut self.result.diagnostics,
            &self.component_interface_ids,
            &self.internal_api_interface_ids,
        );
        self.check_component_interfaces_declared_by_internal_api();
        self.result
    }

    fn check_component_interfaces_declared_by_internal_api(&mut self) {
        let missing_interfaces: BTreeSet<String> = self
            .component_interface_ids
            .difference(&self.internal_api_interface_ids)
            .cloned()
            .collect();

        if !missing_interfaces.is_empty() {
            self.result
                .add_failure(format_missing_internal_api_interface_error(
                    &missing_interfaces,
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
        "Internal API consistency failure: Missing internal API interface:\n\
          Missing interfaces  : {missing_interfaces}\n\
          Action              : Add each component interface to the internal API diagram or remove it from the component diagram",
        missing_interfaces = format_name_list(missing_internal_api_interfaces),
    )
}

#[cfg(test)]
#[path = "test/component_internal_api_validator_test.rs"]
mod tests;
