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

//! Validation: compare public API references in the static design diagram with
//! interfaces declared by the public API class diagram.

use std::collections::{BTreeMap, BTreeSet};

use super::shared::{best_string_suggestion, format_name_list};
use crate::models::{ComponentDiagramArchitecture, LogicComponentExt, PublicApiIndex};
use crate::results::{ErrorBuilder, ErrorCategory};
use crate::{Diagnostics, ValidationResult};
use source_location::SourceLocation;

/// Run component-vs-public-API reference validation.
pub fn validate_component_public_api(
    component_diagram: &ComponentDiagramArchitecture,
    public_api_index: &PublicApiIndex,
) -> ValidationResult {
    ComponentPublicApiValidator::new(component_diagram, public_api_index).run()
}

struct ComponentPublicApiValidator {
    /// Public API interfaces explicitly declared in the component diagram.
    component_public_api_sources: BTreeMap<String, SourceLocation>,
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
            component_public_api_sources: collect_component_public_api_sources(component_diagram),
            design_public_api_ids: public_api_index.api_names().cloned().collect(),
            result: ValidationResult::default(),
        }
    }

    fn run(mut self) -> ValidationResult {
        let component_public_api_ids: BTreeSet<String> =
            self.component_public_api_sources.keys().cloned().collect();

        append_debug_log(
            &mut self.result.diagnostics,
            &component_public_api_ids,
            &self.seooc_related_public_api_ids,
            &self.design_public_api_ids,
        );
        self.check_component_public_apis_declared_by_public_api();
        self.check_component_public_apis_have_relationship();
        self.result
    }

    fn check_component_public_apis_declared_by_public_api(&mut self) {
        let component_public_api_ids: BTreeSet<String> =
            self.component_public_api_sources.keys().cloned().collect();
        let missing_public_apis: BTreeSet<String> = component_public_api_ids
            .difference(&self.design_public_api_ids)
            .cloned()
            .collect();

        if !missing_public_apis.is_empty() {
            self.result.add_failure(format_missing_public_api_error(
                &missing_public_apis,
                &self.component_public_api_sources,
                &self.design_public_api_ids,
            ));
        }
    }

    fn check_component_public_apis_have_relationship(&mut self) {
        let component_public_api_ids: BTreeSet<String> =
            self.component_public_api_sources.keys().cloned().collect();
        let unrelated_public_apis: BTreeSet<String> = component_public_api_ids
            .difference(&self.seooc_related_public_api_ids)
            .cloned()
            .collect();

        if !unrelated_public_apis.is_empty() {
            self.result.add_failure(format_unrelated_public_api_error(
                &unrelated_public_apis,
                &self.component_public_api_sources,
            ));
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

fn collect_component_public_api_sources(
    component_diagram: &ComponentDiagramArchitecture,
) -> BTreeMap<String, SourceLocation> {
    component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_interface() && entity.parent_id.is_none())
        .map(|entity| (entity.id.clone(), entity.source_location.clone()))
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

fn format_missing_public_api_error(
    missing_public_apis: &BTreeSet<String>,
    component_public_api_sources: &BTreeMap<String, SourceLocation>,
    design_public_api_ids: &BTreeSet<String>,
) -> String {
    let missing_public_api_names = format_name_list(missing_public_apis);
    let case_mismatch_public_apis =
        collect_case_mismatch_public_apis(missing_public_apis, design_public_api_ids);
    let has_case_mismatch = !case_mismatch_public_apis.is_empty();
    let case_mismatch_names = format_name_list(&case_mismatch_public_apis);
    let case_mismatch_title_suffix = if has_case_mismatch {
        format!("; use {case_mismatch_names} (case-sensitive)")
    } else {
        String::new()
    };
    let case_mismatch_fix_suffix = if has_case_mismatch {
        format!("; if already declared, use {case_mismatch_names} (case-sensitive)")
    } else {
        String::new()
    };

    let mut error = ErrorBuilder::new(ErrorCategory::Interface)
        .title(format!(
            "public API interface(s) {missing_public_api_names} from the static diagram not found in the public API diagram{case_mismatch_title_suffix}",
        ))
        .field("missing public APIs", missing_public_api_names.clone());

    for interface_id in missing_public_apis {
        if let Some(source_location) = component_public_api_sources.get(interface_id) {
            let (source_file, source_line) = source_location.display();
            error = error
                .field(
                    format!("static source file for \"{interface_id}\""),
                    format!("\"{source_file}\""),
                )
                .field(
                    format!("static source line for \"{interface_id}\""),
                    source_line.to_string(),
                );
        }

        if case_mismatch_public_apis.contains(interface_id) {
            continue;
        }

        if let Some(suggested_interface) = best_string_suggestion(
            interface_id,
            design_public_api_ids.iter().map(String::as_str),
        ) {
            error = error.suggest(interface_id, Some("interface"), &suggested_interface);
        }
    }

    error
        .fix(format!(
            "add public API declaration(s) {missing_public_api_names} in the public API diagram, or remove those interface declarations from the static diagram{case_mismatch_fix_suffix}",
        ))
        .build()
}

fn collect_case_mismatch_public_apis(
    missing_public_apis: &BTreeSet<String>,
    design_public_api_ids: &BTreeSet<String>,
) -> BTreeSet<String> {
    missing_public_apis
        .iter()
        .filter(|missing_id| {
            design_public_api_ids.iter().any(|design_id| {
                design_id != *missing_id && design_id.eq_ignore_ascii_case(missing_id)
            })
        })
        .cloned()
        .collect()
}

fn format_unrelated_public_api_error(
    unrelated_public_apis: &BTreeSet<String>,
    component_public_api_sources: &BTreeMap<String, SourceLocation>,
) -> String {
    let public_api_names = format_name_list(unrelated_public_apis);

    let mut error = ErrorBuilder::new(ErrorCategory::Interface)
        .title(format!(
            "public API interface(s) {public_api_names} in the static diagram have no relationship to the SEooC"
        ))
        .field("public APIs", public_api_names.clone());

    for interface_id in unrelated_public_apis {
        if let Some(source_location) = component_public_api_sources.get(interface_id) {
            let (source_file, source_line) = source_location.display();
            error = error
                .field(
                    format!("static source file for \"{interface_id}\""),
                    format!("\"{source_file}\""),
                )
                .field(
                    format!("static source line for \"{interface_id}\""),
                    source_line.to_string(),
                );
        }
    }

    error
        .fix(format!(
            "connect public API interface(s) {public_api_names} from the SEooC boundary in the static diagram, or remove those interface declarations if they are not intended to be public"
        ))
        .build()
}

#[cfg(test)]
#[path = "test/component_public_api_validator_test.rs"]
mod tests;
