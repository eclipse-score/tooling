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

//! Extracts C++ callable definitions via libclang into [`VisitContext::functions`].
//! calls, branches and loops appear in execution order relative to one another.

use clang::{Entity, EntityKind};
use cpp_semantics::{
    BodyItem, BranchCase, FunctionDef, FunctionId, FunctionKind, GuardExpression, LoopKind,
};

use crate::clang_adapter::scope::callable_scope;
use crate::clang_adapter::source_location::{is_in_main_file, parse_source_location};
use crate::types::resolver::resolve_type;
use crate::visitor::SourceFileCache;
use crate::{context::VisitContext, AstVisitor};

pub struct FunctionVisitor;

/// Semantic roles assigned to the direct children of a supported libclang `IfStmt`.
struct IfParts<'tu> {
    condition: Entity<'tu>,
    then_body: Entity<'tu>,
    else_body: Option<Entity<'tu>>,
}

impl AstVisitor for FunctionVisitor {
    fn visit(ctx: &mut VisitContext, entity: Entity) {
        let mut source_files = SourceFileCache::default();
        Self::visit_with_source_files(ctx, &mut source_files, entity);
    }
}

impl FunctionVisitor {
    /// Extracts a callable using source-text resources owned by the traversal.
    pub(crate) fn visit_with_source_files(
        ctx: &mut VisitContext,
        source_files: &mut SourceFileCache,
        entity: Entity,
    ) {
        if let Some(func_def) = Self::extract_function_def(source_files, entity) {
            ctx.functions.push(func_def);
        }
    }

    // ── Top-level extraction ──────────────────────────────────────────────────

    fn extract_function_def(
        source_files: &mut SourceFileCache,
        entity: Entity,
    ) -> Option<FunctionDef> {
        if !is_in_main_file(&entity) {
            log::debug!(
                "skipping callable '{}': not located in the main file",
                entity.get_name().unwrap_or_default()
            );
            return None;
        }

        let Some(id) = Self::extract_function_id(&entity) else {
            log::debug!(
                "skipping callable '{}': no supported function identity",
                entity.get_name().unwrap_or_default()
            );
            return None;
        };

        let Some(kind) = Self::extract_function_kind(&entity) else {
            log::debug!(
                "skipping callable '{}': unsupported callable kind {:?}",
                id.qualified_name(),
                entity.get_kind()
            );
            return None;
        };

        let Some(body) = Self::process_function_body(source_files, entity, &id) else {
            log::debug!(
                "skipping callable '{}': no compound statement body (declaration-only?)",
                id.qualified_name()
            );
            return None;
        };

        let return_type = if matches!(kind, FunctionKind::Constructor | FunctionKind::Destructor) {
            None
        } else {
            entity.get_result_type().map(|t| resolve_type(&t))
        };

        Some(FunctionDef {
            id,
            kind,
            return_type,
            body,
        })
    }

    // ── AST navigation helpers ────────────────────────────────────────────────

    fn extract_function_id(entity: &Entity) -> Option<FunctionId> {
        Some(FunctionId {
            scope: callable_scope(entity)?,
            name: entity.get_name()?,
        })
    }

    fn get_children(entity: Entity) -> Vec<Entity> {
        let mut v = Vec::new();
        entity.visit_children(|child, _| {
            v.push(child);
            clang::EntityVisitResult::Continue
        });
        v
    }

    /// Returns an expression's original source-range text when available.
    ///
    /// Libclang locations expose byte offsets into the source file, so this
    /// preserves the author's whitespace and operator spelling.
    fn extract_expression_text(source_files: &mut SourceFileCache, entity: Entity) -> String {
        entity
            .get_range()
            .and_then(|range| {
                let start = range.get_start().get_file_location();
                let end = range.get_end().get_file_location();
                let file = start.file?;
                let source = source_files.get(&file.get_path())?;
                let start_offset = start.offset as usize;
                let end_offset = end.offset as usize;

                source
                    .get(start_offset..end_offset)
                    .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
            })
            .unwrap_or_default()
    }

