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

//! `diagram_links.json`: the links the docs build injects into diagrams.

use serde::{Deserialize, Serialize};

/// `alias` in diagram `source` links to diagram `target`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ElementLink {
    pub source: String,
    pub alias: String,
    pub target: String,
}

/// The call `method` on 1-based `line` of diagram `source` links to the first
/// of `targets` that is in a document.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CallLink {
    pub source: String,
    pub line: u32,
    pub method: String,
    pub targets: Vec<String>,
}

/// Sorted links of one dependable element.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagramLinks {
    pub version: u32,
    pub element_links: Vec<ElementLink>,
    pub call_links: Vec<CallLink>,
}

impl DiagramLinks {
    pub const VERSION: u32 = 1;

    pub fn new(mut element_links: Vec<ElementLink>, mut call_links: Vec<CallLink>) -> Self {
        element_links.sort();
        element_links.dedup();
        call_links.sort();
        call_links.dedup();
        Self {
            version: Self::VERSION,
            element_links,
            call_links,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("diagram links serialize to JSON")
    }
}
