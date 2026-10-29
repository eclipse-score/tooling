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

//! Class collection policy, separate from the public model's template kinds.

use clang::{token::TokenKind, Entity, EntityKind};
use class_diagram::ClassTemplateKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClassCollectionDecision {
    Collect(ClassTemplateKind),
    SkipInstantiation,
    Unknown,
}

/// Template association alone also matches instantiations. Only a recognized
/// declaration header establishes an explicit specialization or instantiation.
pub(crate) fn classify_class(entity: &Entity) -> ClassCollectionDecision {
    match entity.get_kind() {
        EntityKind::ClassTemplate => ClassCollectionDecision::Collect(ClassTemplateKind::Primary),
        EntityKind::ClassTemplatePartialSpecialization => {
            ClassCollectionDecision::Collect(ClassTemplateKind::PartialSpecialization)
        }
        _ if entity.get_template().is_some() => {
            let Some(range) = entity.get_range() else {
                return ClassCollectionDecision::Unknown;
            };
            let tokens: Vec<_> = range
                .tokenize()
                .into_iter()
                .filter(|token| token.get_kind() != TokenKind::Comment)
                .map(|token| token.get_spelling())
                .collect();
            classify_associated_header(&tokens.iter().map(String::as_str).collect::<Vec<_>>())
        }
        _ => ClassCollectionDecision::Collect(ClassTemplateKind::NonTemplate),
    }
}

fn classify_associated_header(mut tokens: &[&str]) -> ClassCollectionDecision {
    // Do not search the body for template<>: it may belong to a nested class.
    // Multiple empty heads can specialize successive enclosing templates.
    let mut specialization = false;
    while let ["template", "<", ">", rest @ ..] = tokens {
        specialization = true;
        tokens = rest;
    }
    if specialization {
        return match tokens {
            ["class" | "struct" | "union", ..] => {
                ClassCollectionDecision::Collect(ClassTemplateKind::ExplicitSpecialization)
            }
            // Nonempty subsequent heads, macros and unfamiliar prefixes need
            // more information; they must not become NonTemplate or a guessed
            // specialization. ClassTemplate cursors were handled above.
            _ => ClassCollectionDecision::Unknown,
        };
    }
    match tokens {
        ["template", "class" | "struct" | "union", ..]
        | ["extern", "template", "class" | "struct" | "union", ..] => {
            ClassCollectionDecision::SkipInstantiation
        }
        _ => ClassCollectionDecision::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_associated_header, ClassCollectionDecision};
    use class_diagram::ClassTemplateKind;

    #[test]
    fn recognizes_specialization_headers_only() {
        for tokens in [
            vec!["template", "<", ">", "struct", "A", "<", "int", ">", ";"],
            vec![
                "template", "<", ">", "class", "A", "<", "int", ">", "{", "}",
            ],
            vec![
                "template", "<", ">", "template", "<", ">", "struct", "Inner",
            ],
        ] {
            assert_eq!(
                classify_associated_header(&tokens),
                ClassCollectionDecision::Collect(ClassTemplateKind::ExplicitSpecialization),
            );
        }
    }

    #[test]
    fn skips_both_explicit_instantiation_forms() {
        for tokens in [
            vec!["template", "struct", "A", "<", "double", ">"],
            vec!["extern", "template", "class", "A", "<", "float", ">"],
        ] {
            assert_eq!(
                classify_associated_header(&tokens),
                ClassCollectionDecision::SkipInstantiation,
            );
        }
    }

    #[test]
    fn ambiguous_tokens_are_not_guessed_or_scanned_for_nested_heads() {
        for tokens in [
            vec![],
            vec!["SPECIAL", "struct", "A"],
            vec!["template", "<", ">", "SPECIAL", "struct", "A"],
            vec!["struct", "A", "{", "template", "<", ">", "struct", "Inner"],
            vec![
                "template", "<", ">", "template", "<", "class", "T", ">", "struct", "Inner",
            ],
        ] {
            assert_eq!(
                classify_associated_header(&tokens),
                ClassCollectionDecision::Unknown
            );
        }
    }
}
