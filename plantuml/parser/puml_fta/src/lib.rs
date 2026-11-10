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

//! Fault-Tree-Analysis (FTA) model and emitters.
//!
//! Consumes the procedure parser's `ProcedureFile` (the stream of
//! `$FailureMode(...)` / `$RootCause(...)` / gate macro calls produced after
//! `fta_metamodel.puml` has been inlined), attributes root causes to failure
//! modes across diagrams ([`resolve_root_causes`]) and emits the generated
//! `RootCause` TRLC records ([`render_trlc_stub`]) and the review index with
//! the completeness findings ([`render_index`]).

mod model;
mod render;
mod resolve;

pub use model::{FtaError, FtaModel, FtaNode, NodeKind};
pub use render::{render_index, render_trlc_stub, INDEX_VERSION};
pub use resolve::{
    resolve_root_causes, FindingKind, FtaFinding, Resolution, RootCauseStub, SourceRef,
};
