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

//! Rules shared by the diagram kinds that resolve several files as one group.

use std::collections::{BTreeMap, HashMap, HashSet};

use source_location::SourceLocation;
use uid_normalization::InternalScope;

/// A top-level declaration that may be reparented onto a nested declaration
/// of another file.
#[derive(Debug, Clone)]
pub struct TopLevelCandidate<K> {
    /// Id in its own file, before reparenting.
    pub id: String,
    pub file_idx: usize,
    /// Must equal the kind of the nested declaration.
    pub kind: K,
    pub has_body: bool,
    pub rooted: bool,
    pub source_location: SourceLocation,
}

/// A nested declaration a top-level declaration of another file may be
/// reparented onto.
#[derive(Debug, Clone)]
pub struct NestedCandidate<K> {
    /// Id in its own file, before reparenting.
    pub id: String,
    /// Written name path, the tail of `id`.
    pub own: String,
    /// Id of the top-level declaration it sits under.
    pub root_id: String,
    pub file_idx: usize,
    pub kind: K,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImplicitParentError {
    /// The declaration matches nested declarations under several scopes.
    #[error(
        "AmbiguousImplicitParent: {id} (line {line}) matches nested declarations under \
         {candidates:?}",
        line = source_location.line
    )]
    Ambiguous {
        id: String,
        file_idx: usize,
        candidates: Vec<String>,
        source_location: SourceLocation,
    },
    /// The parent is itself reparented back onto the declaration.
    #[error(
        "ImplicitParentCycle: {id} (line {line}) is reparented through {chain:?}",
        line = source_location.line
    )]
    Cycle {
        id: String,
        file_idx: usize,
        chain: Vec<String>,
        source_location: SourceLocation,
    },
}

impl ImplicitParentError {
    /// Index of the file the declaration is in.
    pub fn file_idx(&self) -> usize {
        match self {
            Self::Ambiguous { file_idx, .. } | Self::Cycle { file_idx, .. } => *file_idx,
        }
    }
}

type Memo = HashMap<(usize, String), Option<InternalScope>>;

/// Per file, the scope each top-level declaration is reparented onto.
///
/// Only bodied, non-rooted declarations move, onto the nested declaration of
/// another file with the same name and kind. The parent may itself move.
pub fn discover_implicit_parents<K: PartialEq>(
    top_level: &[TopLevelCandidate<K>],
    nested: &[NestedCandidate<K>],
    file_names: &[&str],
) -> Result<Vec<HashMap<String, InternalScope>>, ImplicitParentError> {
    let mut memo = Memo::new();
    let mut result = vec![HashMap::new(); file_names.len()];

    for candidate in top_level.iter().filter(|c| c.has_body && !c.rooted) {
        let scope = resolve_scope(
            candidate,
            top_level,
            nested,
            file_names,
            &mut memo,
            &mut HashSet::new(),
        )?;
        if let Some(scope) = scope {
            result[candidate.file_idx].insert(candidate.id.clone(), scope);
        }
    }

    Ok(result)
}

/// Scope `candidate` is reparented onto, `None` if it stays top-level.
fn resolve_scope<K: PartialEq>(
    candidate: &TopLevelCandidate<K>,
    top_level: &[TopLevelCandidate<K>],
    nested: &[NestedCandidate<K>],
    file_names: &[&str],
    memo: &mut Memo,
    visiting: &mut HashSet<(usize, String)>,
) -> Result<Option<InternalScope>, ImplicitParentError> {
    if candidate.rooted {
        return Ok(None);
    }
    let key = (candidate.file_idx, candidate.id.clone());
    if let Some(cached) = memo.get(&key) {
        return Ok(cached.clone());
    }
    if !visiting.insert(key.clone()) {
        let mut chain: Vec<String> = visiting
            .iter()
            .map(|(file_idx, id)| format!("{id}@{}", file_names[*file_idx]))
            .collect();
        chain.sort();
        return Err(ImplicitParentError::Cycle {
            id: candidate.id.clone(),
            file_idx: candidate.file_idx,
            chain,
            source_location: candidate.source_location.clone(),
        });
    }

    let mut scopes: BTreeMap<String, InternalScope> = BTreeMap::new();
    for entry in nested {
        if entry.file_idx == candidate.file_idx
            || entry.own != candidate.id
            || entry.kind != candidate.kind
        {
            continue;
        }
        let Some(ancestors) = entry.id.strip_suffix(&format!(".{}", entry.own)) else {
            continue;
        };
        let root = top_level
            .iter()
            .find(|c| c.file_idx == entry.file_idx && c.id == entry.root_id);
        let root_scope = match root {
            Some(root) => resolve_scope(root, top_level, nested, file_names, memo, visiting)?,
            None => None,
        };
        let scope = match root_scope {
            Some(prefix) => prefix.child(ancestors),
            None => InternalScope::from_path(ancestors),
        };
        scopes.insert(scope.id(), scope);
    }

    visiting.remove(&key);

    let resolved = match scopes.len() {
        0 => None,
        1 => scopes.pop_first().map(|(_, scope)| scope),
        _ => {
            return Err(ImplicitParentError::Ambiguous {
                id: candidate.id.clone(),
                file_idx: candidate.file_idx,
                candidates: scopes.into_keys().collect(),
                source_location: candidate.source_location.clone(),
            })
        }
    };
    memo.insert(key, resolved.clone());
    Ok(resolved)
}

