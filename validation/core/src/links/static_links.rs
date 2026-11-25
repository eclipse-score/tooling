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

//! Element links: a reference links to the one other diagram that defines it.

use std::collections::{BTreeMap, HashSet};

use super::facts::DiagramFacts;
use super::schema::ElementLink;

fn id_leaf(id: &str) -> &str {
    id.rsplit('.').next().unwrap_or(id)
}

/// Defining diagrams by id and by last id segment.
struct DefinerIndex<'a> {
    diagrams: BTreeMap<&'a str, &'a DiagramFacts>,
    by_id: BTreeMap<&'a str, Vec<&'a str>>,
    by_leaf: BTreeMap<&'a str, Vec<&'a str>>,
}

impl<'a> DefinerIndex<'a> {
    fn new(diagrams: &'a [DiagramFacts]) -> Self {
        let mut index = Self {
            diagrams: BTreeMap::new(),
            by_id: BTreeMap::new(),
            by_leaf: BTreeMap::new(),
        };
        for diagram in diagrams {
            index.diagrams.entry(&diagram.source).or_insert(diagram);
        }
        for (&source, diagram) in &index.diagrams {
            for define in &diagram.defines {
                if define.id.is_empty() {
                    continue;
                }
                add(&mut index.by_id, &define.id, source);
                add(&mut index.by_leaf, id_leaf(&define.id), source);
            }
        }
        index
    }

    /// The diagram that defines `id` for a reference in `source`, or `None`.
    ///
    /// The exact id wins; without another diagram that defines it, the last
    /// id segment is used. Candidates that define it only by a namespace
    /// wrapper lose against diagrams that draw it. Ties go to the closest
    /// path, then to the diagram with most elements below `id`. A tie after
    /// that logs a warning and gives no link. A diagram never links to itself.
    fn definer(&self, id: &str, source: &str) -> Option<&'a str> {
        let others = |index: &BTreeMap<&'a str, Vec<&'a str>>, key: &str| -> Vec<&'a str> {
            index
                .get(key)
                .map(|sources| sources.iter().copied().filter(|&s| s != source).collect())
                .unwrap_or_default()
        };
        let mut candidates = others(&self.by_id, id);
        let mut by_leaf = false;
        if candidates.is_empty() {
            candidates = others(&self.by_leaf, id_leaf(id));
            by_leaf = true;
        }
        if candidates.len() > 1 {
            candidates = self.prefer_elements(candidates, id, by_leaf);
        }
        match candidates.as_slice() {
            [] => return None,
            [only] => return Some(only),
            _ => {}
        }

        let (_, closest) = tied_top(&candidates, |c| common_prefix_length(source, c));
        if let [only] = closest.as_slice() {
            return Some(only);
        }
        let (most, deepest) = tied_top(&closest, |c| self.descendant_count(c, id));
        if let [only] = deepest.as_slice() {
            if most > 0 {
                return Some(only);
            }
        }

        log::warn!(
            "diagram links: ambiguous definition for '{id}' in '{source}': \
             tied candidates {candidates:?}; no link"
        );
        None
    }

    fn prefer_elements(&self, candidates: Vec<&'a str>, id: &str, by_leaf: bool) -> Vec<&'a str> {
        let only_namespace = |source: &str| {
            let diagram = self.diagrams[source];
            if diagram.namespace_defines.is_empty() {
                return false;
            }
            let mut matched = diagram
                .defines
                .iter()
                .filter(|define| {
                    if by_leaf {
                        id_leaf(&define.id) == id_leaf(id)
                    } else {
                        define.id == id
                    }
                })
                .peekable();
            matched.peek().is_some()
                && matched.all(|define| diagram.namespace_defines.contains(&define.id))
        };
        let elements: Vec<&'a str> = candidates
            .iter()
            .copied()
            .filter(|c| !only_namespace(c))
            .collect();
        if elements.is_empty() {
            candidates
        } else {
            elements
        }
    }

    /// Elements of `source` below `id`.
    fn descendant_count(&self, source: &str, id: &str) -> usize {
        let prefix = format!("{id}.");
        let diagram = self.diagrams[source];
        diagram
            .defines
            .iter()
            .chain(&diagram.references)
            .filter(|entry| entry.id.starts_with(&prefix))
            .count()
    }
}

fn add<'a>(index: &mut BTreeMap<&'a str, Vec<&'a str>>, key: &'a str, source: &'a str) {
    let sources = index.entry(key).or_default();
    if !sources.contains(&source) {
        sources.push(source);
    }
}

/// Number of leading path components `a` and `b` share.
fn common_prefix_length(a: &str, b: &str) -> usize {
    a.split('/')
        .zip(b.split('/'))
        .take_while(|(x, y)| x == y)
        .count()
}

/// Highest score and every candidate with it.
fn tied_top<'a>(candidates: &[&'a str], score: impl Fn(&str) -> usize) -> (usize, Vec<&'a str>) {
    let mut best = 0;
    let mut top = Vec::new();
    for &candidate in candidates {
        let value = score(candidate);
        if top.is_empty() || value > best {
            best = value;
            top = vec![candidate];
        } else if value == best {
            top.push(candidate);
        }
    }
    (best, top)
}

/// One link per alias of each diagram: the first reference with that alias
/// that has a definer.
pub(crate) fn element_links(diagrams: &[DiagramFacts]) -> Vec<ElementLink> {
    let index = DefinerIndex::new(diagrams);
    let mut links = Vec::new();
    for (&source, diagram) in &index.diagrams {
        let mut linked: HashSet<&str> = HashSet::new();
        for reference in &diagram.references {
            if reference.alias.is_empty() || linked.contains(reference.alias.as_str()) {
                continue;
            }
            if let Some(target) = index.definer(&reference.id, source) {
                linked.insert(&reference.alias);
                links.push(ElementLink {
                    source: source.to_string(),
                    alias: reference.alias.clone(),
                    target: target.to_string(),
                });
            }
        }
    }
    links
}

#[cfg(test)]
#[path = "test/static_links_test.rs"]
mod tests;
