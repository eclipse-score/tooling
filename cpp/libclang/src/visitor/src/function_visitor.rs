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
//! Preserves structured calls, branches, and loops for supported AST shapes,
//! and falls back to conservative traversal for unsupported control-flow forms.

use clang::{Entity, EntityKind, ExceptionSpecification};
use class_diagram::{FreeFunctionDecl, Method, MethodModifier};
use cpp_semantics::{
    BodyItem, BranchCase, FunctionDef, FunctionId, FunctionKind, GuardExpression, LoopKind,
    ResolvedType, Scope,
};
use std::collections::HashSet;

use crate::callable_declaration::{
    parse_callable_parameters, parse_callable_return_type, parse_template_parameters,
};
use crate::clang_adapter::scope::{
    callable_scope, has_translation_unit_local_linkage, namespace_id,
};
use crate::clang_adapter::source_filter;
use crate::clang_adapter::source_location::parse_source_location;
use crate::class_visitor::{parse_visibility, ClassVisitor};
use crate::context::{
    CallableDeclarationKey, CallableLinkageScope, CallableOwnerKey, CallableSignatureKey,
    ExtractedFreeFunctionDeclaration, ExtractedFunction, ExtractedMethodDeclaration,
    ParsedMethodType, SourceEntityKey,
};
use crate::types::resolver::resolve_type;
use crate::visitor::{normalize_source_identity_path, SourceFileCache};
use crate::VisitContext;

pub struct FunctionVisitor;

/// Semantic roles assigned to the direct children of a supported libclang `IfStmt`.
struct IfParts<'tu> {
    condition: Entity<'tu>,
    then_body: Entity<'tu>,
    else_body: Option<Entity<'tu>>,
}

impl FunctionVisitor {
    /// Extracts a callable using traversal-scoped state and source-text resources.
    pub(crate) fn visit_with_state(
        ctx: &mut VisitContext,
        source_files: &mut SourceFileCache,
        seen_free_function_declarations: &mut HashSet<CallableDeclarationKey>,
        seen_method_declarations: &mut HashSet<CallableDeclarationKey>,
        seen_function_definitions: &mut HashSet<SourceEntityKey>,
        entity: Entity,
    ) {
        let Some((function_id, function_kind)) = Self::extract_callable(&entity) else {
            return;
        };

        match &function_id.scope {
            Scope::Type { .. } => {
                if let Some(declaration) =
                    Self::extract_method_declaration(&entity, &function_id, function_kind)
                {
                    ClassVisitor::register_method_declaration(
                        ctx,
                        seen_method_declarations,
                        declaration,
                    );
                } else {
                    log::debug!(
                        "skipping type-scoped callable '{}': unsupported function kind {:?}",
                        function_id.qualified_name(),
                        function_kind
                    );
                }
            }
            Scope::Global | Scope::Namespace(_) => {
                if let Some(declaration) = Self::extract_free_function_declaration(
                    seen_free_function_declarations,
                    &entity,
                    &function_id,
                ) {
                    ctx.free_function_declarations.push(declaration);
                }
            }
        }

        if let Some(function) = Self::extract_function_def(
            entity,
            function_id,
            function_kind,
            source_files,
            seen_function_definitions,
        ) {
            ctx.functions.push(function);
        }
    }

    // ── Top-level extraction ──────────────────────────────────────────────────

    fn extract_method_declaration(
        entity: &Entity,
        id: &FunctionId,
        kind: FunctionKind,
    ) -> Option<ExtractedMethodDeclaration> {
        if !matches!(
            kind,
            FunctionKind::Method
                | FunctionKind::StaticMethod
                | FunctionKind::Constructor
                | FunctionKind::Destructor
        ) {
            return None;
        }

        let parsed_parameters = parse_callable_parameters(entity);
        let class_id = id.scope.qualified_name();

        let return_type = entity
            .get_result_type()
            .map(|ty| resolve_type(&ty))
            .unwrap_or_else(|| ResolvedType::Builtin("void".to_string()));
        let method_type = ParsedMethodType {
            name: id.name.clone(),
            return_type: return_type.clone(),
            parameter_types: parsed_parameters.parameter_types.clone(),
            source_location: parse_source_location(entity),
        };

        let is_override_method = entity
            .get_overridden_methods()
            .is_some_and(|methods| !methods.is_empty());
        let is_final_method = entity
            .get_children()
            .into_iter()
            .any(|child| child.get_kind() == EntityKind::FinalAttr);

        // Only the bare `noexcept` specifier is modeled (mirrors the PlantUML grammar, which has
        // no support for the conditional `noexcept(expr)` form). Requiring `BasicNoexcept` filters
        // out `noexcept(expr)`, but on its own it isn't enough: for an implicit/defaulted special
        // member (e.g. `~Foo() = default;` with no written specifier at all), the compiler-computed
        // specification also resolves to `BasicNoexcept` once evaluated -- and that evaluation is
        // lazily triggered by unrelated code (e.g. a derived class use), making it unstable. So this
        // also requires the literal `noexcept` token to appear in the declarator (the tokens up to
        // the first `{` or `;`), which excludes both that case and `noexcept` written inside a
        // lambda in the method body.
        let has_noexcept_token = entity.get_range().is_some_and(|range| {
            range
                .tokenize()
                .iter()
                .take_while(|token| !matches!(token.get_spelling().as_str(), "{" | ";"))
                .any(|token| token.get_spelling() == "noexcept")
        });

        let is_noexcept_method = has_noexcept_token
            && matches!(
                entity.get_exception_specification(),
                Some(ExceptionSpecification::BasicNoexcept)
            );

        let return_type = if matches!(kind, FunctionKind::Constructor | FunctionKind::Destructor) {
            None
        } else {
            parse_callable_return_type(entity)
        };

        let method = Method {
            name: id.name.clone(),
            return_type,
            visibility: parse_visibility(entity),
            parameters: parsed_parameters.parameters,
            template_parameters: parse_template_parameters(entity),
            modifiers: MethodModifier::from_conditions([
                (entity.is_static_method(), MethodModifier::Static),
                (entity.is_virtual_method(), MethodModifier::Virtual),
                (entity.is_pure_virtual_method(), MethodModifier::Abstract),
                (is_override_method, MethodModifier::Override),
                (is_noexcept_method, MethodModifier::Noexcept),
                (
                    kind == FunctionKind::Constructor,
                    MethodModifier::Constructor,
                ),
                (kind == FunctionKind::Destructor, MethodModifier::Destructor),
                (is_final_method, MethodModifier::Final),
            ]),
            source_location: parse_source_location(entity),
        };

        Some(ExtractedMethodDeclaration {
            class_id,
            method,
            method_type,
            signature_key: CallableSignatureKey {
                name: id.name.clone(),
                parameters: parsed_parameters.parameter_keys,
            },
        })
    }

