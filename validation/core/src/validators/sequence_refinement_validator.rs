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

//! Validation: a sequence diagram that opens a black box (zoom detail) keeps
//! the boundary of the call it refines. See
//! `docs/specifications/sequence_refinement.md`.

use std::collections::HashSet;

use super::shared::format_sequence_call;
use crate::models::{Lifeline, SequenceRefinementIndex, WindowCall, ZoomEntry};
use crate::results::{ErrorBuilder, ErrorCategory};
use crate::ValidationResult;

/// Findings of the refinement check and which refinements they rule out.
#[derive(Default)]
pub struct RefinementCheck {
    pub result: ValidationResult,
    /// Synchronous refined entries whose window is never closed.
    unclosed: HashSet<usize>,
    /// Entries with an exit that no detail keeps.
    exit_missing: HashSet<usize>,
    /// `(entry, detail)` where the detail leaves the boundary of the entry.
    rejected: HashSet<(usize, usize)>,
}

impl RefinementCheck {
    /// The detail passes the check of the entry it refines: both windows are
    /// closed, no exit of the entry is missing and the detail keeps the
    /// boundary.
    pub fn accepts(&self, entry: usize, detail: usize) -> bool {
        !self.unclosed.contains(&entry)
            && !self.unclosed.contains(&detail)
            && !self.exit_missing.contains(&entry)
            && !self.rejected.contains(&(entry, detail))
    }
}

pub fn validate_sequence_refinement(index: &SequenceRefinementIndex) -> ValidationResult {
    check_sequence_refinement(index).result
}

/// A call without a detail is not checked. For each detail of a call into
/// `C`:
/// - its lifelines are inside `C`, the sender, an exit partner (or an element it
///   calls) or environment;
/// - together, the details of a call keep every exit of `C`.
///
/// Every window of a synchronous zoomed call is closed by a reply or a
/// deactivation.
pub fn check_sequence_refinement(index: &SequenceRefinementIndex) -> RefinementCheck {
    let mut check = RefinementCheck::default();

    for (position, entry) in index.entries.iter().enumerate() {
        if index.is_zoom_active(position) && !entry.closed && !entry.asynchronous {
            check.result.add_failure(window_not_closed_error(entry));
            check.unclosed.insert(position);
        }

        let details = &index.details[position];
        if details.is_empty() {
            continue;
        }

        let exits: Vec<&WindowCall> = index.exits(entry).collect();
        for &detail in details {
            if !check_lifelines(
                index,
                entry,
                &index.entries[detail],
                &exits,
                &mut check.result,
            ) {
                check.rejected.insert((position, detail));
            }
        }

        let mut reported = HashSet::new();
        for exit in exits {
            if !reported.insert((&exit.to, &exit.method)) {
                continue;
            }
            if !details
                .iter()
                .any(|&detail| covers(index, entry, &index.entries[detail], exit))
            {
                check
                    .result
                    .add_failure(exit_missing_error(entry, exit, details, index));
                check.exit_missing.insert(position);
            }
        }
    }

    check
}

/// `false` if the detail has a participant outside the boundary of `entry`.
fn check_lifelines(
    index: &SequenceRefinementIndex,
    entry: &ZoomEntry,
    detail: &ZoomEntry,
    exits: &[&WindowCall],
    result: &mut ValidationResult,
) -> bool {
    let mut partners: Vec<String> = exits
        .iter()
        .filter_map(|exit| match &exit.to {
            Lifeline::Element(partner) => Some(partner.clone()),
            Lifeline::Environment(_) | Lifeline::Unresolved(_) => None,
        })
        .collect();
    extend_with_downstream(index, entry, detail, &mut partners);

    let allowed = |id: &str| {
        let lifeline = Lifeline::Element(id.to_string());
        index.inside(&lifeline, &entry.container)
            || lifeline == entry.sender
            || partners
                .iter()
                .any(|p| index.within_partner(&lifeline, p, &entry.container))
    };

    let mut reported = HashSet::new();
    for call in &detail.calls {
        for lifeline in [&call.from, &call.to] {
            let Lifeline::Element(id) = lifeline else {
                continue;
            };
            if *id == entry.container {
                if reported.insert(id) {
                    result.add_failure(container_in_detail_error(entry, detail, call));
                }
                continue;
            }
            if allowed(id) || !reported.insert(id) {
                continue;
            }
            result.add_failure(outside_container_error(entry, detail, id, call));
        }
    }
    reported.is_empty()
}

