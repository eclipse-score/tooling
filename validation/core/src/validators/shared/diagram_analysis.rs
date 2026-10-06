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

//! Derived diagram analysis shared by validators.

use std::collections::{BTreeMap, BTreeSet};

use source_location::SourceLocation;
use uid_utils::{resolve_uid, UidMatch};

use crate::models::{
    is_external_endpoint, ComponentDiagramArchitecture, ComponentRelationType, EndpointRole,
    LogicComponentExt, ObservedSequenceCall,
};

/// Unit interface bindings keyed by component id.
pub(in crate::validators) type UnitBindings = BTreeMap<String, UnitInterfaces>;

#[derive(Clone, Default)]
pub(in crate::validators) struct UnitInterfaces {
    pub(in crate::validators) source_location: Option<SourceLocation>,
    pub(in crate::validators) all_interfaces: BTreeSet<String>,
    pub(in crate::validators) required_interfaces: BTreeSet<String>,
    pub(in crate::validators) provided_interfaces: BTreeSet<String>,
}

/// One sequence call with caller and callee mapped to unit ids. A participant
/// that does not resolve to exactly one unit keeps its uid.
#[derive(Clone)]
pub(in crate::validators) struct SequenceCallContext {
    pub(in crate::validators) caller_unit: String,
    pub(in crate::validators) callee_unit: String,
    pub(in crate::validators) method: String,
    pub(in crate::validators) source_location: SourceLocation,
    pub(in crate::validators) caller_interfaces: BTreeSet<String>,
    pub(in crate::validators) callee_interfaces: BTreeSet<String>,
}

impl SequenceCallContext {
    pub(in crate::validators) fn has_shared_interfaces(&self) -> bool {
        !self.caller_interfaces.is_disjoint(&self.callee_interfaces)
    }
}

pub(in crate::validators) fn build_unit_bindings(
    component_diagram: &ComponentDiagramArchitecture,
) -> UnitBindings {
    let interface_ids: BTreeSet<&str> = component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_interface())
        .map(|entity| entity.id.as_str())
        .collect();
    let mut unit_bindings = BTreeMap::new();

    // `unit_set` is already merged, so units sharing an alias under different
    // parents stay distinct by id.
    for entity in component_diagram.unit_set.values() {
        let mut bindings = UnitInterfaces {
            source_location: Some(entity.source_location.clone()),
            ..UnitInterfaces::default()
        };

        for relation in &entity.relations {
            if !interface_ids.contains(relation.target.as_str()) {
                continue;
            }

            bindings.all_interfaces.insert(relation.target.clone());

            if relation.relation_type != ComponentRelationType::InterfaceBinding {
                continue;
            }

            match relation.source_role {
                EndpointRole::Required => {
                    bindings.required_interfaces.insert(relation.target.clone());
                }
                EndpointRole::Provided => {
                    bindings.provided_interfaces.insert(relation.target.clone());
                }
                EndpointRole::None => {}
            }
        }

        unit_bindings.insert(entity.id.clone(), bindings);
    }

    unit_bindings
}

/// Matches a sequence participant uid against the unit ids.
pub(in crate::validators) fn resolve_unit<'a>(
    unit_bindings: &'a UnitBindings,
    uid: &str,
) -> UidMatch<'a> {
    resolve_uid(uid, None, unit_bindings.keys().map(String::as_str))
}

fn all_interfaces_for_unit(unit_bindings: &UnitBindings, unit_id: &str) -> BTreeSet<String> {
    unit_bindings
        .get(unit_id)
        .map(|bindings| bindings.all_interfaces.clone())
        .unwrap_or_default()
}

fn unit_id_for_participant(unit_bindings: &UnitBindings, uid: &str) -> String {
    if is_external_endpoint(uid) {
        return uid.to_string();
    }

    match resolve_unit(unit_bindings, uid) {
        UidMatch::Resolved(unit_id) => unit_id.to_string(),
        UidMatch::Ambiguous(_) | UidMatch::Unresolved => uid.to_string(),
    }
}