    fn extract_free_function_declaration(
        seen_free_function_declarations: &mut HashSet<CallableDeclarationKey>,
        entity: &Entity,
        id: &FunctionId,
    ) -> Option<ExtractedFreeFunctionDeclaration> {
        let key = Self::extract_source_entity_key(entity)?;
        let parsed_parameters = parse_callable_parameters(entity);
        if !seen_free_function_declarations.insert(CallableDeclarationKey {
            owner: Self::free_function_owner_key(entity, &key),
            signature: CallableSignatureKey {
                name: id.name.clone(),
                parameters: parsed_parameters.parameter_keys,
            },
        }) {
            return None;
        }

        Some(ExtractedFreeFunctionDeclaration {
            key,
            declaration: FreeFunctionDecl {
                name: id.name.clone(),
                enclosing_namespace_id: namespace_id(entity),
                return_type: parse_callable_return_type(entity),
                parameters: parsed_parameters.parameters,
                template_parameters: parse_template_parameters(entity),
                source_location: parse_source_location(entity),
            },
        })
    }

    fn extract_function_def(
        entity: Entity,
        id: FunctionId,
        kind: FunctionKind,
        source_files: &mut SourceFileCache,
        seen_function_definitions: &mut HashSet<SourceEntityKey>,
    ) -> Option<ExtractedFunction> {
        let key = Self::extract_source_entity_key(&entity)?;

        if seen_function_definitions.contains(&key) {
            log::debug!(
                "skipping callable '{}': definition already extracted at {:?}",
                entity.get_name().unwrap_or_default(),
                key
            );
            return None;
        }

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

        let extracted_function = ExtractedFunction {
            key,
            definition: FunctionDef {
                id,
                kind,
                return_type,
                body,
            },
        };
        seen_function_definitions.insert(extracted_function.key.clone());

        Some(extracted_function)
    }

    fn free_function_owner_key(entity: &Entity, key: &SourceEntityKey) -> CallableOwnerKey {
        CallableOwnerKey::FreeFunction {
            enclosing_namespace_id: namespace_id(entity),
            linkage_scope: if has_translation_unit_local_linkage(entity) {
                CallableLinkageScope::TranslationUnitLocal {
                    source_file: key.source_file.clone(),
                }
            } else {
                CallableLinkageScope::External
            },
        }
    }

    // ── AST navigation helpers ────────────────────────────────────────────────

    fn extract_callable(entity: &Entity) -> Option<(FunctionId, FunctionKind)> {
        let function_id = Self::extract_function_id(entity)?;
        let function_kind = Self::extract_function_kind(entity, &function_id.scope)?;
        Some((function_id, function_kind))
    }

    fn extract_function_id(entity: &Entity) -> Option<FunctionId> {
        Some(FunctionId {
            scope: callable_scope(entity)?,
            name: entity.get_name()?,
        })
    }

    fn extract_function_kind(entity: &Entity, scope: &Scope) -> Option<FunctionKind> {
        match entity.get_kind() {
            EntityKind::FunctionDecl => Some(FunctionKind::Free),
            EntityKind::FunctionTemplate => match scope {
                Scope::Type { .. } => Some(Self::method_function_kind(entity)),
                Scope::Global | Scope::Namespace(_) => Some(FunctionKind::Free),
            },
            EntityKind::Method => Some(Self::method_function_kind(entity)),
            EntityKind::Constructor => Some(FunctionKind::Constructor),
            EntityKind::Destructor => Some(FunctionKind::Destructor),
            EntityKind::ConversionFunction => Some(FunctionKind::Conversion),
            _ => None,
        }
    }

    fn method_function_kind(entity: &Entity) -> FunctionKind {
        if entity.is_static_method() {
            FunctionKind::StaticMethod
        } else {
            FunctionKind::Method
        }
    }

    fn extract_source_entity_key(entity: &Entity) -> Option<SourceEntityKey> {
        let location = entity.get_location()?.get_file_location();
        Some(SourceEntityKey {
            source_file: normalize_source_identity_path(&location.file?.get_path()),
            source_offset: location.offset,
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

        if source_filter::is_excluded_entity(&resolved) {
            return None;
        }

        Self::extract_callable(&resolved).map(|(function_id, _)| function_id)
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

    /// Extracts an `IfStmt` as an ordered branch when its child layout is
    /// supported, or falls back to unstructured child traversal otherwise.
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

    /// Extracts a condition as a tree that models `&&`, `||`, and `!`
    /// structure explicitly. Other expressions remain source-backed leaves.
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
