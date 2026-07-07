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

use crate::models::{
    ComponentDiagramArchitecture, ComponentRelationType, EndpointRole, LogicComponentExt,
    ObservedSequenceCall,
};

pub(in crate::validators) type UnitBindings = BTreeMap<String, UnitInterfaces>;

#[derive(Clone, Default)]
pub(in crate::validators) struct UnitInterfaces {
    pub(in crate::validators) all_interfaces: BTreeSet<String>,
    pub(in crate::validators) required_interfaces: BTreeSet<String>,
    pub(in crate::validators) provided_interfaces: BTreeSet<String>,
}

#[derive(Clone)]
pub(in crate::validators) struct SequenceCallContext<'a> {
    pub(in crate::validators) caller_unit: &'a str,
    pub(in crate::validators) callee_unit: &'a str,
    pub(in crate::validators) method: &'a str,
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

    for entity in component_diagram
        .entities
        .iter()
        .filter(|entity| entity.is_unit())
    {
        let Some(alias) = entity.alias.clone() else {
            continue;
        };

        let mut bindings = UnitInterfaces::default();

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

        unit_bindings.insert(alias, bindings);
    }

    unit_bindings
}

pub(in crate::validators) fn all_interfaces_for_alias(
    unit_bindings: &UnitBindings,
    alias: &str,
) -> BTreeSet<String> {
    unit_bindings
        .get(alias)
        .map(|bindings| bindings.all_interfaces.clone())
        .unwrap_or_default()
}

pub(in crate::validators) fn build_observed_call_contexts<'a>(
    observed_calls: &'a [ObservedSequenceCall],
    unit_bindings: &UnitBindings,
) -> Vec<SequenceCallContext<'a>> {
    observed_calls
        .iter()
        .map(|call| {
            let caller_interfaces = all_interfaces_for_alias(unit_bindings, &call.caller);
            let callee_interfaces = all_interfaces_for_alias(unit_bindings, &call.callee);

            SequenceCallContext {
                caller_unit: call.caller.as_str(),
                callee_unit: call.callee.as_str(),
                method: call.method.as_str(),
                caller_interfaces,
                callee_interfaces,
            }
        })
        .collect()
}
