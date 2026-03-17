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

// Re-export commonly used items that don't have name conflicts
pub use class_parser::{ClassError, ClassUmlFile, PumlClassParser};
pub use component_parser::{CompPumlDocument, PumlComponentParser};
pub use parser_core::{common_ast, common_parser, Arrow, BaseParseError, DiagramParser};
pub use preprocessor::{
    IncludeExpandError, IncludeParseError, PreprocessError, Preprocessor, ProcedureExpandError,
    ProcedureParseError,
};
pub use sequence_parser::{PumlSequenceParser, SeqPumlDocument};
