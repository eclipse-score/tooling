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

use sequence_logic::SourceLocation;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SequenceResolverError {
    #[error("participant `{participant}` is used after destroy at {source_location}")]
    DestroyedParticipantUse {
        participant: String,
        source_location: SourceLocation,
    },

    #[error(
        "invalid sequence message arrow `{arrow}` at {source_location}: exactly one directional arrowhead is required"
    )]
    InvalidMessageDirection {
        arrow: String,
        source_location: SourceLocation,
    },

    #[error("unterminated sequence group at {source_location}, add 'end' to close the group")]
    UnterminatedGroup { source_location: SourceLocation },

    #[error("'else' is not valid in {kind:?} ({source_location}), only supported in alt and par")]
    ElseNotAllowedInGroup {
        kind: sequence_parser::sequence_ast::GroupKind,
        source_location: SourceLocation,
    },

    #[error("'else' is not valid outside a sequence group ({source_location})")]
    ElseOutsideGroup { source_location: SourceLocation },

    #[error(
        "group end kind {found:?} does not match start kind {expected:?} at {source_location}"
    )]
    MismatchedGroupEnd {
        expected: sequence_parser::sequence_ast::GroupKind,
        found: sequence_parser::sequence_ast::GroupKind,
        source_location: SourceLocation,
    },
}
