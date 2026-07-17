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

//! Validation: compare public API references in the component diagram with
//! interfaces declared by the public API class diagram.

use std::collections::BTreeSet;

use super::shared::format_name_list;
use crate::models::{ComponentDiagramArchitecture, LogicComponentExt, PublicApiIndex};
use crate::{Diagnostics, ValidationResult};

/// Run component-vs-public-API reference validation.
pub fn validate_component_public_api(
    component_diagram: &ComponentDiagramArchitecture,
    public_api_index: &PublicApiIndex,
) -> ValidationResult {
    ComponentPublicApiValidator::new(component_diagram, public_api_index).run()
}

struct ComponentPublicApiValidator {
    /// Public API interfaces explicitly declared in the component diagram.
    component_public_api_ids: BTreeSet<String>,
    /// Public API interfaces referenced by relationships from SEooC entities.
    seooc_related_public_api_ids: BTreeSet<String>,
    /// Public API interfaces declared in the public API class diagram.
    design_public_api_ids: BTreeSet<String>,
    result: ValidationResult,
}

impl ComponentPublicApiValidator {
    fn new(
        component_diagram: &ComponentDiagramArchitecture,
        public_api_index: &PublicApiIndex,
    ) -> Self {
        Self {
            seooc_related_public_api_ids: collect_seooc_related_public_api_ids(component_diagram),
            component_public_api_ids: collect_component_public_api_ids(component_diagram),
            design_public_api_ids: public_api_index.api_names().cloned().collect(),
            result: ValidationResult::default(),
        }
    }

    fn run(mut self) -> ValidationResult {
        append_debug_log(
            &mut self.result.diagnostics,
            &self.component_public_api_ids,
            &self.seooc_related_public_api_ids,
            &self.design_public_api_ids,
        );
        self.check_component_public_apis_declared_by_public_api();
        self.check_component_public_apis_have_relationship();
        self.result
    }

    fn check_component_public_apis_declared_by_public_api(&mut self) {
        let missing_public_apis: BTreeSet<String> = self
            .component_public_api_ids
            .difference(&self.design_public_api_ids)
            .cloned()
            .collect();

        if !missing_public_apis.is_empty() {
            self.result
                .add_failure(format_missing_public_api_error(&missing_public_apis));
        }
    }

    fn check_component_public_apis_have_relationship(&mut self) {
        let unrelated_public_apis: BTreeSet<String> = self
            .component_public_api_ids
            .difference(&self.seooc_related_public_api_ids)
            .cloned()
            .collect();

        if !unrelated_public_apis.is_empty() {
            self.result
                .add_failure(format_unrelated_public_api_error(&unrelated_public_apis));
        }
    }
}

fn append_debug_log(
    diagnostics: &mut Diagnostics,
    component_public_api_ids: &BTreeSet<String>,
    seooc_related_public_api_ids: &BTreeSet<String>,
    design_public_api_ids: &BTreeSet<String>,
) {
    diagnostics.debug(|| "Component public APIs checked against public API diagram:".to_string());
    for api_id in component_public_api_ids {
        diagnostics.debug(|| format!("  {api_id}"));
    }

    diagnostics.debug(|| "Component public APIs referenced by component relations:".to_string());
    for api_id in seooc_related_public_api_ids {
        diagnostics.debug(|| format!("  {api_id}"));
    }

    diagnostics.debug(|| "Public API entries available for component public APIs:".to_string());
    for api_id in design_public_api_ids {
        diagnostics.debug(|| format!("  {api_id}"));
    }
}

fn collect_component_public_api_ids(
    component_diagram: &ComponentDiagramArchitecture,
) -> BTreeSet<String> {
    component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_interface() && entity.parent_id.is_none())
        .map(|entity| entity.id.clone())
        .collect()
}

fn collect_seooc_related_public_api_ids(
    component_diagram: &ComponentDiagramArchitecture,
) -> BTreeSet<String> {
    let interface_ids: BTreeSet<String> = component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_interface())
        .map(|entity| entity.id.clone())
        .collect();

    component_diagram
        .seooc_set
        .values()
        .flat_map(|entity| entity.relations.iter())
        .filter(|relation| interface_ids.contains(&relation.target))
        .map(|relation| relation.target.clone())
        .collect()
}

fn format_missing_public_api_error(missing_public_apis: &BTreeSet<String>) -> String {
    format!(
        "Public API consistency failure: Missing public API declaration:\n\
          Missing public APIs : {missing_public_apis}\n\
          Action              : Declare each public API interface in the public API class diagram or remove it from the component diagram",
        missing_public_apis = format_name_list(missing_public_apis),
    )
}

fn format_unrelated_public_api_error(unrelated_public_apis: &BTreeSet<String>) -> String {
    format!(
        "Public API consistency failure: Public API interface has no component relationship:\n\
          Public APIs          : {public_apis}\n\
          Action               : Connect each public API interface to the SEooC, or remove it from the static design diagram",
        public_apis = format_name_list(unrelated_public_apis),
    )
}

#[cfg(test)]
#[path = "test/component_public_api_validator_test.rs"]
mod tests;
