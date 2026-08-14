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
use crate::models::{ClassEntityIndex, SequenceDiagramIndex, SequenceParticipantInfo};
use crate::{Diagnostics, ErrorBuilder, ErrorCategory, ValidationResult};

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
            let ParticipantResolution::Matched(callee_class) =
                self.resolve_participant_class(&observed_call.callee)
            else {
                continue;
            };

            let method_name = extract_method_name(&observed_call.method);
            if method_name.is_empty() {
                continue;
            }

            if callee_class.methods.iter().any(|method| method.name == method_name) {
                continue;
            }

            let sequence_call =
                format_sequence_call(&observed_call.caller, &observed_call.callee, method_name);
            let (source_file, source_line) = observed_call.source_location.display();

            self.result.add_failure(
                ErrorBuilder::new(ErrorCategory::Method)
                    .title(format!(
                        "sequence function \"{method_name}\" from sequence call {sequence_call} not found on target class \"{}\" in the class diagram",
                        callee_class.id,
                    ))
                    .field("sequence call", sequence_call)
                    .field("target class", format!("\"{}\"", callee_class.id))
                    .field("sequence source file", format!("\"{source_file}\""))
                    .field("sequence source line", source_line.to_string())
                    .fix(format!(
                        "add method \"{method_name}\" to class \"{}\" in the class diagram, or change or remove that sequence call",
                        callee_class.id,
                    ))
                    .build(),
            );
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