    fn extract_function_kind(entity: &Entity) -> Option<FunctionKind> {
        match entity.get_kind() {
            EntityKind::FunctionDecl => Some(FunctionKind::Free),
            EntityKind::Method => Some(if entity.is_static_method() {
                FunctionKind::StaticMethod
            } else {
                FunctionKind::Method
            }),
            EntityKind::Constructor => Some(FunctionKind::Constructor),
            EntityKind::Destructor => Some(FunctionKind::Destructor),
            EntityKind::ConversionFunction => Some(FunctionKind::Conversion),
            _ => None,
        }
    }

    /// Resolves a call expression to its semantic callable target.
    fn extract_call_target(call_expr: Entity) -> Option<FunctionId> {
        // Direct reference works for simple `obj.method()` calls.
        // For virtual/pointer calls (`ptr->method()`), the reference lives on the
        // MemberRefExpr child — fall back to that when the direct lookup returns None.
        let resolved = call_expr.get_reference().or_else(|| {
            Self::get_children(call_expr)
                .into_iter()
                .find(|c| c.get_kind() == EntityKind::MemberRefExpr)
                .and_then(|c| c.get_reference())
        })?;

        Self::extract_function_kind(&resolved)?;
        Self::extract_function_id(&resolved)
    }

    fn is_cross_owner_call(caller: &FunctionId, callee: &FunctionId) -> bool {
        callee.scope != caller.scope
    }

    // ── Scope/branch processors ───────────────────────────────────────────────

    /// Locates a callable's compound body and processes its statements.
    fn process_function_body(
        source_files: &mut SourceFileCache,
        function: Entity,
        caller: &FunctionId,
    ) -> Option<Vec<BodyItem>> {
        let body = Self::get_children(function)
            .into_iter()
            .find(|child| child.get_kind() == EntityKind::CompoundStmt)?;

        Some(Self::process_compound(source_files, body, caller))
    }

    /// Processes the direct statements of a `CompoundStmt` in source order.
    fn process_compound(
        source_files: &mut SourceFileCache,
        compound: Entity,
        caller: &FunctionId,
    ) -> Vec<BodyItem> {
        Self::get_children(compound)
            .into_iter()
            .flat_map(|statement| Self::process_statement(source_files, statement, caller))
            .collect()
    }

    /// Processes one statement, preserving nested control-flow structure.
    fn process_statement(
        source_files: &mut SourceFileCache,
        entity: Entity,
        caller: &FunctionId,
    ) -> Vec<BodyItem> {
        match entity.get_kind() {
            EntityKind::CompoundStmt => Self::process_compound(source_files, entity, caller),
            EntityKind::IfStmt => Self::process_if(source_files, entity, caller),
            EntityKind::ForStmt | EntityKind::WhileStmt | EntityKind::DoStmt => {
                Self::process_loop(source_files, entity, caller)
            }
            _ => Self::collect_nested_calls(entity, caller),
        }
    }

    /// Turns an IfStmt into one [`BodyItem::Branch`] with ordered cases.
    ///
    /// `else if` chains are flattened into cases, while an `else` that contains
    /// a nested `if` remains a final else case containing a nested Branch.
    fn process_if(
        source_files: &mut SourceFileCache,
        if_entity: Entity,
        caller: &FunctionId,
    ) -> Vec<BodyItem> {
        match Self::collect_branch_cases(source_files, if_entity, caller) {
            Some(cases) => vec![BodyItem::Branch { cases }],
            None => Self::process_if_fallback(source_files, if_entity, caller),
        }
    }

    /// Collects the ordered cases of an if/else-if/else chain.
    fn collect_branch_cases(
        source_files: &mut SourceFileCache,
        if_entity: Entity,
        caller: &FunctionId,
    ) -> Option<Vec<BranchCase>> {
        let parts = Self::split_if_parts(if_entity)?;
        let mut cases = vec![BranchCase {
            guard: Some(Self::extract_guard_expression(
                source_files,
                parts.condition,
                caller,
            )),
            body: Self::process_statement(source_files, parts.then_body, caller),
            source_location: parse_source_location(&if_entity),
        }];

        if let Some(else_body) = parts.else_body {
            if else_body.get_kind() == EntityKind::IfStmt {
                cases.extend(Self::collect_branch_cases(source_files, else_body, caller)?);
            } else {
                cases.push(BranchCase {
                    guard: None,
                    body: Self::process_statement(source_files, else_body, caller),
                    source_location: parse_source_location(&else_body),
                });
            }
        }

        Some(cases)
    }

