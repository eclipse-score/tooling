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

//! Call links: a call links to the diagrams that refine it (its details).

use std::collections::BTreeSet;

use super::schema::CallLink;
use crate::models::SequenceRefinementIndex;
use crate::validators::RefinementCheck;

/// One link per refined call written in its own diagram (not in an
/// `!include` fragment). Targets are the sorted detail diagrams that pass the
/// refinement check; a call without such a detail has no link.
pub(crate) fn call_links(
    index: &SequenceRefinementIndex,
    check: &RefinementCheck,
) -> Vec<CallLink> {
    let mut links = Vec::new();
    for (position, (entry, details)) in index.entries.iter().zip(&index.details).enumerate() {
        let Some(source) = entry.source.as_deref() else {
            continue;
        };
        if &*entry.source_location.file != source {
            continue;
        }
        let targets: BTreeSet<&str> = details
            .iter()
            .filter(|&&detail| {
                let accepted = check.accepts(position, detail);
                if !accepted {
                    log::debug!(
                        "diagram links: no link from {source}:{} to {}: refinement check failed",
                        entry.source_location.line,
                        index.entries[detail].diagram
                    );
                }
                accepted
            })
            .filter_map(|&detail| index.entries[detail].source.as_deref())
            .filter(|&target| target != source)
            .collect();
        if targets.is_empty() {
            continue;
        }
        links.push(CallLink {
            source: source.to_string(),
            line: entry.source_location.line,
            method: entry.method.clone(),
            targets: targets.into_iter().map(str::to_string).collect(),
        });
    }
    links
}

#[cfg(test)]
#[path = "test/zoom_links_test.rs"]
mod tests;
