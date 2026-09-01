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

mod clang_adapter;
mod class_relationship_resolver;
mod class_visitor;
pub mod context;
mod enum_visitor;
mod function_visitor;
mod types;
pub mod visitor;

pub use cpp_semantics::{BodyItem, FunctionDef, ResolvedType};

pub use clang_adapter::source_filter::is_external_dependency_path;
pub use class_visitor::ClassVisitor;
pub use context::VisitContext;
pub use enum_visitor::EnumVisitor;
pub use function_visitor::FunctionVisitor;
pub use visitor::{AstVisitor, Visitor};