    /// Maps the supported direct-child layout of an `IfStmt` to semantic roles.
    ///
    /// The current layout is `[condition, then_body, else_body?]`. More complex
    /// forms, such as C++17 `if` statements with an initializer, use the
    /// conservative no-data-loss fallback until their child layout is modeled.
    fn split_if_parts(if_entity: Entity<'_>) -> Option<IfParts<'_>> {
        let children = Self::get_children(if_entity);

        match children.as_slice() {
            [condition, then_body] => Some(IfParts {
                condition: *condition,
                then_body: *then_body,
                else_body: None,
            }),
            [condition, then_body, else_body] => Some(IfParts {
                condition: *condition,
                then_body: *then_body,
                else_body: Some(*else_body),
            }),
            _ => {
                log::warn!(
                    "using fallback for IfStmt with unsupported direct-child layout: {} children",
                    children.len()
                );
                None
            }
        }
    }

    /// Preserves reachable nested calls when an `IfStmt` layout is unsupported.
    ///
    /// The fallback deliberately does not invent a condition or branch shape;
    /// it traverses all direct children so an unsupported cursor never causes
    /// its entire subtree to disappear from the extracted model.
    fn process_if_fallback(
        source_files: &mut SourceFileCache,
        if_entity: Entity,
        caller: &FunctionId,
    ) -> Vec<BodyItem> {
        log::warn!(
            "falling back to unstructured processing for IfStmt at {:?}",
            parse_source_location(&if_entity)
        );

        Self::get_children(if_entity)
            .into_iter()
            .flat_map(|child| Self::process_statement(source_files, child, caller))
            .collect()
    }

    /// Extracts a condition as a tree that preserves `&&`, `||`, and `!`
    /// short-circuit semantics. Other expressions remain source-backed leaves.
    fn extract_guard_expression(
        source_files: &mut SourceFileCache,
        entity: Entity,
        caller: &FunctionId,
    ) -> GuardExpression {
        match entity.get_kind() {
            EntityKind::CallExpr => {
                if let Some(target) = Self::extract_call_target(entity)
                    .filter(|target| Self::is_cross_owner_call(caller, target))
                {
                    return GuardExpression::Call {
                        target: target.qualified_name(),
                        text: Self::extract_expression_text(source_files, entity),
                        source_location: parse_source_location(&entity),
                    };
                }
            }
            EntityKind::UnaryOperator if Self::has_leading_operator(entity, "!") => {
                if let Some(expression) = Self::get_children(entity).into_iter().next() {
                    return GuardExpression::Not {
                        expression: Box::new(Self::extract_guard_expression(
                            source_files,
                            expression,
                            caller,
                        )),
                    };
                }
            }
            EntityKind::BinaryOperator => {
                let children = Self::get_children(entity);
                if let [left, right] = children.as_slice() {
                    if let Some(operator) = Self::logical_operator(entity, *left, *right) {
                        return Self::combine_guard_expressions(
                            operator,
                            Self::extract_guard_expression(source_files, *left, caller),
                            Self::extract_guard_expression(source_files, *right, caller),
                        );
                    }
                }
            }
            EntityKind::ParenExpr | EntityKind::UnexposedExpr => {
                let children = Self::get_children(entity);
                if let [expression] = children.as_slice() {
                    return Self::extract_guard_expression(source_files, *expression, caller);
                }
            }
            _ => {}
        }

        GuardExpression::Opaque {
            text: Self::extract_expression_text(source_files, entity),
            source_location: parse_source_location(&entity),
        }
    }

    fn combine_guard_expressions(
        operator: &str,
        left: GuardExpression,
        right: GuardExpression,
    ) -> GuardExpression {
        match operator {
            "&&" => GuardExpression::And {
                expressions: Self::flatten_guard_expressions(left, right, |expression| {
                    matches!(expression, GuardExpression::And { .. })
                }),
            },
            "||" => GuardExpression::Or {
                expressions: Self::flatten_guard_expressions(left, right, |expression| {
                    matches!(expression, GuardExpression::Or { .. })
                }),
            },
            _ => unreachable!("only logical operators are combined"),
        }
    }

    fn flatten_guard_expressions<F>(
        left: GuardExpression,
        right: GuardExpression,
        is_same_operator: F,
    ) -> Vec<GuardExpression>
    where
        F: Fn(&GuardExpression) -> bool,
    {
        let mut expressions = Vec::new();
        for expression in [left, right] {
            if is_same_operator(&expression) {
                match expression {
                    GuardExpression::And {
                        expressions: nested,
                    }
                    | GuardExpression::Or {
                        expressions: nested,
                    } => expressions.extend(nested),
                    _ => unreachable!("matching guard expression must be logical"),
                }
            } else {
                expressions.push(expression);
            }
        }
        expressions
    }

    /// Returns the logical operator located between a binary cursor's direct
    /// left and right operands. This avoids interpreting an operator nested in
    /// either operand, including template arguments and `operator&&` calls, as
    /// the current cursor's operator.
    fn logical_operator(entity: Entity, left: Entity, right: Entity) -> Option<&'static str> {
        let left_end = left.get_range()?.get_end().get_file_location();
        let right_start = right.get_range()?.get_start().get_file_location();
        let file = left_end.file?;

        if right_start.file != Some(file) || left_end.offset > right_start.offset {
            return None;
        }

        entity
            .get_range()?
            .tokenize()
            .into_iter()
            .find_map(|token| {
                let location = token.get_location().get_file_location();
                (location.file == Some(file)
                    && (left_end.offset..right_start.offset).contains(&location.offset))
                .then(|| match token.get_spelling().as_str() {
                    "&&" | "and" => Some("&&"),
                    "||" | "or" => Some("||"),
                    _ => None,
                })
                .flatten()
            })
    }

