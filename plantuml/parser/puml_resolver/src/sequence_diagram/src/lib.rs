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

mod error;
mod lifecycle_validator;
mod participant_table;
mod sequence_resolver;
mod sequence_tree_builder;
mod statement_resolver;

pub use error::SequenceResolverError;
pub use sequence_resolver::SequenceResolver;