/// The single `(id, payload)` the other files resolve a reference to.
///
/// The same id found twice counts once; distinct ids are ambiguous and yield
/// the sorted candidate ids.
pub fn unique_peer_match<T>(
    matches: impl IntoIterator<Item = (String, T)>,
) -> Result<Option<(String, T)>, Vec<String>> {
    let mut found: BTreeMap<String, T> = BTreeMap::new();
    for (id, payload) in matches {
        found.entry(id).or_insert(payload);
    }
    if found.len() > 1 {
        return Err(found.into_keys().collect());
    }
    Ok(found.pop_first())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location() -> SourceLocation {
        SourceLocation::new("test.puml", 1)
    }

    fn top(id: &str, file_idx: usize, has_body: bool) -> TopLevelCandidate<&'static str> {
        TopLevelCandidate {
            id: id.to_string(),
            file_idx,
            kind: "component",
            has_body,
            rooted: false,
            source_location: location(),
        }
    }

    fn nested(
        id: &str,
        own: &str,
        root_id: &str,
        file_idx: usize,
    ) -> NestedCandidate<&'static str> {
        NestedCandidate {
            id: id.to_string(),
            own: own.to_string(),
            root_id: root_id.to_string(),
            file_idx,
            kind: "component",
        }
    }

    #[test]
    fn body_matches_nested_declaration_of_other_file() {
        let top_level = [top("c", 1, true), top("pkg", 0, true)];
        let nested = [nested("pkg.c", "c", "pkg", 0)];

        let result = discover_implicit_parents(&top_level, &nested, &["a", "b"]).unwrap();

        assert!(result[0].is_empty());
        assert_eq!(result[1]["c"].id(), "pkg");
    }

    #[test]
    fn bodyless_declaration_stays_top_level() {
        let top_level = [top("c", 1, false), top("pkg", 0, true)];
        let nested = [nested("pkg.c", "c", "pkg", 0)];

        let result = discover_implicit_parents(&top_level, &nested, &["a", "b"]).unwrap();

        assert!(result[1].is_empty());
    }

    #[test]
    fn kind_mismatch_stays_top_level() {
        let top_level = [top("c", 1, true), top("pkg", 0, true)];
        let mut other = nested("pkg.c", "c", "pkg", 0);
        other.kind = "interface";

        let result = discover_implicit_parents(&top_level, &[other], &["a", "b"]).unwrap();

        assert!(result[1].is_empty());
    }

    #[test]
    fn same_file_declaration_is_no_parent() {
        let top_level = [top("c", 0, true), top("pkg", 0, true)];
        let nested = [nested("pkg.c", "c", "pkg", 0)];

        let result = discover_implicit_parents(&top_level, &nested, &["a"]).unwrap();

        assert!(result[0].is_empty());
    }

    #[test]
    fn rooted_declaration_is_never_reparented() {
        let mut rooted = top("c", 1, true);
        rooted.rooted = true;
        let top_level = [rooted, top("pkg", 0, true)];
        let nested = [nested("pkg.c", "c", "pkg", 0)];

        let result = discover_implicit_parents(&top_level, &nested, &["a", "b"]).unwrap();

        assert!(result[1].is_empty());
    }

    #[test]
    fn matches_under_two_scopes_are_ambiguous() {
        let top_level = [top("c", 2, true), top("p", 0, true), top("q", 1, true)];
        let nested = [nested("p.c", "c", "p", 0), nested("q.c", "c", "q", 1)];

        let error = discover_implicit_parents(&top_level, &nested, &["a", "b", "c"]).unwrap_err();

        assert!(matches!(
            error,
            ImplicitParentError::Ambiguous { ref candidates, .. }
                if candidates == &["p".to_string(), "q".to_string()]
        ));
    }

    #[test]
    fn parent_reparented_first_prefixes_scope() {
        let top_level = [
            top("outer", 0, true),
            top("pkg", 1, true),
            top("c", 2, true),
        ];
        let nested = [
            nested("outer.pkg", "pkg", "outer", 0),
            nested("pkg.c", "c", "pkg", 1),
        ];

        let result = discover_implicit_parents(&top_level, &nested, &["a", "b", "c"]).unwrap();

        assert_eq!(result[1]["pkg"].id(), "outer");
        assert_eq!(result[2]["c"].id(), "outer.pkg");
    }

    #[test]
    fn mutual_reparenting_is_a_cycle() {
        let top_level = [top("a", 0, true), top("b", 1, true)];
        let nested = [nested("b.a", "a", "b", 1), nested("a.b", "b", "a", 0)];

        let error = discover_implicit_parents(&top_level, &nested, &["x", "y"]).unwrap_err();

        assert!(matches!(error, ImplicitParentError::Cycle { .. }));
    }

    #[test]
    fn peer_match_dedups_equal_ids() {
        let found = unique_peer_match([("a.b".to_string(), 1), ("a.b".to_string(), 2)]);

        assert_eq!(found, Ok(Some(("a.b".to_string(), 1))));
    }

    #[test]
    fn peer_match_reports_distinct_ids_as_ambiguous() {
        let found = unique_peer_match([("b".to_string(), ()), ("a".to_string(), ())]);

        assert_eq!(found, Err(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn peer_match_without_hits_is_none() {
        let found = unique_peer_match(Vec::<(String, ())>::new());

        assert_eq!(found, Ok(None));
    }
}
