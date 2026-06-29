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

//! Unified validation library.
//!
//! This crate contains the shared models, readers, and validators used by the
//! CLI entrypoints for architecture and design verification.

mod models;
mod profiles;
mod readers;
mod validators;

pub use profiles::{read_profile_inputs, run_profile, Profile, ProfileInputs, ProfileRun};
