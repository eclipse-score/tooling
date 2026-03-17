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
    Include { kind: IncludeKind, path: String },
    IncludeSub { path: String, suffix: IncludeSuffix },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubBlock {
    pub name: IncludeSuffix,
    pub content: Vec<PreprocessStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreprocessStmt {
    Include(IncludeStmt),
    Text(String),
    SubBlock(SubBlock),
}

impl PreprocessStmt {
    pub fn render(&self, out: &mut String) {
        match self {
            PreprocessStmt::Text(text) => out.push_str(text),
            PreprocessStmt::SubBlock(sub) => {
                for stmt in &sub.content {
                    stmt.render(out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_text_node_outputs_text() {
        let stmt = PreprocessStmt::Text("hello".into());
        let mut out = String::new();
        stmt.render(&mut out);
        assert_eq!(out, "hello");
    }

    #[test]
    fn render_subblock_with_text_nodes() {
        let sub = SubBlock {
            name: IncludeSuffix::Label("sub".into()),
            content: vec![
                PreprocessStmt::Text("a\n".into()),
                PreprocessStmt::Text("b\n".into()),
            ],
        };
        let stmt = PreprocessStmt::SubBlock(sub);
        let mut out = String::new();
        stmt.render(&mut out);
        assert_eq!(out, "a\nb\n");
    }

    #[test]
    fn render_empty_subblock_outputs_nothing() {
        let sub = SubBlock {
            name: IncludeSuffix::Label("empty".into()),
            content: vec![],
        };
        let stmt = PreprocessStmt::SubBlock(sub);
        let mut out = String::new();
        stmt.render(&mut out);
        assert_eq!(out, "");
    }

    #[test]
    fn render_nested_subblocks() {
        let inner_sub = SubBlock {
            name: IncludeSuffix::Label("inner".into()),
            content: vec![PreprocessStmt::Text("inner\n".into())],
        };
        let outer_sub = SubBlock {
            name: IncludeSuffix::Label("outer".into()),
            content: vec![
                PreprocessStmt::Text("start\n".into()),
                PreprocessStmt::SubBlock(inner_sub),
                PreprocessStmt::Text("end\n".into()),
            ],
        };
        let stmt = PreprocessStmt::SubBlock(outer_sub);
        let mut out = String::new();
        stmt.render(&mut out);
        assert_eq!(out, "start\ninner\nend\n");
    }

    #[test]
    fn render_include_stmt_does_not_panic() {
        let include = PreprocessStmt::Include(IncludeStmt::Include {
            kind: IncludeKind::Include,
            path: "file.puml".into(),
        });
        let mut out = String::new();
        include.render(&mut out);
        assert_eq!(out, "");
    }

    #[test]
    fn render_include_sub_stmt_does_not_panic() {
        let include_sub = PreprocessStmt::Include(IncludeStmt::IncludeSub {
            path: "file.puml".into(),
            suffix: IncludeSuffix::Label("sub".into()),
        });
        let mut out = String::new();
        include_sub.render(&mut out);
        assert_eq!(out, "");
    }
}