pub(in crate::validators) fn build_observed_call_contexts(
    observed_calls: &[ObservedSequenceCall],
    unit_bindings: &UnitBindings,
) -> Vec<SequenceCallContext> {
    observed_calls
        .iter()
        .map(|call| {
            let caller_unit = unit_id_for_participant(unit_bindings, &call.caller);
            let callee_unit = unit_id_for_participant(unit_bindings, &call.callee);

            SequenceCallContext {
                caller_interfaces: all_interfaces_for_unit(unit_bindings, &caller_unit),
                callee_interfaces: all_interfaces_for_unit(unit_bindings, &callee_unit),
                caller_unit,
                callee_unit,
                method: call.method.clone(),
                source_location: call.source_location.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ComponentDiagramInputs, ComponentType, LogicComponent, LogicRelation};
    use crate::validators::fixtures::dummy_source_location;
    use crate::ValidationResult;

    fn entity(
        id: &str,
        alias: &str,
        parent_id: Option<&str>,
        element_type: ComponentType,
        stereotype: Option<&str>,
        relations: Vec<LogicRelation>,
    ) -> LogicComponent {
        LogicComponent {
            id: id.to_string(),
            name: Some(alias.to_string()),
            alias: Some(alias.to_string()),
            parent_id: parent_id.map(str::to_string),
            element_type,
            stereotype: stereotype.map(str::to_string),
            relations,
            source_location: dummy_source_location(),
        }
    }

    fn interface_binding(target: &str, source_role: EndpointRole) -> LogicRelation {
        LogicRelation {
            target: target.to_string(),
            annotation: None,
            relation_type: ComponentRelationType::InterfaceBinding,
            source_role,
            source_location: dummy_source_location(),
        }
    }

    fn two_parents_with_unit_x() -> ComponentDiagramArchitecture {
        let entities = vec![
            entity(
                "comp_a",
                "comp_a",
                None,
                ComponentType::Component,
                Some("component"),
                Vec::new(),
            ),
            entity(
                "comp_b",
                "comp_b",
                None,
                ComponentType::Component,
                Some("component"),
                Vec::new(),
            ),
            entity(
                "iface_a",
                "iface_a",
                None,
                ComponentType::Interface,
                None,
                Vec::new(),
            ),
            entity(
                "iface_b",
                "iface_b",
                None,
                ComponentType::Interface,
                None,
                Vec::new(),
            ),
            entity(
                "comp_a.unit_x",
                "unit_x",
                Some("comp_a"),
                ComponentType::Component,
                Some("unit"),
                vec![interface_binding("iface_a", EndpointRole::Provided)],
            ),
            entity(
                "comp_b.unit_x",
                "unit_x",
                Some("comp_b"),
                ComponentType::Component,
                Some("unit"),
                vec![interface_binding("iface_b", EndpointRole::Required)],
            ),
            entity(
                "comp_b.unit_y",
                "unit_y",
                Some("comp_b"),
                ComponentType::Component,
                Some("unit"),
                Vec::new(),
            ),
        ];

        let mut result = ValidationResult::default();
        let architecture = ComponentDiagramInputs { entities }.to_diagram_architecture(&mut result);
        assert!(
            result.is_empty(),
            "expected no failures, got {:?}",
            result.failures
        );
        architecture
    }

    fn observed_call(caller: &str, callee: &str) -> ObservedSequenceCall {
        ObservedSequenceCall {
            caller: caller.to_string(),
            callee: callee.to_string(),
            method: "Call()".to_string(),
            source_location: dummy_source_location(),
        }
    }

    #[test]
    fn units_sharing_an_alias_under_different_parents_keep_distinct_id_bindings() {
        let bindings = build_unit_bindings(&two_parents_with_unit_x());

        assert_eq!(
            bindings.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["comp_a.unit_x", "comp_b.unit_x", "comp_b.unit_y"]
        );
        assert_eq!(
            bindings["comp_a.unit_x"].all_interfaces,
            BTreeSet::from(["iface_a".to_string()])
        );
        assert_eq!(
            bindings["comp_b.unit_x"].all_interfaces,
            BTreeSet::from(["iface_b".to_string()])
        );
    }

    #[test]
    fn call_contexts_map_qualified_and_leaf_uids_to_unit_ids() {
        let bindings = build_unit_bindings(&two_parents_with_unit_x());

        let contexts =
            build_observed_call_contexts(&[observed_call("comp_a.unit_x", "unit_y")], &bindings);

        assert_eq!(contexts[0].caller_unit, "comp_a.unit_x");
        assert_eq!(contexts[0].callee_unit, "comp_b.unit_y");
        assert_eq!(
            contexts[0].caller_interfaces,
            BTreeSet::from(["iface_a".to_string()])
        );
    }

    #[test]
    fn call_contexts_keep_ambiguous_unresolved_and_external_uids() {
        let bindings = build_unit_bindings(&two_parents_with_unit_x());

        let contexts = build_observed_call_contexts(
            &[
                observed_call("unit_x", "missing"),
                observed_call("ExternalEndpoint", "unit_y"),
            ],
            &bindings,
        );

        assert_eq!(contexts[0].caller_unit, "unit_x");
        assert!(contexts[0].caller_interfaces.is_empty());
        assert_eq!(contexts[0].callee_unit, "missing");
        assert_eq!(contexts[1].caller_unit, "ExternalEndpoint");
        assert_eq!(contexts[1].callee_unit, "comp_b.unit_y");
    }
}
