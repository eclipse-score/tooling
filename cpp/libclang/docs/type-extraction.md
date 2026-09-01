<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# libclang class and enum extraction

This document records the current contract for extracting C++ classes, structs,
and enums. It describes the implementation choices in `ClassVisitor` and
`EnumVisitor`, rather than the complete libclang API. For the overall
per-source-file analysis flow, see [libclang C++ source analysis](ast-traversal.md).

## Traversal and source filtering

`Visitor` recursively walks the translation-unit AST and dispatches matching
cursors to specialized visitors:

- `ClassVisitor` handles classes, structs, class templates, and partial class
  template specializations;
- `EnumVisitor` handles enums;
- `FunctionVisitor` handles function definitions and control flow, documented
  in [function-extraction.md](function-extraction.md).

Namespace nodes are traversal containers. Visitors derive namespaces and type
owners from each entity's semantic-parent chain instead of maintaining mutable
namespace state. Before dispatch, entities in system or external paths and
entities in excluded namespaces such as `std` are omitted.

`Entity::visit_children` is used for direct-child inspection where a visitor
needs control over recursion. The top-level visitor recursively traverses the
translation unit, so nested classes and nested enums are discovered separately.

## Class, struct, interface, and abstract-class extraction

Named `ClassDecl`, `StructDecl`, `ClassTemplate`, and
`ClassTemplatePartialSpecialization` cursors are represented as class-diagram
entities. Anonymous classes and structs are skipped because they have no stable
name. Their resulting entity type is inferred from the cursor kind and member
set as `Struct`, `Class`, `Interface`, or `AbstractClass`.

The classification rules are:

- `Struct`: the source cursor is `StructDecl`;
- `Class`: the cursor is not a struct and has no pure-virtual method;
- `Interface`: the class has at least one pure-virtual method, no data member,
  and no concrete non-constructor/destructor method;
- `AbstractClass`: the class has at least one pure-virtual method but does not
  meet the `Interface` conditions, because it has a data member or a concrete
  non-constructor/destructor method.

For each direct member cursor, `ClassVisitor` currently extracts:

- base specifiers and their resolved types;
- methods, constructors, destructors, and method templates;
- fields and variable declarations;
- `using` aliases and `typedef` declarations.

Method parameter types normally come from libclang's argument list. When that
list is unavailable for a cursor such as `FunctionTemplate`, the visitor falls
back to direct `ParmDecl` children. Template parameters are retained for class
templates, partial specializations, and method templates.

The visitor records intermediate type information for base classes, variables,
and methods. After the translation unit has been traversed, relationship
resolution derives the class-diagram relationships from that collected type
information.

## Enum extraction

Named `EnumDecl` cursors are represented as enum entities. Anonymous enums are
skipped because they have no stable name.

Each direct `EnumConstantDecl` becomes an enum literal with its name, numeric
value, and source location. The visitor reads the enum's underlying type to
choose the unsigned value supplied by libclang for unsigned enums; values are
stored as `i128` so every `u64` and `i64` value can be serialized safely.
