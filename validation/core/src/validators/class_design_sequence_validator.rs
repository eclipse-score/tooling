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

use super::shared::{
    best_string_suggestion, display_entity_name, display_name_from_source_path_in_context,
    display_reference_name, extract_method_name, format_sequence_call,
};
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
            self.validate_participant(participant, participant_info);
        }
    }

    fn validate_participant(
        &mut self,
        participant: &str,
        participant_info: &SequenceParticipantInfo,
    ) {
        let (source_file, source_line) = participant_info.source_location.display();

        if !looks_like_class_identifier(participant) {
            self.result
                .add_failure(self.invalid_participant_identifier_failure(
                    participant,
                    &source_file,
                    source_line,
                ));
            return;
        }

        match self.resolve_participant_class(participant) {
            ParticipantResolution::Matched(_) => {}
            ParticipantResolution::Missing => {
                self.result.add_failure(self.missing_participant_failure(
                    participant,
                    participant_info,
                    &source_file,
                    source_line,
                ))
            }
        }
    }

    fn invalid_participant_identifier_failure(
        &self,
        participant: &str,
        source_file: &str,
        source_line: u32,
    ) -> String {
        let display_participant =
            display_sequence_participant_name_from_source(participant, source_file);

        ErrorBuilder::new(ErrorCategory::Class)
            .title(format!(
                "sequence participant \"{display_participant}\" does not provide a valid class identifier for class-design validation"
            ))
            .field("participant", format!("\"{participant}\""))
            .field("sequence source file", format!("\"{source_file}\""))
            .field("sequence source line", source_line.to_string())
            .fix(format!(
                "set the participant uid to a class id such as \"{display_participant}\", or change the participant declaration to use a valid class identifier"
            ))
            .build()
    }

    fn missing_participant_failure(
        &self,
        participant: &str,
        _participant_info: &SequenceParticipantInfo,
        source_file: &str,
        source_line: u32,
    ) -> String {
        let display_participant =
            display_sequence_participant_name_from_source(participant, source_file);
        let suggested_class = self.best_participant_class_suggestion(participant);
        let display_suggested_class = suggested_class.map(display_suggested_class_name);
        let missing_class_name = suggested_class
            .filter(|entity| self.should_use_suggested_class_in_fix(&display_participant, entity))
            .map(display_suggested_class_name)
            .unwrap_or_else(|| display_participant.clone());
        let error = ErrorBuilder::new(ErrorCategory::Class)
            .title(format!(
                "sequence participant \"{display_participant}\" has no matching class in the class diagram"
            ))
            .field("participant", format!("\"{display_participant}\""))
            .field("sequence source file", format!("\"{source_file}\""))
            .field("sequence source line", source_line.to_string())
            .fix(format!(
                "add class \"{missing_class_name}\" to the class diagram, or remove the participant from the sequence diagram"
            ));

        if let Some(display_suggested_class) = display_suggested_class.as_deref() {
            error.suggest(&display_participant, Some("class"), display_suggested_class)
        } else {
            error
        }
        .build()
    }

    fn should_use_suggested_class_in_fix(
        &self,
        display_participant: &str,
        suggested_class: &class_diagram::SimpleEntity,
    ) -> bool {
        suggested_class.name == display_participant
            && self
                .design_classes
                .entities()
                .filter(|entity| entity.name == display_participant)
                .take(2)
                .count()
                == 1
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
        let sequence_call = format_sequence_call(
            display_reference_name(&observed_call.caller),
            display_reference_name(&observed_call.callee),
            method_name,
        );
        let target_class = display_entity_name(callee_class);
        let (source_file, source_line) = observed_call.source_location.display();

        match method_lookup {
            MethodLookupResult::FoundAccessible => {
                unreachable!("accessible methods should return early")
            }
            MethodLookupResult::FoundPrivateInherited => ErrorBuilder::new(ErrorCategory::Method)
                .title(format!(
                    "sequence function \"{method_name}\" from sequence call {sequence_call} exists only as a private inherited method on target class \"{}\" in the class diagram",
                    target_class,
                ))
                .field("sequence call", sequence_call)
                .field("target class", format!("\"{}\"", target_class))
                .field("sequence source file", format!("\"{source_file}\""))
                .field("sequence source line", source_line.to_string())
                .fix(format!(
                    "consider changing method \"{method_name}\" to public or protected on an inherited type of class \"{}\", add an accessible wrapper on that class, or change or remove that sequence call",
                    target_class,
                ))
                .build(),
            MethodLookupResult::NotFound => {
                let error = ErrorBuilder::new(ErrorCategory::Method)
                .title(format!(
                    "sequence function \"{method_name}\" from sequence call {sequence_call} not found on target class \"{}\" or its accessible inherited types in the class diagram",
                    target_class,
                ))
                .field("sequence call", sequence_call)
                .field("target class", format!("\"{}\"", target_class))
                .field("sequence source file", format!("\"{source_file}\""))
                .field("sequence source line", source_line.to_string())
                .fix(format!(
                    "add method \"{method_name}\" to class \"{}\" or one of its accessible inherited types in the class diagram, or change or remove that sequence call",
                    target_class,
                ));

                if let Some(suggested_method) = self
                    .best_method_suggestion(callee_class, method_name)
                    .as_deref()
                {
                    error.suggest(method_name, Some("method"), suggested_method)
                } else {
                    error
                }
                .build()
            }
        }
    }

    fn best_participant_class_suggestion(
        &self,
        participant: &str,
    ) -> Option<&'a class_diagram::SimpleEntity> {
        let class_candidates: BTreeSet<String> = self
            .design_classes
            .entities()
            .map(|entity| entity.id.clone())
            .filter(|candidate| !candidate.is_empty())
            .collect();

        participant_suggestion_queries(participant)
            .into_iter()
            .find_map(|query| {
                best_string_suggestion(&query, class_candidates.iter().map(String::as_str))
                    .and_then(|suggested_id| self.design_classes.find_by_id(&suggested_id))
            })
    }

    fn best_method_suggestion(
        &self,
        callee_class: &'a class_diagram::SimpleEntity,
        method_name: &str,
    ) -> Option<String> {
        let mut visited_ids = BTreeSet::new();
        let mut method_candidates = BTreeSet::new();
        self.collect_related_method_names(callee_class, &mut visited_ids, &mut method_candidates);

        best_string_suggestion(method_name, method_candidates.iter().map(String::as_str))
    }

    fn resolve_participant_class(&self, participant: &str) -> ParticipantResolution<'a> {
        if let Some(resolution) = self.resolve_class_from_participant(participant) {
            return resolution;
        }

        ParticipantResolution::Missing
    }

    fn resolve_class_from_participant(
        &self,
        participant: &str,
    ) -> Option<ParticipantResolution<'a>> {
        self.resolve_by_class_id(participant)
            .map(ParticipantResolution::Matched)
    }

    fn resolve_by_class_id(&self, reference: &str) -> Option<&'a class_diagram::SimpleEntity> {
        if let Some(entity) = self.design_classes.find_by_id(reference) {
            return Some(entity);
        }

        let normalized_reference = SequenceParticipantInfo::normalize_qualified_name(reference);
        if normalized_reference != reference {
            return self.design_classes.find_by_id(&normalized_reference);
        }

        None
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

    fn collect_related_method_names(
        &self,
        entity: &'a class_diagram::SimpleEntity,
        visited_ids: &mut BTreeSet<String>,
        method_candidates: &mut BTreeSet<String>,
    ) {
        if !visited_ids.insert(entity.id.clone()) {
            return;
        }

        method_candidates.extend(
            entity
                .methods
                .iter()
                .map(|method| method.name.as_str())
                .filter(|name| !name.is_empty())
                .map(str::to_string),
        );

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

            self.collect_related_method_names(parent, visited_ids, method_candidates);
        }
    }
}

