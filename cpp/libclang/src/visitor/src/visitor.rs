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

use crate::class_visitor::ClassVisitor;
use crate::context::VisitContext;
use crate::enum_visitor::EnumVisitor;
use crate::function_visitor::FunctionVisitor;
use crate::source_filter;
use clang::{Entity, EntityKind};

pub trait AstVisitor {
    fn visit(ctx: &mut VisitContext, entity: Entity);

    fn get_namespace_id(entity: &Entity) -> Option<String> {
        namespace_id(entity)
    }
}

fn namespace_id(entity: &Entity) -> Option<String> {
    let mut stack: Vec<String> = vec![];
    let mut current = entity.get_semantic_parent();
    while let Some(parent) = current {
        if parent.get_kind() == EntityKind::Namespace {
            if let Some(name) = parent.get_name() {
                stack.push(name);
            }
        }
        current = parent.get_semantic_parent();
    }

    if stack.is_empty() {
        None
    } else {
        Some(stack.into_iter().rev().collect::<Vec<String>>().join("::"))
    }
}

pub struct Visitor<'a> {
    ctx: &'a mut VisitContext,
}

impl<'a> Visitor<'a> {
    pub fn new(ctx: &'a mut VisitContext) -> Self {
        Self { ctx }
    }

    pub fn visit(&mut self, entity: Entity) {
        self.visit_recursive(entity);
        ClassVisitor::resolve_relationships(self.ctx);
    }

    fn visit_recursive(&mut self, entity: Entity) {
        self.ctx.is_templated = false;
        if is_ignored_entity(entity) {
            return;
        }

        match entity.get_kind() {
            EntityKind::ClassDecl | EntityKind::StructDecl => {
                ClassVisitor::visit(self.ctx, entity);
            }
            EntityKind::ClassTemplate => {
                self.ctx.is_templated = true;
                ClassVisitor::visit(self.ctx, entity);
                // ClassTemplate parsing already processes all members,
                // so skip generic child recursion to avoid double-processing.
                return;
            }
            EntityKind::Method => FunctionVisitor::visit(self.ctx, entity),
            EntityKind::EnumDecl => EnumVisitor::visit(self.ctx, entity),
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
