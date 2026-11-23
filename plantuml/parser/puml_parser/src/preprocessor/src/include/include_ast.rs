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
#[derive(Debug, Clone, PartialEq)]
pub enum IncludeSuffix {
    Label(String),
    Index(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncludeKind {
    Include,
    IncludeOnce,
    IncludeMany,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncludeStmt {
    Include {
        kind: IncludeKind,
        path: String,
        /// 1-based line of the directive.
        line: u32,
    },
    IncludeSub {
        path: String,
        suffix: IncludeSuffix,
        /// 1-based line of the directive.
        line: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubBlock {
    pub name: IncludeSuffix,
    pub content: Vec<IncludeFile>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncludeFile {
    Include(IncludeStmt),
    /// One line of text, with its 1-based line number in the file.
    Text {
        text: String,
        line: u32,
    },
    SubBlock(SubBlock),
}
