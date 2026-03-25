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
use serde::{Deserialize, Serialize};
use std::default::Default;

////////////////////////////////////////////////////////////////////////////////
// Arrow
////////////////////////////////////////////////////////////////////////////////
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
pub struct Arrow {
    pub left: Option<ArrowDecor>,
    pub line: ArrowLine,
    pub middle: Option<ArrowMiddle>,
    pub right: Option<ArrowDecor>,
}

// ---------- Decorator ----------
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ArrowDecor {
    pub raw: String,
}

// ---------- Line ----------
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
pub struct ArrowLine {
    pub raw: String,
}

// ---------- Middle ----------
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ArrowMiddle {
    pub style: Option<ArrowStyle>,
    pub direction: Option<ArrowDirection>,
    pub decorator: Option<String>,
}

// ---------- Style ----------
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
pub struct ArrowStyle {
    pub color: Option<String>,
    pub patterns: Vec<String>,
    pub thickness: Option<u32>,
    pub extra_attrs: Vec<String>,
}

// ---------- Direction ----------
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum ArrowDirection {
    Up,
    Down,
    Left,
    Right,
}
