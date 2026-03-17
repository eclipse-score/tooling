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
use crate::class_ast::{Attribute, Method, Name};

pub trait TypeDef {
    fn name_mut(&mut self) -> &mut Name;
    fn attributes_mut(&mut self) -> &mut Vec<Attribute>;
    fn methods_mut(&mut self) -> &mut Vec<Method>;
}

pub trait WritableName {
    fn write_name(&mut self, internal: impl Into<String>, display: Option<impl Into<String>>);
}
