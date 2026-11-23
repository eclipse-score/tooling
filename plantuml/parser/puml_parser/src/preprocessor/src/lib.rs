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

mod preprocessor;

pub use include_preprocessor::{
    Expanded, IncludeExpandError, IncludeExpander, IncludeParseError, IncludeSite, SourceLine,
};
pub use preprocessor::{PreprocessError, Preprocessed, Preprocessor};
pub use procedure_preprocessor::{
    ProcedureExpandError, ProcedureExpander, ProcedureFile, ProcedureParseError,
    ProcedureParserService,
};
