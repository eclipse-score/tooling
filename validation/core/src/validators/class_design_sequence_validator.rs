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

//! Validation: compare unit class-design entities with sequence-diagram usage.

use std::collections::BTreeSet;

use super::shared::{extract_method_name, format_sequence_call};
use crate::models::{
    ClassEntityIndex, ObservedSequenceCall, SequenceDiagramIndex, SequenceParticipantInfo,
};
use crate::{Diagnostics, ErrorBuilder, ErrorCategory, ValidationResult};
use class_diagram::{RelationType, Visibility};

/// Run class-design-vs-sequence validation.
pub fn validate_class_design_sequence(
    design_classes: &ClassEntityIndex,
    sequence_diagram: &SequenceDiagramIndex,
) -> ValidationResult {
    ClassDesignSequenceValidator::new(design_classes, sequence_diagram).run()
}

struct ClassDesignSequenceValidator<'a> {
    design_classes: &'a ClassEntityIndex,
    sequence_diagram: &'a SequenceDiagramIndex,
    result: ValidationResult,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MethodLookupResult {
    FoundAccessible,
    FoundPrivateInherited,
    NotFound,
}

impl<'a> ClassDesignSequenceValidator<'a> {
    fn new(
        design_classes: &'a ClassEntityIndex,
        sequence_diagram: &'a SequenceDiagramIndex,
    ) -> Self {
        Self {
            design_classes,
            sequence_diagram,
            result: ValidationResult::default(),
        }
    }

    fn run(mut self) -> ValidationResult {
        append_debug_log(
            &mut self.result.diagnostics,
            self.design_classes,
            self.sequence_diagram,
        );
        self.check_participant_class_consistency();
        self.check_message_operation_consistency();
        self.result
    }

    fn check_participant_class_consistency(&mut self) {
        for (participant, participant_info) in self.sequence_diagram.participants() {
            let (source_file, source_line) = participant_info.source_location.display();

            match self.resolve_participant_class(participant) {
                ParticipantResolution::Matched(_) => {}
                ParticipantResolution::Missing => {
                    self.result.add_failure(
                        ErrorBuilder::new(ErrorCategory::Class)
                            .title(format!(
                                "sequence participant \"{participant}\" has no matching class in the class diagram"
                            ))
                            .field("participant", format!("\"{participant}\""))
                            .field("sequence source file", format!("\"{source_file}\""))
                            .field("sequence source line", source_line.to_string())
                            .fix(format!(
                                "add class \"{participant}\" to the class diagram, or remove the participant from the sequence diagram"
                            ))
                            .build(),
                    );
                }
                ParticipantResolution::Ambiguous(matches) => {
                    let matching_classes = format_name_set(&matches);
                    self.result.add_failure(
                        ErrorBuilder::new(ErrorCategory::Class)
                            .title(format!(
                                "sequence participant \"{participant}\" matches multiple classes in the class diagram"
                            ))
                            .field("participant", format!("\"{participant}\""))
                            .field("matching classes", matching_classes.clone())
                            .field("sequence source file", format!("\"{source_file}\""))
                            .field("sequence source line", source_line.to_string())
                            .fix(format!(
                                "rename participant \"{participant}\" in the sequence diagram to a unique class id, or rename one of the matching classes in the class diagram"
                            ))
                            .build(),
                    );
                }
            }
        }
    }

    fn check_message_operation_consistency(&mut self) {
        for observed_call in self.sequence_diagram.observed_calls() {
            self.validate_observed_call(observed_call);
        }
    }

    fn validate_observed_call(&mut self, observed_call: &ObservedSequenceCall) {
        let ParticipantResolution::Matched(callee_class) =
            self.resolve_participant_class(&observed_call.callee)
        else {
            return;
        };

        let method_name = extract_method_name(&observed_call.method);
        if method_name.is_empty() {
            return;
        }

        let method_lookup = self.class_or_ancestors_define_method(
            callee_class,
            method_name,
            false,
            &mut BTreeSet::new(),
        );
        if method_lookup == MethodLookupResult::FoundAccessible {
            return;
        }

        self.result.add_failure(self.method_lookup_failure(
            observed_call,
            callee_class,
            method_name,
            method_lookup,
        ));
    }

    fn method_lookup_failure(
        &self,
        observed_call: &ObservedSequenceCall,
        callee_class: &'a class_diagram::SimpleEntity,
        method_name: &str,
        method_lookup: MethodLookupResult,
    ) -> String {
        let sequence_call =
            format_sequence_call(&observed_call.caller, &observed_call.callee, method_name);
        let (source_file, source_line) = observed_call.source_location.display();

        match method_lookup {
            MethodLookupResult::FoundAccessible => {
                unreachable!("accessible methods should return early")
            }
            MethodLookupResult::FoundPrivateInherited => ErrorBuilder::new(ErrorCategory::Method)
                .title(format!(
                    "sequence function \"{method_name}\" from sequence call {sequence_call} exists only as a private inherited method on target class \"{}\" in the class diagram",
                    callee_class.id,
                ))
                .field("sequence call", sequence_call)
                .field("target class", format!("\"{}\"", callee_class.id))
                .field("sequence source file", format!("\"{source_file}\""))
                .field("sequence source line", source_line.to_string())
                .fix(format!(
                    "consider changing method \"{method_name}\" to public or protected on an inherited type of class \"{}\", add an accessible wrapper on that class, or change or remove that sequence call",
                    callee_class.id,
                ))
                .build(),
            MethodLookupResult::NotFound => ErrorBuilder::new(ErrorCategory::Method)
                .title(format!(
                    "sequence function \"{method_name}\" from sequence call {sequence_call} not found on target class \"{}\" or its accessible inherited types in the class diagram",
                    callee_class.id,
                ))
                .field("sequence call", sequence_call)
                .field("target class", format!("\"{}\"", callee_class.id))
                .field("sequence source file", format!("\"{source_file}\""))
                .field("sequence source line", source_line.to_string())
                .fix(format!(
                    "add method \"{method_name}\" to class \"{}\" or one of its accessible inherited types in the class diagram, or change or remove that sequence call",
                    callee_class.id,
                ))
                .build(),
        }
    }

