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
use source_location::SourceLocation;
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

/// Formats an arrow from its parsed tokens.
///
/// This intentionally avoids using `Display`. The current `Arrow` structure
/// does not distinguish whether the line segment is located before or after
/// the middle part of the arrow, so a `Display` implementation may render an
/// ambiguous representation. Use this token-based formatting as a temporary
/// solution until the `Arrow` model preserves the original syntax layout.
pub fn format_arrow(arrow: &Arrow) -> String {
    format!(
        "{}{}{}",
        arrow.left.as_ref().map_or("", |d| d.raw.as_str()),
        arrow.line.raw,
        arrow.right.as_ref().map_or("", |d| d.raw.as_str()),
    )
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

// ---------- Shared Raw Element Identity ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementIdentity {
    pub name: Option<String>,       // text before "as", stripped of quotes
    pub alias: Option<String>,      // text after "as"
    pub stereotype: Option<String>, // inside << >>, stripped of delimiters
    pub element_kind: String,       // keyword verbatim: "component","class","participant"…
    pub source_location: SourceLocation,
}
