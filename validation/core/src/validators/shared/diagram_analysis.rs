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

use crate::models::{
    ComponentDiagramArchitecture, ComponentRelationType, EndpointRole, LogicComponentExt,
    ObservedSequenceCall,
};

pub(in crate::validators) type UnitBindings = BTreeMap<String, UnitInterfaces>;

#[derive(Clone, Default)]
pub(in crate::validators) struct UnitInterfaces {
    pub(in crate::validators) source_location: Option<SourceLocation>,
    pub(in crate::validators) all_interfaces: BTreeSet<String>,
    pub(in crate::validators) required_interfaces: BTreeSet<String>,
    pub(in crate::validators) provided_interfaces: BTreeSet<String>,
}

#[derive(Clone)]
pub(in crate::validators) struct SequenceCallContext<'a> {
    pub(in crate::validators) caller_unit: String,
    pub(in crate::validators) callee_unit: String,
    pub(in crate::validators) method: &'a str,
    pub(in crate::validators) source_location: &'a SourceLocation,
    pub(in crate::validators) caller_interfaces: BTreeSet<String>,
    pub(in crate::validators) callee_interfaces: BTreeSet<String>,
}

impl SequenceCallContext<'_> {
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

    // `unit_set` is already merged and keyed by canonical unit and parent IDs.
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

pub(in crate::validators) fn all_interfaces_for_unit_id(
    unit_bindings: &UnitBindings,
    unit_id: &str,
) -> BTreeSet<String> {
    unit_bindings
        .get(unit_id)
        .map(|bindings| bindings.all_interfaces.clone())
        .unwrap_or_default()
}

pub(in crate::validators) fn build_observed_call_contexts<'a>(
    observed_calls: &'a [ObservedSequenceCall],
    participants: &BTreeMap<String, crate::models::SequenceParticipantInfo>,
    unit_bindings: &UnitBindings,
) -> Vec<SequenceCallContext<'a>> {
    // Sequence interactions reference participants by sender/receiver names,
    // so validators need a declaration bridge from those reference names back
    // to the canonical participant uid used as the participant map key.
    let mut participant_uids_by_reference_name: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (participant_uid, participant_info) in participants {
        participant_uids_by_reference_name
            .entry(participant_info.reference_name.as_str())
            .or_default()
            .insert(participant_uid.as_str());
    }

    observed_calls
        .iter()
        .map(|call| {
            let caller_unit = resolve_observed_call_unit_id(
                unit_bindings,
                &participant_uids_by_reference_name,
                &call.caller,
            );
            let callee_unit = resolve_observed_call_unit_id(
                unit_bindings,
                &participant_uids_by_reference_name,
                &call.callee,
            );
            let caller_interfaces = all_interfaces_for_unit_id(unit_bindings, &caller_unit);
            let callee_interfaces = all_interfaces_for_unit_id(unit_bindings, &callee_unit);

            SequenceCallContext {
                caller_unit,
                callee_unit,
                method: call.method.as_str(),
                caller_interfaces,
                callee_interfaces,
                source_location: &call.source_location,
            }
        })
        .collect()
}

pub(in crate::validators) fn resolve_observed_call_unit_id(
    unit_bindings: &UnitBindings,
    participant_uids_by_reference_name: &BTreeMap<&str, BTreeSet<&str>>,
    observed_unit: &str,
) -> String {
    if unit_bindings.contains_key(observed_unit) {
        return observed_unit.to_string();
    }

    if let Some(participant_uids) = participant_uids_by_reference_name.get(observed_unit) {
        if participant_uids.len() == 1 {
            return participant_uids
                .iter()
                .next()
                .expect("participant UID set length was checked")
                .to_string();
        }
    }

    observed_unit.to_string()
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

    #[test]
    fn units_sharing_an_alias_under_different_parents_keep_distinct_id_bindings() {
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
        ];

        let mut result = ValidationResult::default();
        let architecture = ComponentDiagramInputs { entities }.to_diagram_architecture(&mut result);
        assert!(
            result.is_empty(),
            "expected no failures, got {:?}",
            result.failures
        );

        // Different parents -> distinct `unit_set` entries.
        assert_eq!(architecture.unit_set.len(), 2);

        let bindings = build_unit_bindings(&architecture);
        assert_eq!(
            bindings["comp_a.unit_x"].all_interfaces,
            BTreeSet::from(["iface_a".to_string()])
        );
        assert_eq!(
            bindings["comp_b.unit_x"].all_interfaces,
            BTreeSet::from(["iface_b".to_string()])
        );
    }
}