    fn has_leading_operator(entity: Entity, operator: &str) -> bool {
        entity
            .get_range()
            .and_then(|range| range.tokenize().into_iter().next())
            .is_some_and(|token| {
                token.get_spelling() == operator
                    || (operator == "!" && token.get_spelling() == "not")
            })
    }

    /// Collects cross-owner calls in `entity`, without crossing control-flow
    /// boundaries. Calls are emitted post-order, so nested calls precede their
    /// enclosing call. This is structural nesting order, not a claim about the
    /// evaluation order of sibling C++ call arguments.
    fn collect_nested_calls(entity: Entity, caller: &FunctionId) -> Vec<BodyItem> {
        match entity.get_kind() {
            EntityKind::IfStmt
            | EntityKind::ForStmt
            | EntityKind::WhileStmt
            | EntityKind::DoStmt => Vec::new(),
            EntityKind::CallExpr => {
                let mut calls: Vec<_> = Self::get_children(entity)
                    .into_iter()
                    .flat_map(|child| Self::collect_nested_calls(child, caller))
                    .collect();

                if let Some(target) = Self::extract_call_target(entity) {
                    if Self::is_cross_owner_call(caller, &target) {
                        calls.push(BodyItem::Call {
                            target: target.qualified_name(),
                            source_location: parse_source_location(&entity),
                        });
                    }
                }

                calls
            }
            _ => Self::get_children(entity)
                .into_iter()
                .flat_map(|child| Self::collect_nested_calls(child, caller))
                .collect(),
        }
    }

    /// Turns a loop statement into its single [`BodyItem::Loop`] representation.
    fn process_loop(
        source_files: &mut SourceFileCache,
        loop_entity: Entity,
        caller: &FunctionId,
    ) -> Vec<BodyItem> {
        let kind = match loop_entity.get_kind() {
            EntityKind::ForStmt => LoopKind::For,
            EntityKind::WhileStmt => LoopKind::While,
            EntityKind::DoStmt => LoopKind::DoWhile,
            _ => unreachable!("only loop statements are processed as loops"),
        };

        let parts = Self::get_children(loop_entity);
        let body_idx = match loop_entity.get_kind() {
            EntityKind::DoStmt => 0usize,
            _ => parts.len().saturating_sub(1),
        };

        let body = parts
            .get(body_idx)
            .map(|&b| Self::process_statement(source_files, b, caller))
            .unwrap_or_default();

        vec![BodyItem::Loop {
            kind,
            body,
            source_location: parse_source_location(&loop_entity),
        }]
    }
}
