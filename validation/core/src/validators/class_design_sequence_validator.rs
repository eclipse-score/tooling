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

use super::shared::{best_string_suggestion, extract_method_name, format_sequence_call};
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

        if let Some(display_issue) = unsupported_participant_display_form(participant_info) {
            self.report_unsupported_participant_display_name(
                participant,
                participant_info,
                &source_file,
                source_line,
                display_issue,
            );
            return;
        }

        self.log_ignored_special_display_suffix(
            participant,
            participant_info,
            &source_file,
            source_line,
        );

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
            ParticipantResolution::Ambiguous(matches) => {
                self.result.add_failure(self.ambiguous_participant_failure(
                    participant,
                    &source_file,
                    source_line,
                    &matches,
                ))
            }
        }
    }

    fn log_ignored_special_display_suffix(
        &self,
        participant: &str,
        participant_info: &SequenceParticipantInfo,
        source_file: &str,
        source_line: u32,
    ) {
        if let Some(ignored_suffix) = ignored_special_display_suffix(participant_info) {
            log::warn!(
                "sequence participant \"{}\" ignores trailing special display text \"{}\" at {}:{}; it is not treated as namespace or class matching data",
                participant,
                ignored_suffix,
                source_file,
                source_line,
            );
        }
    }

    fn missing_participant_failure(
        &self,
        participant: &str,
        participant_info: &SequenceParticipantInfo,
        source_file: &str,
        source_line: u32,
    ) -> String {
        let error = ErrorBuilder::new(ErrorCategory::Class)
            .title(format!(
                "sequence participant \"{participant}\" has no matching class in the class diagram"
            ))
            .field("participant", format!("\"{participant}\""))
            .field("sequence source file", format!("\"{source_file}\""))
            .field("sequence source line", source_line.to_string())
            .fix(format!(
                "add class \"{participant}\" to the class diagram, or remove the participant from the sequence diagram"
            ));

        if let Some(suggested_class) = self
            .best_participant_class_suggestion(participant, participant_info)
            .as_deref()
        {
            error.suggest(participant, Some("class"), suggested_class)
        } else {
            error
        }
        .build()
    }

    fn ambiguous_participant_failure(
        &self,
        participant: &str,
        source_file: &str,
        source_line: u32,
        matches: &BTreeSet<String>,
    ) -> String {
        ErrorBuilder::new(ErrorCategory::Class)
            .title(format!(
                "sequence participant \"{participant}\" matches multiple classes in the class diagram"
            ))
            .field("participant", format!("\"{participant}\""))
            .field("matching classes", format_name_set(matches))
            .field("sequence source file", format!("\"{source_file}\""))
            .field("sequence source line", source_line.to_string())
            .fix(format!(
                "rename participant \"{participant}\" in the sequence diagram to a unique class id, or rename one of the matching classes in the class diagram"
            ))
            .build()
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
            MethodLookupResult::NotFound => {
                let error = ErrorBuilder::new(ErrorCategory::Method)
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
        participant_info: &SequenceParticipantInfo,
    ) -> Option<String> {
        let class_candidates: BTreeSet<String> = self
            .design_classes
            .entities()
            .flat_map(|entity| [entity.id.clone(), entity.name.clone()])
            .filter(|candidate| !candidate.is_empty())
            .collect();

        participant_suggestion_queries(participant, participant_info)
            .into_iter()
            .find_map(|query| {
                best_string_suggestion(&query, class_candidates.iter().map(String::as_str))
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

        if let Some(participant_info) = self.sequence_diagram.participant_info(participant) {
            if let Some(resolution) = self.resolve_class_from_display_name(
                participant,
                participant_info.display_name.as_str(),
            ) {
                return resolution;
            }

            if let Some(resolution) = self.resolve_class_from_special_display_form(participant_info)
            {
                return resolution;
            }
        }

        self.resolve_by_class_name(participant)
    }

    fn resolve_class_from_participant(
        &self,
        participant: &str,
    ) -> Option<ParticipantResolution<'a>> {
        self.resolve_by_class_id(participant)
            .map(ParticipantResolution::Matched)
    }

    fn resolve_class_from_display_name(
        &self,
        participant: &str,
        display_name: &str,
    ) -> Option<ParticipantResolution<'a>> {
        if display_name == participant {
            return None;
        }

        if let Some(entity) = self.resolve_by_class_id(display_name) {
            return Some(ParticipantResolution::Matched(entity));
        }

        match self.resolve_by_class_name(display_name) {
            ParticipantResolution::Missing => None,
            matched_or_ambiguous => Some(matched_or_ambiguous),
        }
    }

    fn resolve_class_from_special_display_form(
        &self,
        participant_info: &SequenceParticipantInfo,
    ) -> Option<ParticipantResolution<'a>> {
        let display_candidates = class_match_candidates_from_display(participant_info);

        for id_candidate in display_candidates.id_candidates {
            if let Some(entity) = self.resolve_by_class_id(&id_candidate) {
                return Some(ParticipantResolution::Matched(entity));
            }
        }

        for name_candidate in display_candidates.name_candidates {
            match self.resolve_by_class_name(&name_candidate) {
                ParticipantResolution::Missing => {}
                matched_or_ambiguous => return Some(matched_or_ambiguous),
            }
        }

        None
    }

    fn report_unsupported_participant_display_name(
        &mut self,
        participant: &str,
        participant_info: &SequenceParticipantInfo,
        source_file: &str,
        source_line: u32,
        display_issue: UnsupportedParticipantDisplayForm,
    ) {
        self.result.add_failure(
            ErrorBuilder::new(ErrorCategory::Class)
                .title(format!(
                    "sequence participant \"{participant}\" uses an invalid kind of display name"
                ))
                .field("participant", format!("\"{participant}\""))
                .field("display name", format!("\"{}\"", participant_info.display_name))
                .field(
                    "invalid form",
                    display_issue.invalid_form(&participant_info.display_name),
                )
                .field("sequence source file", format!("\"{source_file}\""))
                .field("sequence source line", source_line.to_string())
                .fix(
                    "use one supported form such as :Name, prefix:qualified::Type, or provide an unambiguous alias"
                        .to_string(),
                )
                .build(),
        );
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
    Ambiguous(BTreeSet<String>),
}

