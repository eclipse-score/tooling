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

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use clang::{Entity, EntityKind};
use log::warn;

use crate::clang_adapter::source_filter;
use crate::class_visitor::ClassVisitor;
use crate::context::{CallableDeclarationKey, SourceEntityKey, VisitContext};
use crate::enum_visitor::EnumVisitor;
use crate::function_visitor::FunctionVisitor;

/// Visitor interface for AST handlers that only depend on the shared output
/// context and the current entity.
///
/// Visitors that require per-traversal state should use an explicit entry point
/// instead of implementing this trait.
pub trait AstVisitor {
    fn visit(ctx: &mut VisitContext, entity: Entity);
}

/// Per-parser-execution cache for source-file contents.
///
/// The driver owns this temporary traversal state and shares it between
/// visitors for all translation units in one parser execution.
#[derive(Default)]
pub struct SourceFileCache {
    files: HashMap<PathBuf, Option<Vec<u8>>>,
}

pub(crate) fn normalize_source_identity_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

impl SourceFileCache {
    /// Returns source bytes, loading each path at most once during traversal.
    pub(crate) fn get(&mut self, path: &Path) -> Option<&[u8]> {
        self.files
            .entry(normalize_source_identity_path(path))
            .or_insert_with(|| std::fs::read(path).ok())
            .as_deref()
    }
}

pub struct Visitor<'a> {
    ctx: &'a mut VisitContext,
    source_files: &'a mut SourceFileCache,
    seen_free_function_declarations: &'a mut HashSet<CallableDeclarationKey>,
    seen_method_declarations: &'a mut HashSet<CallableDeclarationKey>,
    seen_function_definitions: &'a mut HashSet<SourceEntityKey>,
}

impl<'a> Visitor<'a> {
    pub fn new(
        ctx: &'a mut VisitContext,
        source_files: &'a mut SourceFileCache,
        seen_free_function_declarations: &'a mut HashSet<CallableDeclarationKey>,
        seen_method_declarations: &'a mut HashSet<CallableDeclarationKey>,
        seen_function_definitions: &'a mut HashSet<SourceEntityKey>,
    ) -> Self {
        Self {
            ctx,
            source_files,
            seen_free_function_declarations,
            seen_method_declarations,
            seen_function_definitions,
        }
    }

    pub fn visit(&mut self, entity: Entity) {
        self.visit_recursive(entity);
        ClassVisitor::resolve_relationships(self.ctx);
    }

    fn visit_recursive(&mut self, entity: Entity) {
        if source_filter::is_excluded_entity(&entity) {
            return;
        }

        match entity.get_kind() {
            EntityKind::ClassDecl | EntityKind::StructDecl => {
                ClassVisitor::visit(self.ctx, entity);
            }
            EntityKind::ClassTemplate | EntityKind::ClassTemplatePartialSpecialization => {
                ClassVisitor::visit(self.ctx, entity);
            }
            EntityKind::EnumDecl => EnumVisitor::visit(self.ctx, entity),
            EntityKind::FunctionDecl
            | EntityKind::FunctionTemplate
            | EntityKind::Method
            | EntityKind::Constructor
            | EntityKind::Destructor => {
                FunctionVisitor::visit_with_state(
                    self.ctx,
                    self.source_files,
                    self.seen_free_function_declarations,
                    self.seen_method_declarations,
                    self.seen_function_definitions,
                    entity,
                );
            }
            EntityKind::ConversionFunction => {
                warn!("Ignoring conversion function: {:?}", entity);
            }
            _ => {}
        }

        for child in entity.get_children() {
            self.visit_recursive(child);
        }
    }
}