    fn resolve_participant_class(&self, participant: &str) -> ParticipantResolution<'a> {
        if let Some(entity) = self.resolve_by_class_id(participant) {
            return ParticipantResolution::Matched(entity);
        }

        if let Some(participant_info) = self.sequence_diagram.participant_info(participant) {
            let display_name = participant_info.display_name.as_str();

            if display_name != participant {
                if let Some(entity) = self.resolve_by_class_id(display_name) {
                    return ParticipantResolution::Matched(entity);
                }

                match self.resolve_by_class_name(display_name) {
                    ParticipantResolution::Missing => {}
                    matched_or_ambiguous => return matched_or_ambiguous,
                }
            }
        }

        self.resolve_by_class_name(participant)
    }

    fn resolve_by_class_id(&self, reference: &str) -> Option<&'a class_diagram::SimpleEntity> {
        if let Some(entity) = self.design_classes.find_by_id(reference) {
            return Some(entity);
        }

        let normalized_reference = SequenceParticipantInfo::normalize_qualified_name(reference);
        if normalized_reference == reference {
            return None;
        }

        self.design_classes.find_by_id(&normalized_reference)
    }

    fn resolve_by_class_name(&self, class_name: &str) -> ParticipantResolution<'a> {
        let short_name_matches: Vec<_> = self
            .design_classes
            .entities()
            .filter(|entity| entity.name == class_name)
            .collect();

        match short_name_matches.as_slice() {
            [] => ParticipantResolution::Missing,
            [entity] => ParticipantResolution::Matched(entity),
            entities => ParticipantResolution::Ambiguous(
                entities.iter().map(|entity| entity.id.clone()).collect(),
            ),
        }
    }

    fn class_or_ancestors_define_method(
        &self,
        entity: &'a class_diagram::SimpleEntity,
        method_name: &str,
        inherited: bool,
        visited_ids: &mut BTreeSet<String>,
    ) -> MethodLookupResult {
        let local_result = Self::method_lookup_on_entity(entity, method_name, inherited);
        if local_result != MethodLookupResult::NotFound {
            return local_result;
        }

        if !visited_ids.insert(entity.id.clone()) {
            return MethodLookupResult::NotFound;
        }

        self.related_parent_or_interface_defines_method(entity, method_name, visited_ids)
    }

    fn method_lookup_on_entity(
        entity: &'a class_diagram::SimpleEntity,
        method_name: &str,
        inherited: bool,
    ) -> MethodLookupResult {
        let mut found_private_inherited = false;

        for method in &entity.methods {
            if method.name != method_name {
                continue;
            }

            if inherited && matches!(method.visibility, Visibility::Private) {
                found_private_inherited = true;
                continue;
            }

            return MethodLookupResult::FoundAccessible;
        }

        if found_private_inherited {
            MethodLookupResult::FoundPrivateInherited
        } else {
            MethodLookupResult::NotFound
        }
    }

    fn related_parent_or_interface_defines_method(
        &self,
        entity: &'a class_diagram::SimpleEntity,
        method_name: &str,
        visited_ids: &mut BTreeSet<String>,
    ) -> MethodLookupResult {
        let mut found_private_inherited = false;

        for relationship in &entity.relationships {
            if relationship.source != entity.id
                || !matches!(
                    relationship.relation_type,
                    RelationType::Inheritance | RelationType::Implementation
                )
            {
                continue;
            }

            let Some(parent) = self.design_classes.find_by_id(&relationship.target) else {
                continue;
            };

            match self.class_or_ancestors_define_method(parent, method_name, true, visited_ids) {
                MethodLookupResult::FoundAccessible => return MethodLookupResult::FoundAccessible,
                MethodLookupResult::FoundPrivateInherited => found_private_inherited = true,
                MethodLookupResult::NotFound => {}
            }
        }

        if found_private_inherited {
            MethodLookupResult::FoundPrivateInherited
        } else {
            MethodLookupResult::NotFound
        }
    }
}

enum ParticipantResolution<'a> {
    Matched(&'a class_diagram::SimpleEntity),
    Missing,
    Ambiguous(BTreeSet<String>),
}

fn format_name_set(names: &BTreeSet<String>) -> String {
    names
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

fn append_debug_log(
    diagnostics: &mut Diagnostics,
    design_classes: &ClassEntityIndex,
    sequence_diagram: &SequenceDiagramIndex,
) {
    diagnostics.debug(|| "Design classes available for sequence validation:".to_string());
    for entity in design_classes.entities() {
        diagnostics.debug(|| format!("  {}", entity.id));
    }

    diagnostics.debug(|| "Observed participants from sequence diagrams:".to_string());
    for participant in sequence_diagram.declared_participants() {
        diagnostics.debug(|| format!("  {participant}"));
    }

    diagnostics.debug(|| "Observed sequence calls from sequence diagrams:".to_string());
    for observed_call in sequence_diagram.observed_calls() {
        diagnostics.debug(|| {
            format!(
                "  {} -> {} : {}",
                observed_call.caller, observed_call.callee, observed_call.method
            )
        });
    }
}

#[cfg(test)]
#[path = "test/class_design_sequence_validator_test.rs"]
mod tests;
