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

//! Validation: check that `static_view` component diagrams only reference
//! components/units that are also defined in the `static` component
//! diagrams.
//!
//! `static_view` diagrams are partial "views" onto the full static
//! architecture: any component/unit they define must already be defined in
//! `static`, and a `static_view` diagram may only include a subset of the
//! units/components of the matching `static` component. It is not permitted
//! to introduce components/units in `static_view` that do not exist in
//! `static`.
//!
//! A component/unit may appear in more than one `static_view` diagram (e.g.
//! overlapping views); such repetition across `static_view` diagrams is not
//! checked for duplicates, only consistency with `static` is checked.

use std::collections::BTreeMap;

use crate::models::{ComponentDiagramArchitecture, EntityKey, LogicComponent};
use crate::results::{ErrorBuilder, ErrorCategory};
use crate::{Diagnostics, ValidationResult};

/// Run static-vs-static_view component diagram consistency validation.
pub fn validate_static_view_consistency(
    static_diagram: &ComponentDiagramArchitecture,
    static_view_diagram: &ComponentDiagramArchitecture,
) -> ValidationResult {
    StaticViewConsistencyValidator::new().run(static_diagram, static_view_diagram)
}

/// Compares a `static` [`ComponentDiagramArchitecture`] against a
/// `static_view` [`ComponentDiagramArchitecture`], reporting any
/// component/unit defined in `static_view` that is not also defined in
/// `static`. A parentless static-view entry may reference a uniquely named
/// static entity from a different architectural scope.
struct StaticViewConsistencyValidator {
    result: ValidationResult,
}

impl StaticViewConsistencyValidator {
    fn new() -> Self {
        Self {
            result: ValidationResult::default(),
        }
    }

    fn run(
        mut self,
        static_diagram: &ComponentDiagramArchitecture,
        static_view_diagram: &ComponentDiagramArchitecture,
    ) -> ValidationResult {
        append_debug_log(
            &mut self.result.diagnostics,
            static_diagram,
            static_view_diagram,
        );
        self.check_only_known_entities(
            &static_diagram.comp_set,
            &static_view_diagram.comp_set,
            "component",
        );
        self.check_only_known_entities(
            &static_diagram.unit_set,
            &static_view_diagram.unit_set,
            "unit",
        );
        self.result
    }

    /// Reports every entity present in `static_view_set` that is not present
    /// in `static_set`. Parent-qualified view entries must match exactly;
    /// parentless view entries may match a uniquely named static entity.
    fn check_only_known_entities(
        &mut self,
        static_set: &BTreeMap<EntityKey, LogicComponent>,
        static_view_set: &BTreeMap<EntityKey, LogicComponent>,
        entity_type: &str,
    ) {
        for (key, entity) in static_view_set {
            if !Self::is_known_static_entity(static_set, key) {
                let (name, parent) = key;
                let parent_str = parent.as_deref().unwrap_or("(top-level)");
                self.result
                    .add_failure(Self::format_extra(entity_type, name, parent_str, entity));
            }
        }
    }

    fn is_known_static_entity(
        static_set: &BTreeMap<EntityKey, LogicComponent>,
        static_view_key: &EntityKey,
    ) -> bool {
        if static_set.contains_key(static_view_key) {
            return true;
        }

        let (name, parent) = static_view_key;
        parent.is_none()
            && static_set
                .keys()
                .filter(|(static_name, _)| static_name == name)
                .take(2)
                .count()
                == 1
    }

    fn format_extra(
        entity_type: &str,
        name: &str,
        parent_str: &str,
        entity: &LogicComponent,
    ) -> String {
        let (source_file, source_line) = entity.source_location.display();

        ErrorBuilder::new(ErrorCategory::Design)
            .title(format!(
                "{entity_type} \"{name}\" in the static_view diagram is not defined in the static diagram"
            ))
            .field("alias", format!("\"{name}\""))
            .field("parent", parent_str)
            .field("static_view source file", format!("\"{source_file}\""))
            .field("static_view source line", source_line.to_string())
            .fix(format!(
                "add {entity_type} \"{name}\" under \"{parent_str}\" to the static diagram, or remove it from the static_view diagram"
            ))
            .build()
    }
}

fn append_debug_log(
    diagnostics: &mut Diagnostics,
    static_diagram: &ComponentDiagramArchitecture,
    static_view_diagram: &ComponentDiagramArchitecture,
) {
    diagnostics.debug(|| "static component set:".to_string());
    for key in static_diagram.comp_set.keys() {
        diagnostics.debug(|| format!("  {:?}", key));
    }
    diagnostics.debug(|| "static unit set:".to_string());
    for key in static_diagram.unit_set.keys() {
        diagnostics.debug(|| format!("  {:?}", key));
    }
    diagnostics.debug(|| "static_view component set:".to_string());
    for key in static_view_diagram.comp_set.keys() {
        diagnostics.debug(|| format!("  {:?}", key));
    }
    diagnostics.debug(|| "static_view unit set:".to_string());
    for key in static_view_diagram.unit_set.keys() {
        diagnostics.debug(|| format!("  {:?}", key));
    }
}

#[cfg(test)]
#[path = "test/static_view_consistency_validator_test.rs"]
mod tests;
