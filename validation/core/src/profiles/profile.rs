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

use clap::ValueEnum;
use serde::de::DeserializeOwned;
use std::fs;

use crate::models::Errors;
use crate::readers::Reader;

#[derive(Copy, Clone, ValueEnum, Debug, PartialEq, Eq)]
pub enum Profile {
    #[value(name = "architectural-design")]
    ArchitecturalDesign,
    #[value(name = "dependable-element")]
    DependableElement,
}

impl Profile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArchitecturalDesign => "architectural-design",
            Self::DependableElement => "dependable-element",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::ArchitecturalDesign, Self::DependableElement]
    }
}

pub struct ProfileRun {
    pub ran_validator: bool,
    pub errors: Errors,
}

pub(super) fn read_input_bundle<T>(path: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read validation input bundle {path}: {e}"))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse validation input bundle {path}: {e}"))
}

pub(super) fn read_and_convert<R, O>(
    input: &R::Input,
    errors: &mut Errors,
    convert: impl Fn(R::Raw, &mut Errors) -> O,
) -> Result<Option<O>, String>
where
    R: Reader,
{
    if !R::is_present(input) {
        return Ok(None);
    }

    let raw = R::read(input).map_err(|e| e.to_string())?;
    Ok(Some(convert(raw, errors)))
}

pub(super) fn merge_errors(target: &mut Errors, incoming: Errors) {
    target.messages.extend(incoming.messages);
    if !incoming.debug_output.is_empty() {
        if !target.debug_output.is_empty() {
            target.debug_output.push_str("\n\n");
        }
        target.debug_output.push_str(&incoming.debug_output);
    }
}
