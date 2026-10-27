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
    ComponentDiagramArchitecture, ComponentRelationType, EndpointRole, Environment,
    LogicComponentExt, ObservedSequenceCall,
};

/// Unit interface bindings keyed by component id.
pub(in crate::validators) type UnitBindings = BTreeMap<String, UnitInterfaces>;

/// Unit ids contained in each component or SEooC, keyed by element id.
pub(in crate::validators) type ContainerUnits = BTreeMap<String, BTreeSet<String>>;

#[derive(Clone, Default)]
pub(in crate::validators) struct UnitInterfaces {
    pub(in crate::validators) source_location: Option<SourceLocation>,
    pub(in crate::validators) all_interfaces: BTreeSet<String>,
    pub(in crate::validators) required_interfaces: BTreeSet<String>,
    pub(in crate::validators) provided_interfaces: BTreeSet<String>,
}

/// One sequence call with caller and callee mapped to architecture element
/// ids (unit, component or SEooC). A participant that does not resolve to
/// exactly one element keeps its uid.
#[derive(Clone)]
pub(in crate::validators) struct SequenceCallContext {
    pub(in crate::validators) caller_unit: String,
    pub(in crate::validators) callee_unit: String,
    pub(in crate::validators) caller_environment: Option<Environment>,
    pub(in crate::validators) callee_environment: Option<Environment>,
    /// An end is a component or SEooC lifeline rather than a unit.
    pub(in crate::validators) involves_container: bool,
    /// Units behind the caller: itself for a unit, the contained units for a
    /// component or SEooC lifeline, empty otherwise.
    pub(in crate::validators) caller_contained_units: BTreeSet<String>,
    pub(in crate::validators) callee_contained_units: BTreeSet<String>,
    pub(in crate::validators) method: String,
    pub(in crate::validators) source_location: SourceLocation,
    pub(in crate::validators) caller_interfaces: BTreeSet<String>,
    pub(in crate::validators) callee_interfaces: BTreeSet<String>,
    /// Interfaces the caller binds as required, merged over its units.
    pub(in crate::validators) caller_required_interfaces: BTreeSet<String>,
    /// Interfaces the callee binds as provided, merged over its units.
    pub(in crate::validators) callee_provided_interfaces: BTreeSet<String>,
}

impl SequenceCallContext {
    pub(in crate::validators) fn has_shared_interfaces(&self) -> bool {
        !self.caller_interfaces.is_disjoint(&self.callee_interfaces)
    }

    /// A call with an actor, external system or gate on either end.
    pub(in crate::validators) fn involves_environment(&self) -> bool {
        self.caller_environment.is_some() || self.callee_environment.is_some()
    }

    /// One end contains the other (or is the other): not a call across
    /// element boundaries.
    pub(in crate::validators) fn ends_overlap(&self) -> bool {
        !self
            .caller_contained_units
            .is_disjoint(&self.callee_contained_units)
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
fn resolve_unit<'a>(unit_bindings: &'a UnitBindings, uid: &str) -> UidMatch<'a> {
    resolve_uid(uid, unit_bindings.keys().map(String::as_str))
}

/// Interfaces of `unit_ids`, merged.
fn merged_interfaces<'a>(
    unit_bindings: &UnitBindings,
    unit_ids: impl IntoIterator<Item = &'a String>,
) -> UnitInterfaces {
    let mut merged = UnitInterfaces::default();

    for bindings in unit_ids.into_iter().filter_map(|id| unit_bindings.get(id)) {
        merged
            .all_interfaces
            .extend(bindings.all_interfaces.iter().cloned());
        merged
            .required_interfaces
            .extend(bindings.required_interfaces.iter().cloned());
        merged
            .provided_interfaces
            .extend(bindings.provided_interfaces.iter().cloned());
    }

    merged
}

/// Units contained in each component and SEooC, by id prefix. Elements
/// without units are omitted.
pub(in crate::validators) fn build_container_units(
    component_diagram: &ComponentDiagramArchitecture,
) -> ContainerUnits {
    let unit_ids: Vec<&str> = component_diagram
        .unit_set
        .values()
        .map(|unit| unit.id.as_str())
        .collect();

    component_diagram
        .comp_set
        .values()
        .chain(component_diagram.seooc_set.values())
        .filter_map(|container| {
            let prefix = format!("{}.", container.id);
            let contained: BTreeSet<String> = unit_ids
                .iter()
                .filter(|unit_id| unit_id.starts_with(&prefix))
                .map(|unit_id| unit_id.to_string())
                .collect();

            (!contained.is_empty()).then(|| (container.id.clone(), contained))
        })
        .collect()
}