#[derive(Default)]
struct ClassMatchCandidates {
    id_candidates: Vec<String>,
    name_candidates: Vec<String>,
}

#[derive(Clone, Copy)]
enum UnsupportedParticipantDisplayForm {
    MultipleStandaloneColons,
    EmptyColonSuffix,
}

impl UnsupportedParticipantDisplayForm {
    fn invalid_form(self, display_name: &str) -> String {
        match self {
            Self::MultipleStandaloneColons => format!(
                "\"{}\" contains multiple standalone ':' separators",
                display_name
            ),
            Self::EmptyColonSuffix => {
                format!(
                    "\"{}\" uses ':' without a non-empty right-hand side",
                    display_name
                )
            }
        }
    }
}

fn unsupported_participant_display_form(
    participant_info: &SequenceParticipantInfo,
) -> Option<UnsupportedParticipantDisplayForm> {
    let primary_line = first_nonempty_display_line(&participant_info.display_name)?;
    let separator_colons = separator_colon_positions(primary_line);

    if separator_colons.len() > 1 {
        return Some(UnsupportedParticipantDisplayForm::MultipleStandaloneColons);
    }

    if let Some(colon_index) = separator_colons.first() {
        if primary_line[colon_index + 1..].trim().is_empty() {
            return Some(UnsupportedParticipantDisplayForm::EmptyColonSuffix);
        }
    }

    None
}

fn class_match_candidates_from_display(
    participant_info: &SequenceParticipantInfo,
) -> ClassMatchCandidates {
    let Some(primary_line) = first_nonempty_display_line(&participant_info.display_name) else {
        return ClassMatchCandidates::default();
    };

    let separator_colons = separator_colon_positions(primary_line);
    if separator_colons.len() != 1 {
        return ClassMatchCandidates::default();
    }

    let colon_index = separator_colons[0];
    if colon_index == 0 {
        let short_name = primary_line[1..].trim();
        if short_name.is_empty() {
            return ClassMatchCandidates::default();
        }

        return ClassMatchCandidates {
            id_candidates: Vec::new(),
            name_candidates: vec![short_name.to_string()],
        };
    }

    let Some(type_text) = text_after_separator_colon(primary_line, colon_index) else {
        return ClassMatchCandidates::default();
    };

    class_match_candidates_from_type_text(type_text)
}

fn participant_suggestion_queries(
    participant: &str,
    participant_info: &SequenceParticipantInfo,
) -> Vec<String> {
    let mut queries = BTreeSet::new();

    insert_participant_suggestion_query(&mut queries, participant);

    if participant_info.display_name != participant {
        insert_participant_suggestion_query(&mut queries, &participant_info.display_name);
    }

    let display_candidates = class_match_candidates_from_display(participant_info);
    for candidate in display_candidates
        .id_candidates
        .into_iter()
        .chain(display_candidates.name_candidates)
    {
        insert_participant_suggestion_query(&mut queries, &candidate);
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

fn text_after_separator_colon(primary_line: &str, colon_index: usize) -> Option<&str> {
    let type_text = primary_line[colon_index + 1..].trim();
    (!type_text.is_empty()).then_some(type_text)
}

fn class_match_candidates_from_type_text(type_text: &str) -> ClassMatchCandidates {
    let mut candidates = ClassMatchCandidates {
        id_candidates: Vec::new(),
        name_candidates: Vec::new(),
    };

    if type_text.contains("::") {
        candidates.id_candidates.push(type_text.to_string());
    }

    candidates.name_candidates.push(type_text.to_string());

    if let Some(short_name) = type_text.rsplit("::").next().map(str::trim) {
        if !short_name.is_empty() && short_name != type_text {
            candidates.name_candidates.push(short_name.to_string());
        }
    }

    candidates
}

fn ignored_special_display_suffix(participant_info: &SequenceParticipantInfo) -> Option<String> {
    let normalized_segments = normalized_display_segments(&participant_info.display_name);
    let (primary_line, ignored_segments) = normalized_segments.split_first()?;
    let separator_colons = separator_colon_positions(primary_line);

    if separator_colons.len() != 1 || ignored_segments.is_empty() {
        return None;
    }

    Some(ignored_segments.join(" | "))
}

fn first_nonempty_display_line(display_name: &str) -> Option<&str> {
    normalized_display_segments(display_name).into_iter().next()
}

fn normalized_display_segments(display_name: &str) -> Vec<&str> {
    let mut lines = Vec::new();

    for physical_line in display_name.lines() {
        for escaped_line in physical_line
            .split("\\n")
            .flat_map(|segment| segment.split("/n"))
        {
            let trimmed = escaped_line.trim();
            if !trimmed.is_empty() {
                lines.push(trimmed);
            }
        }
    }

    lines
}

fn separator_colon_positions(text: &str) -> Vec<usize> {
    const COLON: u8 = b':';

    let bytes = text.as_bytes();

    bytes
        .iter()
        .enumerate()
        .filter_map(|(index, byte)| {
            if *byte != COLON {
                return None;
            }

            let previous_is_colon = index > 0 && bytes[index - 1] == COLON;
            let next_is_colon = index + 1 < bytes.len() && bytes[index + 1] == COLON;

            (!previous_is_colon && !next_is_colon).then_some(index)
        })
        .collect()
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
