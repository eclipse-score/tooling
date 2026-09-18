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
use std::collections::HashSet;

use crate::common_parser::Rule;

#[derive(Debug, Default)]
pub struct IgnoredNoteRegistry {
    aliases: HashSet<String>,
}

impl IgnoredNoteRegistry {
    pub fn register(&mut self, alias: impl Into<String>) {
        self.aliases.insert(alias.into());
    }

    pub fn contains(&self, alias: &str) -> bool {
        self.aliases.contains(alias)
    }

    pub fn filters_endpoints(&self, left: &str, right: &str) -> bool {
        self.contains(left) || self.contains(right)
    }
}

pub fn is_note_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::note_single_line | Rule::note_multiline | Rule::note_declaration
    )
}

pub fn find_note_alias(pair: pest::iterators::Pair<Rule>) -> Option<String> {
    if pair.as_rule() == Rule::note_alias {
        return Some(pair.as_str().to_string());
    }

    pair.into_inner().find_map(find_note_alias)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_parser::PlantUmlCommonParser;
    use pest::Parser;

    #[test]
    fn test_find_note_alias_from_single_line_note() {
        let pair = PlantUmlCommonParser::parse(
            Rule::note_declaration,
            "note \"Synchronised access only\" as SyncNote",
        )
        .unwrap()
        .next()
        .unwrap();

        assert_eq!(find_note_alias(pair).as_deref(), Some("SyncNote"));
    }

    #[test]
    fn test_find_note_alias_from_multiline_note() {
        let pair = PlantUmlCommonParser::parse(
            Rule::note_declaration,
            "note as SyncNote\n  Synchronised access only\nend note\n",
        )
        .unwrap()
        .next()
        .unwrap();

        assert_eq!(find_note_alias(pair).as_deref(), Some("SyncNote"));
    }
}
