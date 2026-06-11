///////////////////////////////////////////////////////////////////////////////////
// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
////////////////////////////////////////////////////////////////////////////////////

//! Visits C++ method entities via libclang and populates [`VisitContext::functions`].
//!
//! Each method body is represented as an ordered [`Vec<BodyItem>`] so that
//! calls, branches and loops appear in execution order relative to one another.

use crate::{context::VisitContext, AstVisitor};
use clang::{Entity, EntityKind};
use sequence_logic::{BodyItem, FunctionDef};

pub struct FunctionVisitor;

impl AstVisitor for FunctionVisitor {
    fn visit(ctx: &mut VisitContext, entity: Entity) {
        if let Some(func_def) = Self::extract_function_def(entity) {
            ctx.functions.push(func_def);
        }
    }
}

impl FunctionVisitor {
    // ── Top-level extraction ──────────────────────────────────────────────────

    fn extract_function_def(entity: Entity) -> Option<FunctionDef> {
        let body_node = Self::get_method_body(entity)?;

        if !entity
            .get_location()
            .map(|loc| loc.is_in_main_file())
            .unwrap_or(false)
        {
            return None;
        }

        let method_name = entity.get_name()?;
        let class_name = entity
            .get_semantic_parent()
            .and_then(|p| p.get_name())
            .unwrap_or_default();

        if class_name.is_empty() {
            return None;
        }

        let body = Self::process_scope(body_node, &class_name);
        let return_type = entity
            .get_result_type()
            .map(|t| t.get_display_name())
            .unwrap_or_else(|| "?".to_string());

        Some(FunctionDef {
            class: class_name,
            name: method_name,
            return_type,
            body,
        })
    }

    // ── AST navigation helpers ────────────────────────────────────────────────

    fn get_method_body(entity: Entity) -> Option<Entity> {
        Self::get_children(entity)
            .into_iter()
            .find(|c| c.get_kind() == EntityKind::CompoundStmt)
    }

    fn get_children(entity: Entity) -> Vec<Entity> {
        let mut v = Vec::new();
        entity.visit_children(|child, _| {
            v.push(child);
            clang::EntityVisitResult::Continue
        });
        v
    }

    /// Walk down through wrapper nodes to find the first non-empty name.
    /// Used to extract condition variable names from an IfStmt condition child.
    fn extract_first_name(entity: Entity) -> String {
        if let Some(name) = entity.get_name() {
            if !name.is_empty() {
                return name;
            }
        }
        for child in Self::get_children(entity) {
            let name = Self::extract_first_name(child);
            if !name.is_empty() {
                return name;
            }
        }
        String::new()
    }

    /// If `call_expr` is a call to a method owned by a class OTHER than `owner`,
    /// return `(callee_class, method_name)` (or `"constructor"` for constructors).
    fn cross_class_call_name(call_expr: Entity, owner: &str) -> Option<(String, String)> {
        // Direct reference works for simple `obj.method()` calls.
        // For virtual/pointer calls (`ptr->method()`), the reference lives on the
        // MemberRefExpr child — fall back to that when the direct lookup returns None.
        let resolved = call_expr.get_reference().or_else(|| {
            Self::get_children(call_expr)
                .into_iter()
                .find(|c| c.get_kind() == EntityKind::MemberRefExpr)
                .and_then(|c| c.get_reference())
        })?;
        let parent = resolved.get_semantic_parent()?;

        let is_class_like = matches!(
            parent.get_kind(),
            EntityKind::ClassDecl
                | EntityKind::StructDecl
                | EntityKind::ClassTemplate
                | EntityKind::ClassTemplatePartialSpecialization // | EntityKind::ClassTemplateSpecialization
        );
        if !is_class_like {
            return None;
        }

        let parent_name = parent.get_name().unwrap_or_default();
        if parent_name.is_empty() || parent_name == owner {
            return None;
        }

        if matches!(
            resolved.get_kind(),
            EntityKind::Constructor | EntityKind::Destructor
        ) {
            return Some((parent_name, "constructor".to_string()));
        }

        resolved.get_name().map(|n| (parent_name, n))
    }

    // ── Scope/branch processors ───────────────────────────────────────────────

