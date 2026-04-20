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

//! Input readers for Bazel JSON and PlantUML-derived FlatBuffer artifacts.

mod bazel_reader;
mod class_diagram_reader;
mod component_diagram_reader;

pub trait InputPresence {
    fn is_present(&self) -> bool;
}

impl InputPresence for str {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

impl<T> InputPresence for [T] {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

/// File-reading contract for raw input artifacts.
pub trait Reader {
    type Input: ?Sized + InputPresence;
    type Raw;
    type Error: std::fmt::Display;

    fn is_present(input: &Self::Input) -> bool {
        input.is_present()
    }
    fn read(input: &Self::Input) -> Result<Self::Raw, Self::Error>;
}

pub use bazel_reader::BazelReader;
pub use class_diagram_reader::ClassDiagramReader;
pub use component_diagram_reader::ComponentDiagramReader;