/// Matches a sequence participant uid against the component and SEooC ids.
fn resolve_container<'a>(container_units: &'a ContainerUnits, uid: &str) -> UidMatch<'a> {
    resolve_uid(uid, container_units.keys().map(String::as_str))
}

/// What a sequence participant uid names in the component diagram.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::validators) enum ElementMatch<'a> {
    Unit(&'a str),
    /// A component or SEooC.
    Container(&'a str),
    /// Several units and/or components match; the ids are sorted.
    Ambiguous(Vec<&'a str>),
    Unresolved,
}

/// A uid that matches both a unit and a component is ambiguous.
pub(in crate::validators) fn match_element<'a>(
    unit_bindings: &'a UnitBindings,
    container_units: &'a ContainerUnits,
    uid: &str,
) -> ElementMatch<'a> {
    let candidates = |matched: UidMatch<'a>| match matched {
        UidMatch::Resolved(id) => vec![id],
        UidMatch::Ambiguous(ids) => ids,
        UidMatch::Unresolved => Vec::new(),
    };

    let units = resolve_unit(unit_bindings, uid);
    let containers = resolve_container(container_units, uid);

    match (units, containers) {
        (UidMatch::Resolved(unit_id), UidMatch::Unresolved) => ElementMatch::Unit(unit_id),
        (UidMatch::Unresolved, UidMatch::Resolved(container_id)) => {
            ElementMatch::Container(container_id)
        }
        (UidMatch::Unresolved, UidMatch::Unresolved) => ElementMatch::Unresolved,
        (units, containers) => {
            let mut ids = candidates(units);
            ids.extend(candidates(containers));
            ids.sort_unstable();
            ElementMatch::Ambiguous(ids)
        }
    }
}

/// An architecture element a participant resolved to.
struct ResolvedElement {
    id: String,
    is_container: bool,
    contained_units: BTreeSet<String>,
    interfaces: UnitInterfaces,
}

fn resolve_element(
    unit_bindings: &UnitBindings,
    container_units: &ContainerUnits,
    uid: &str,
) -> ResolvedElement {
    match match_element(unit_bindings, container_units, uid) {
        ElementMatch::Unit(unit_id) => {
            let contained_units = BTreeSet::from([unit_id.to_string()]);
            let interfaces = merged_interfaces(unit_bindings, &contained_units);

            ResolvedElement {
                id: unit_id.to_string(),
                is_container: false,
                contained_units,
                interfaces,
            }
        }
        ElementMatch::Container(container_id) => {
            let contained_units = container_units[container_id].clone();
            let interfaces = merged_interfaces(unit_bindings, &contained_units);

            ResolvedElement {
                id: container_id.to_string(),
                is_container: true,
                contained_units,
                interfaces,
            }
        }
        ElementMatch::Ambiguous(_) | ElementMatch::Unresolved => unresolved_element(uid),
    }
}

fn unresolved_element(uid: &str) -> ResolvedElement {
    ResolvedElement {
        id: uid.to_string(),
        is_container: false,
        contained_units: BTreeSet::new(),
        interfaces: UnitInterfaces::default(),
    }
}

