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

//! Candidate graphs and unique injective matchings covering every left-side row.

/// Whether a candidate graph has zero, multiple, or exactly one covering matching.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::validators) enum MatchingOutcome {
    NoSolution,
    Ambiguous,
    /// Each vector index is a left-side row; its value is the selected right-side column.
    /// A unique assignment does not imply any caller-specific validation has passed.
    Unique(Vec<usize>),
}

/// For each design item, collect the indices of all compatible implementation items.
/// The outer list follows design order; each inner list contains implementation indices.
pub(in crate::validators) fn candidate_graph<T>(
    design_items: &[T],
    implementation_items: &[T],
    matches: impl Fn(&T, &T) -> bool,
) -> Vec<Vec<usize>> {
    let mut candidates_by_design = Vec::new();
    for design_item in design_items {
        let mut compatible_implementation_indices = Vec::new();
        for (implementation_index, implementation_item) in implementation_items.iter().enumerate() {
            if matches(design_item, implementation_item) {
                compatible_implementation_indices.push(implementation_index);
            }
        }
        // Keep an empty list when this design item has no compatible implementation.
        candidates_by_design.push(compatible_implementation_indices);
    }
    candidates_by_design
}

/// Keep design items and their candidate rows aligned while removing empty rows.
/// Preserve item order and implementation column indices. Inputs must have equal
/// lengths; callers are responsible for diagnosing or otherwise handling removed rows.
pub(in crate::validators) fn nonempty_candidate_rows<'a, T>(
    designs: &[&'a T],
    candidates: &[Vec<usize>],
) -> (Vec<&'a T>, Vec<Vec<usize>>) {
    designs
        .iter()
        .zip(candidates)
        .filter(|(_, candidate_columns)| !candidate_columns.is_empty())
        .map(|(&design, candidate_columns)| (design, candidate_columns.clone()))
        .unzip()
}

/// A second covering matching must omit at least one edge in the first.
/// Re-running augmenting paths with each selected edge banned therefore proves
/// uniqueness without enumerating permutations; unused right-side nodes are OK.
/// An empty graph has a unique empty matching. Every candidate column must be
/// less than `implementation_count`.
pub(in crate::validators) fn unique_covering_matching(
    graph: &[Vec<usize>],
    implementation_count: usize,
) -> MatchingOutcome {
    let Some(matching) = covering_matching(graph, implementation_count, None) else {
        return MatchingOutcome::NoSolution;
    };
    for (row, &column) in matching.iter().enumerate() {
        if covering_matching(graph, implementation_count, Some((row, column))).is_some() {
            return MatchingOutcome::Ambiguous;
        }
    }
    MatchingOutcome::Unique(matching)
}

fn covering_matching(
    graph: &[Vec<usize>],
    implementation_count: usize,
    banned: Option<(usize, usize)>,
) -> Option<Vec<usize>> {
    fn augment(
        row: usize,
        graph: &[Vec<usize>],
        owners: &mut [Option<usize>],
        visited: &mut [bool],
        banned: Option<(usize, usize)>,
    ) -> bool {
        for &column in &graph[row] {
            if banned == Some((row, column)) || visited[column] {
                continue;
            }
            visited[column] = true;
            if owners[column].is_none()
                || augment(owners[column].unwrap(), graph, owners, visited, banned)
            {
                owners[column] = Some(row);
                return true;
            }
        }
        false
    }

    let mut owners = vec![None; implementation_count];
    for row in 0..graph.len() {
        if !augment(
            row,
            graph,
            &mut owners,
            &mut vec![false; implementation_count],
            banned,
        ) {
            return None;
        }
    }
    let mut matching = vec![0; graph.len()];
    for (column, row) in owners.into_iter().enumerate() {
        if let Some(row) = row {
            matching[row] = column;
        }
    }
    Some(matching)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonempty_candidate_rows_preserves_alignment_and_column_indices() {
        let items = [10, 20, 30, 40];
        let designs = items.iter().collect::<Vec<_>>();
        let candidates = vec![vec![], vec![0, 2], vec![], vec![2]];
        let (filtered_designs, filtered_candidates) =
            nonempty_candidate_rows(&designs, &candidates);
        assert_eq!(filtered_designs, vec![&items[1], &items[3]]);
        assert_eq!(filtered_candidates, vec![vec![0, 2], vec![2]]);
        assert_eq!(candidates, vec![vec![], vec![0, 2], vec![], vec![2]]);

        let empty_rows = vec![vec![]; designs.len()];
        assert_eq!(
            nonempty_candidate_rows(&designs, &empty_rows),
            (vec![], vec![])
        );
        assert_eq!(nonempty_candidate_rows::<i32>(&[], &[]), (vec![], vec![]));
    }

    #[test]
    fn candidate_graph_preserves_row_and_column_order() {
        let graph = candidate_graph(&[2, 1, 3], &[1, 2, 2], |left, right| left == right);
        assert_eq!(graph, vec![vec![1, 2], vec![0], vec![]]);
        assert!(candidate_graph::<i32>(&[], &[1], |left, right| left == right).is_empty());
        assert_eq!(
            candidate_graph(&[1, 2], &[], |left, right| left == right),
            vec![Vec::<usize>::new(), vec![]]
        );
    }

    #[test]
    fn augmenting_path_matching_agrees_with_exhaustive_small_graph_oracle() {
        fn count_matchings(row: usize, graph: &[Vec<usize>], used: &mut [bool]) -> usize {
            if row == graph.len() {
                return 1;
            }
            let mut count = 0;
            for &column in &graph[row] {
                if !used[column] {
                    used[column] = true;
                    count += count_matchings(row + 1, graph, used);
                    used[column] = false;
                }
            }
            count
        }
        for rows in 0..=3 {
            for columns in 0..=3 {
                for bits in 0usize..(1usize << (rows * columns)) {
                    let graph: Vec<Vec<usize>> = (0..rows)
                        .map(|row| {
                            (0..columns)
                                .filter(|column| bits & (1 << (row * columns + column)) != 0)
                                .collect()
                        })
                        .collect();
                    let count = count_matchings(0, &graph, &mut vec![false; columns]);
                    let actual = unique_covering_matching(&graph, columns);
                    match (count, actual) {
                        (0, MatchingOutcome::NoSolution) => {}
                        (1, MatchingOutcome::Unique(matching)) => {
                            assert_eq!(matching.len(), rows);
                            for (row, column) in matching.iter().enumerate() {
                                assert!(graph[row].contains(column));
                                assert!(!matching[..row].contains(column));
                            }
                        }
                        (2.., MatchingOutcome::Ambiguous) => {}
                        (_, actual) => panic!("graph={graph:?}, oracle={count}, actual={actual:?}"),
                    }
                }
            }
        }
    }
}