    /// Walk the subtree of `entity` collecting cross-class calls as [`BodyItem::Call`]
    /// entries, skipping if/loop boundaries. Post-order on `CallExpr`: argument
    /// calls appear before the outer call (execution order).
    fn collect_calls_no_if(entity: Entity, owner: &str, out: &mut Vec<BodyItem>) {
        for child in Self::get_children(entity) {
            match child.get_kind() {
                EntityKind::IfStmt
                | EntityKind::ForStmt
                | EntityKind::WhileStmt
                | EntityKind::DoStmt => {}
                EntityKind::CallExpr => {
                    Self::collect_calls_no_if(child, owner, out);
                    if let Some((callee, name)) = Self::cross_class_call_name(child, owner) {
                        out.push(BodyItem::Call { callee, name });
                    }
                }
                _ => Self::collect_calls_no_if(child, owner, out),
            }
        }
    }

    /// Process a `CompoundStmt` (or any scope entity) and return an ordered list of
    /// [`BodyItem`]s that reflects the source execution order: calls, branches and
    /// loops appear interleaved exactly as they do in the code.
    fn process_scope(entity: Entity, owner: &str) -> Vec<BodyItem> {
        let mut body: Vec<BodyItem> = Vec::new();

        entity.visit_children(|child, _| {
            match child.get_kind() {
                EntityKind::IfStmt => {
                    Self::process_if(child, owner, &mut body);
                    clang::EntityVisitResult::Continue
                }
                EntityKind::ForStmt | EntityKind::WhileStmt | EntityKind::DoStmt => {
                    body.push(Self::process_loop(child, owner));
                    clang::EntityVisitResult::Continue
                }
                EntityKind::CallExpr => {
                    // Post-order: emit argument calls before the outer call.
                    Self::collect_calls_no_if(child, owner, &mut body);
                    if let Some((callee, name)) = Self::cross_class_call_name(child, owner) {
                        body.push(BodyItem::Call { callee, name });
                    }
                    clang::EntityVisitResult::Continue
                }
                _ => {
                    Self::collect_calls_no_if(child, owner, &mut body);
                    clang::EntityVisitResult::Continue
                }
            }
        });

        body
    }

    /// Turn an IfStmt into one or more [`BodyItem::Branch`] entries (one per arm).
    fn process_if(if_entity: Entity, owner: &str, out: &mut Vec<BodyItem>) {
        let parts = Self::get_children(if_entity);
        // IfStmt children: [condition_expr, then_body, (else_body)?]
        let cond_text = parts
            .first()
            .map(|&c| Self::extract_first_name(c))
            .unwrap_or_default();

        // Collect any cross-class calls embedded inside the condition expression
        // (e.g. `if (!plugin->WaitUntilLoaded(...))` — the call is in the condition,
        // not the body, so it must be captured here before the Branch is created).
        if let Some(&cond_ent) = parts.first() {
            Self::collect_calls_no_if(cond_ent, owner, out);
        }

        if let Some(&then_ent) = parts.get(1) {
            out.push(BodyItem::Branch {
                condition: cond_text,
                body: Self::process_body(then_ent, owner),
            });
        }

        if let Some(&else_ent) = parts.get(2) {
            if else_ent.get_kind() == EntityKind::IfStmt {
                Self::process_if(else_ent, owner, out);
            } else {
                out.push(BodyItem::Branch {
                    condition: "else".to_string(),
                    body: Self::process_body(else_ent, owner),
                });
            }
        }
    }

    /// Process a branch body, dispatching on kind.
    fn process_body(entity: Entity, owner: &str) -> Vec<BodyItem> {
        match entity.get_kind() {
            EntityKind::CompoundStmt => Self::process_scope(entity, owner),
            EntityKind::IfStmt => {
                let mut body = Vec::new();
                Self::process_if(entity, owner, &mut body);
                body
            }
            _ => {
                let mut body = Vec::new();
                Self::collect_calls_no_if(entity, owner, &mut body);
                body
            }
        }
    }

    /// Turn a loop statement into a [`BodyItem::Loop`].
    fn process_loop(loop_entity: Entity, owner: &str) -> BodyItem {
        let kind = match loop_entity.get_kind() {
            EntityKind::ForStmt => "for",
            EntityKind::WhileStmt => "while",
            EntityKind::DoStmt => "do_while",
            _ => "unknown",
        }
        .to_string();

        let parts = Self::get_children(loop_entity);
        let body_idx = match loop_entity.get_kind() {
            EntityKind::DoStmt => 0usize,
            _ => parts.len().saturating_sub(1),
        };

        let body = parts
            .get(body_idx)
            .map(|&b| Self::process_body(b, owner))
            .unwrap_or_default();

        BodyItem::Loop { kind, body }
    }
}
