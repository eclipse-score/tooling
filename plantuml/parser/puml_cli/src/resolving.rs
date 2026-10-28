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

//! Resolution of raw diagram ASTs into logical models.

use serde::Serialize;
use std::collections::HashMap;

use activity_diagram::ActivityDiagram;
use puml_resolver::{
    ActivityResolver, ClassResolver, ComponentResolver, DiagramResolver, DiagramSource,
    SequenceResolver,
};

use crate::parsing::ParsedDiagram;

#[derive(Debug, Serialize)]
pub enum ResolvedDiagram {
    Activity(ActivityDiagram),
    Component(HashMap<String, component_diagram::LogicComponent>),
    Class(class_diagram::ClassDiagram),
    Sequence(sequence_logic::SequenceTree),
}

impl ResolvedDiagram {
    /// Kind name used in diagnostics.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Activity(_) => "Activity",
            Self::Component(_) => "Component",
            Self::Class(_) => "Class",
            Self::Sequence(_) => "Sequence",
        }
    }
}

/// Resolves the diagrams of each kind in `parsed` as one group; `names` (one
/// per diagram) attribute errors to files. Results keep the order of `parsed`.
pub fn resolve_parsed_diagrams(
    names: &[String],
    parsed: &[ParsedDiagram],
) -> Result<Vec<ResolvedDiagram>, Box<dyn std::error::Error>> {
    let mut activity = Vec::new();
    let mut component = Vec::new();
    let mut class = Vec::new();
    let mut sequence = Vec::new();
    for (i, doc) in parsed.iter().enumerate() {
        match doc {
            ParsedDiagram::Activity(d) => activity.push((i, d)),
            ParsedDiagram::Component(d) => component.push((i, d)),
            ParsedDiagram::Class(d) => class.push((i, d)),
            ParsedDiagram::Sequence(d) => sequence.push((i, d)),
        }
    }

    let mut results: Vec<Option<ResolvedDiagram>> = (0..parsed.len()).map(|_| None).collect();
    resolve_bucket::<ActivityResolver>(names, &activity, ResolvedDiagram::Activity, &mut results)?;
    resolve_bucket::<ComponentResolver>(
        names,
        &component,
        ResolvedDiagram::Component,
        &mut results,
    )?;
    resolve_bucket::<ClassResolver>(names, &class, ResolvedDiagram::Class, &mut results)?;
    resolve_bucket::<SequenceResolver>(names, &sequence, ResolvedDiagram::Sequence, &mut results)?;

    Ok(results
        .into_iter()
        .map(|r| r.expect("every diagram is in exactly one bucket"))
        .collect())
}

/// Resolves the `(index, document)` entries of one kind as a group and stores
/// each result at its index.
fn resolve_bucket<R>(
    names: &[String],
    bucket: &[(usize, &R::Document)],
    wrap: fn(R::Output) -> ResolvedDiagram,
    results: &mut [Option<ResolvedDiagram>],
) -> Result<(), Box<dyn std::error::Error>>
where
    R: DiagramResolver + Default,
{
    if bucket.is_empty() {
        return Ok(());
    }
    let sources: Vec<DiagramSource<'_, R::Document>> = bucket
        .iter()
        .map(|&(i, document)| DiagramSource {
            name: names[i].as_str(),
            document,
        })
        .collect();
    let resolved = R::resolve_group(&sources)?;
    for (&(i, _), output) in bucket.iter().zip(resolved) {
        results[i] = Some(wrap(output));
    }
    Ok(())
}

/// Resolves one diagram alone.
#[cfg(test)]
pub fn resolve_single(
    parsed: ParsedDiagram,
) -> Result<ResolvedDiagram, Box<dyn std::error::Error>> {
    resolve_parsed_diagrams(&[String::new()], &[parsed]).map(|mut resolved| resolved.remove(0))
}