enum ParticipantResolution<'a> {
    Matched(&'a class_diagram::SimpleEntity),
    Missing,
}

fn display_sequence_participant_name(participant: &str) -> String {
    let normalized_participant = SequenceParticipantInfo::normalize_qualified_name(participant);

    if looks_like_qualified_class_id(&normalized_participant) {
        display_reference_name(&normalized_participant).to_string()
    } else {
        participant.to_string()
    }
}

fn display_sequence_participant_name_from_source(participant: &str, source_file: &str) -> String {
    let normalized_participant = SequenceParticipantInfo::normalize_qualified_name(participant);

    if looks_like_qualified_class_id(&normalized_participant) {
        display_name_from_source_path_in_context(
            &normalized_participant,
            source_file,
            std::iter::empty(),
        )
    } else {
        participant.to_string()
    }
}

fn display_suggested_class_name(entity: &class_diagram::SimpleEntity) -> String {
    let normalized_identifier = SequenceParticipantInfo::normalize_qualified_name(&entity.id);
    let segments = normalized_identifier
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    match segments.as_slice() {
        [] => String::new(),
        [leaf] => (*leaf).to_string(),
        [.., namespace, leaf] if should_display_namespace_segment(namespace) => {
            format!("{namespace}.{leaf}")
        }
        [.., leaf] => (*leaf).to_string(),
    }
}

fn should_display_namespace_segment(segment: &str) -> bool {
    segment
        .chars()
        .any(|character| character.is_ascii_uppercase())
        || segment.chars().any(|character| character.is_ascii_digit())
        || !segment.contains('_')
}

fn looks_like_qualified_class_id(reference: &str) -> bool {
    let segments = reference
        .trim()
        .split(['.', ':'])
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    segments.len() > 1
        && segments
            .iter()
            .all(|segment| is_identifier_segment(segment))
}

fn looks_like_class_identifier(reference: &str) -> bool {
    let segments = reference
        .trim()
        .split(['.', ':'])
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    !segments.is_empty()
        && segments
            .iter()
            .all(|segment| is_identifier_segment(segment))
}

fn is_identifier_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn participant_suggestion_queries(participant: &str) -> Vec<String> {
    let mut queries = BTreeSet::new();

    insert_participant_suggestion_query(&mut queries, participant);

    let display_participant = display_sequence_participant_name(participant);
    if display_participant != participant {
        insert_participant_suggestion_query(&mut queries, &display_participant);
    }

    queries.into_iter().collect()
}

fn insert_participant_suggestion_query(queries: &mut BTreeSet<String>, query: &str) {
    if query.is_empty() {
        return;
    }

    queries.insert(query.to_string());

    let normalized_query = SequenceParticipantInfo::normalize_qualified_name(query);
    if normalized_query != query {
        queries.insert(normalized_query);
    }
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
