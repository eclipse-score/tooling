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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clang::{Entity, EntityKind};
use log::warn;

use crate::clang_adapter::scope::namespace_id;
use crate::clang_adapter::source_filter;
use crate::class_visitor::ClassVisitor;
use crate::context::VisitContext;
use crate::enum_visitor::EnumVisitor;
use crate::function_visitor::FunctionVisitor;

pub trait AstVisitor {
    fn visit(ctx: &mut VisitContext, entity: Entity);
}

/// Per-traversal cache for source-file contents.
///
/// The cache belongs to `Visitor` because it is temporary traversal state,
/// rather than part of the extracted semantic model.
#[derive(Default)]
pub(crate) struct SourceFileCache {
    files: HashMap<PathBuf, Option<Vec<u8>>>,
}

impl SourceFileCache {
    /// Returns source bytes, loading each path at most once during traversal.
    pub(crate) fn get(&mut self, path: &Path) -> Option<&[u8]> {
        self.files
            .entry(path.to_path_buf())
            .or_insert_with(|| std::fs::read(path).ok())
            .as_deref()
    }
}

pub struct Visitor<'a> {
    ctx: &'a mut VisitContext,
    source_files: SourceFileCache,
}

impl<'a> Visitor<'a> {
    pub fn new(ctx: &'a mut VisitContext) -> Self {
        Self {
            ctx,
            source_files: SourceFileCache::default(),
        }
    }

    pub fn visit(&mut self, entity: Entity) {
        self.visit_recursive(entity);
        ClassVisitor::resolve_relationships(self.ctx);
    }

    fn visit_recursive(&mut self, entity: Entity) {
        if is_ignored_entity(entity) {
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
            EntityKind::FunctionDecl | EntityKind::Method => {
                FunctionVisitor::visit_with_source_files(self.ctx, &mut self.source_files, entity);
            }
            EntityKind::FunctionTemplate => {
                // TBD: Handle function templates if needed
            }
            EntityKind::Constructor | EntityKind::Destructor | EntityKind::ConversionFunction => {
                warn!(
                    "Ignoring constructor, destructor, or conversion function: {:?}",
                    entity
                );
            }
            _ => {}
        }

        for child in entity.get_children() {
            self.visit_recursive(child);
        }
    }
}

fn is_ignored_entity(entity: Entity) -> bool {
    if let Some(location) = entity.get_location() {
        let (file, _line, _column) = location.get_presumed_location();
        source_filter::is_external_or_system_path(&file)
            || source_filter::is_excluded_namespace(namespace_id(&entity).as_deref())
    } else {
        false
    }
}