/// Adds the elements outside the container that the detail reaches through
/// calls made by an exit partner (or an element inside one), transitively.
fn extend_with_downstream(
    index: &SequenceRefinementIndex,
    entry: &ZoomEntry,
    detail: &ZoomEntry,
    partners: &mut Vec<String>,
) {
    let within_partner = |partners: &[String], lifeline: &Lifeline| {
        partners
            .iter()
            .any(|p| index.within_partner(lifeline, p, &entry.container))
    };
    loop {
        let reached = detail.calls.iter().find_map(|call| {
            let Lifeline::Element(callee) = &call.to else {
                return None;
            };
            let from_partner = within_partner(partners, &call.from);
            let known = within_partner(partners, &call.to);
            let outside = *callee != entry.container
                && !index.inside(&call.to, &entry.container)
                && call.to != entry.sender;
            (from_partner && outside && !known).then(|| callee.clone())
        });
        match reached {
            Some(callee) => partners.push(callee),
            None => break,
        }
    }
}

/// Some call of the detail leaves the container like `exit` does.
fn covers(
    index: &SequenceRefinementIndex,
    entry: &ZoomEntry,
    detail: &ZoomEntry,
    exit: &WindowCall,
) -> bool {
    detail.calls.iter().any(|call| {
        call.method == exit.method
            && index.within(&call.from, &entry.container)
            && match (&exit.to, &call.to) {
                (Lifeline::Environment(wanted), Lifeline::Environment(found)) => wanted == found,
                (Lifeline::Element(partner), found) => {
                    index.within_partner(found, partner, &entry.container)
                }
                _ => false,
            }
    })
}

fn describe(entry: &ZoomEntry) -> String {
    format_sequence_call(entry.sender.label(), &entry.container, &entry.method)
}

fn diagram_name(entry: &ZoomEntry) -> String {
    format!("\"{}\"", entry.diagram)
}

fn outside_container_error(
    entry: &ZoomEntry,
    detail: &ZoomEntry,
    id: &str,
    call: &WindowCall,
) -> String {
    let (source_file, source_line) = call.source_location.display();
    let container = &entry.container;

    ErrorBuilder::new(ErrorCategory::Design)
        .title(format!(
            "participant \"{id}\" in the detail of sequence call {} is not inside \"{container}\"",
            describe(entry)
        ))
        .field("finding", "ZoomParticipantOutsideContainer")
        .field("overview call", describe(entry))
        .field("detail diagram", diagram_name(detail))
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .fix(format!(
            "show the call to \"{id}\" as an exit of \"{container}\" in the overview, call it from an exit partner, or use participants inside \"{container}\" only"
        ))
        .build()
}

fn container_in_detail_error(entry: &ZoomEntry, detail: &ZoomEntry, call: &WindowCall) -> String {
    let (source_file, source_line) = call.source_location.display();
    let container = &entry.container;

    ErrorBuilder::new(ErrorCategory::Design)
        .title(format!(
            "the detail of sequence call {} shows the refined element \"{container}\" itself",
            describe(entry)
        ))
        .field("finding", "ZoomContainerInDetail")
        .field("overview call", describe(entry))
        .field("detail diagram", diagram_name(detail))
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .fix(format!(
            "show only the children of \"{container}\" in the detail; calls to \"{container}\" belong to the overview"
        ))
        .build()
}

fn exit_missing_error(
    entry: &ZoomEntry,
    exit: &WindowCall,
    details: &[usize],
    index: &SequenceRefinementIndex,
) -> String {
    let (source_file, source_line) = exit.source_location.display();
    let exit_call = format_sequence_call(exit.from.label(), exit.to.label(), &exit.method);
    let detail_diagrams = details
        .iter()
        .map(|&detail| diagram_name(&index.entries[detail]))
        .collect::<Vec<_>>()
        .join(", ");

    ErrorBuilder::new(ErrorCategory::Design)
        .title(format!(
            "call {exit_call} of the overview has no counterpart in any detail of sequence call {}",
            describe(entry)
        ))
        .field("finding", "ZoomExitMissing")
        .field("overview call", describe(entry))
        .field("missing exit", exit_call)
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .field("detail diagrams", detail_diagrams.clone())
        .fix(format!(
            "add a call \"{}\" from an element inside \"{}\" to one of the detail diagrams ({detail_diagrams}), or remove it from the overview",
            exit.method, entry.container
        ))
        .build()
}

fn window_not_closed_error(entry: &ZoomEntry) -> String {
    let (source_file, source_line) = entry.source_location.display();

    ErrorBuilder::new(ErrorCategory::Design)
        .title(format!(
            "sequence call {} is refined but never replied to or deactivated",
            describe(entry)
        ))
        .field("finding", "ZoomWindowNotClosed")
        .field("sequence diagram", diagram_name(entry))
        .field("sequence source file", format!("\"{source_file}\""))
        .field("sequence source line", source_line.to_string())
        .fix(format!(
            "reply to \"{}\" with `-->`, or `deactivate` \"{}\", or make the call asynchronous (`->>`); a `return` replies only after a call with `++` or an `activate` (`autoactivate` is ignored)",
            entry.sender.label(),
            entry.container
        ))
        .build()
}

#[cfg(test)]
#[path = "test/sequence_refinement_validator_test.rs"]
mod tests;
