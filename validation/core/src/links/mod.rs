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

//! Links between diagrams for the docs build: a reference links to the
//! diagram that defines the element, a call to the diagrams that refine it.

mod facts;
mod schema;
mod static_links;
mod zoom_links;

pub use facts::{ComponentElement, DiagramFacts, FactEntry};
pub use schema::{CallLink, DiagramLinks, ElementLink};

use crate::models::SequenceRefinementIndex;
use crate::validators::check_sequence_refinement;

/// Element links of `diagrams` and call links of `refinement` (only details
/// that pass the refinement check).
pub(crate) fn diagram_links(
    diagrams: &[DiagramFacts],
    refinement: Option<&SequenceRefinementIndex>,
) -> DiagramLinks {
    let call_links = refinement
        .map(|index| zoom_links::call_links(index, &check_sequence_refinement(index)))
        .unwrap_or_default();
    DiagramLinks::new(static_links::element_links(diagrams), call_links)
}