pub(in crate::validators) fn build_observed_call_contexts(
    observed_calls: &[ObservedSequenceCall],
    unit_bindings: &UnitBindings,
    container_units: &ContainerUnits,
) -> Vec<SequenceCallContext> {
    let resolve = |uid: &str, environment: Option<Environment>| {
        if environment.is_some() {
            unresolved_element(uid)
        } else {
            resolve_element(unit_bindings, container_units, uid)
        }
    };

    observed_calls
        .iter()
        .map(|call| {
            let caller = resolve(&call.caller, call.caller_environment);
            let callee = resolve(&call.callee, call.callee_environment);

            SequenceCallContext {
                involves_container: caller.is_container || callee.is_container,
                caller_unit: caller.id,
                callee_unit: callee.id,
                caller_environment: call.caller_environment,
                callee_environment: call.callee_environment,
                caller_contained_units: caller.contained_units,
                callee_contained_units: callee.contained_units,
                method: call.method.clone(),
                source_location: call.source_location.clone(),
                caller_interfaces: caller.interfaces.all_interfaces,
                callee_interfaces: callee.interfaces.all_interfaces,
                caller_required_interfaces: caller.interfaces.required_interfaces,
                callee_provided_interfaces: callee.interfaces.provided_interfaces,
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
        observed_call_with_environment(caller, None, callee, None)
    }

    fn observed_call_with_environment(
        caller: &str,
        caller_environment: Option<Environment>,
        callee: &str,
        callee_environment: Option<Environment>,
    ) -> ObservedSequenceCall {
        ObservedSequenceCall {
            caller: caller.to_string(),
            callee: callee.to_string(),
            caller_environment,
            callee_environment,
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

        let contexts = build_observed_call_contexts(
            &[observed_call("comp_a.unit_x", "unit_y")],
            &bindings,
            &ContainerUnits::new(),
        );

        assert_eq!(contexts[0].caller_unit, "comp_a.unit_x");
        assert_eq!(contexts[0].callee_unit, "comp_b.unit_y");
        assert_eq!(
            contexts[0].caller_interfaces,
            BTreeSet::from(["iface_a".to_string()])
        );
    }

    #[test]
    fn call_contexts_keep_ambiguous_unresolved_and_environment_uids() {
        let bindings = build_unit_bindings(&two_parents_with_unit_x());

        let contexts = build_observed_call_contexts(
            &[
                observed_call("unit_x", "missing"),
                observed_call_with_environment(
                    "operator",
                    Some(Environment::Actor),
                    "unit_y",
                    None,
                ),
            ],
            &bindings,
            &ContainerUnits::new(),
        );

        assert_eq!(contexts[0].caller_unit, "unit_x");
        assert!(contexts[0].caller_interfaces.is_empty());
        assert_eq!(contexts[0].callee_unit, "missing");
        assert_eq!(contexts[1].caller_unit, "operator");
        assert!(contexts[1].caller_contained_units.is_empty());
        assert!(contexts[1].involves_environment());
        assert_eq!(contexts[1].callee_unit, "comp_b.unit_y");
    }

    #[test]
    fn container_lifelines_resolve_to_contained_units() {
        let architecture = two_parents_with_unit_x();
        let bindings = build_unit_bindings(&architecture);
        let containers = build_container_units(&architecture);

        assert_eq!(
            containers["comp_b"],
            BTreeSet::from(["comp_b.unit_x".to_string(), "comp_b.unit_y".to_string()])
        );

        let contexts = build_observed_call_contexts(
            &[observed_call("comp_b", "comp_a.unit_x")],
            &bindings,
            &containers,
        );

        assert_eq!(contexts[0].caller_unit, "comp_b");
        assert_eq!(contexts[0].caller_contained_units.len(), 2);
        assert_eq!(
            contexts[0].caller_interfaces,
            BTreeSet::from(["iface_b".to_string()])
        );
        assert_eq!(
            contexts[0].caller_required_interfaces,
            BTreeSet::from(["iface_b".to_string()])
        );
        assert_eq!(
            contexts[0].callee_provided_interfaces,
            BTreeSet::from(["iface_a".to_string()])
        );
        assert!(contexts[0].involves_container);
        assert!(!contexts[0].ends_overlap());
    }

    #[test]
    fn uid_matching_a_unit_and_a_component_is_ambiguous() {
        let mut entities = vec![
            entity(
                "comp_a",
                "comp_a",
                None,
                ComponentType::Component,
                Some("component"),
                Vec::new(),
            ),
            entity(
                "comp_a.shared",
                "shared",
                Some("comp_a"),
                ComponentType::Component,
                Some("unit"),
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
                "comp_b.shared",
                "shared",
                Some("comp_b"),
                ComponentType::Component,
                Some("component"),
                Vec::new(),
            ),
            entity(
                "comp_b.shared.inner",
                "inner",
                Some("comp_b.shared"),
                ComponentType::Component,
                Some("unit"),
                Vec::new(),
            ),
        ];
        entities.sort_by(|left, right| left.id.cmp(&right.id));
        let mut result = ValidationResult::default();
        let architecture = ComponentDiagramInputs { entities }.to_diagram_architecture(&mut result);
        let bindings = build_unit_bindings(&architecture);
        let containers = build_container_units(&architecture);

        assert_eq!(
            match_element(&bindings, &containers, "shared"),
            ElementMatch::Ambiguous(vec!["comp_a.shared", "comp_b.shared"])
        );
        assert_eq!(
            match_element(&bindings, &containers, "comp_a.shared"),
            ElementMatch::Unit("comp_a.shared")
        );
        assert_eq!(
            match_element(&bindings, &containers, "comp_b.shared"),
            ElementMatch::Container("comp_b.shared")
        );

        let contexts = build_observed_call_contexts(
            &[observed_call("shared", "comp_a.shared")],
            &bindings,
            &containers,
        );
        assert_eq!(contexts[0].caller_unit, "shared");
        assert!(!contexts[0].involves_container);
    }

    #[test]
    fn container_and_its_own_unit_overlap() {
        let architecture = two_parents_with_unit_x();
        let bindings = build_unit_bindings(&architecture);
        let containers = build_container_units(&architecture);

        let contexts = build_observed_call_contexts(
            &[observed_call("comp_b", "comp_b.unit_y")],
            &bindings,
            &containers,
        );

        assert!(contexts[0].ends_overlap());
    }
}
