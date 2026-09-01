<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# libclang C++ source analysis

This document is the entry point for the parser's analysis of a C++ source
file. The parser creates one libclang translation unit for each input file and
walks its AST recursively. It extracts a class-diagram model and a callable
control-flow model from eligible entities.

Detailed extraction contracts are documented separately:

- [Function extraction and control flow](function-extraction.md)
- [Class and enum extraction](type-extraction.md)

## Translation units and the main file

In libclang terminology, the *main file* is the input file of one parse
operation; it is not necessarily the C++ program's `main.cpp`. For example,
when parsing `src/service.cpp`, declarations and definitions in that file are
in the main file, while entities brought in through `#include` are not.

Function extraction keeps only callable definitions from the main file. This
avoids extracting an inline function from a shared header once for every
translation unit that includes it. A header-only function is extracted when
that header is itself an input file.

## Analysis flow

`Visitor` recursively walks the translation-unit AST. For each entity that is
not filtered out, it dispatches to the relevant specialized visitor:

| Entity kind | Analysis |
| --- | --- |
| `ClassDecl`, `StructDecl`, `ClassTemplate`, and `ClassTemplatePartialSpecialization` | Extract class/struct entities, members, aliases, bases, and relationship inputs. |
| `EnumDecl` | Extract enum entities and literals. |
| `FunctionDecl` and `Method` | Extract callable definitions and their body control flow. |

After traversal, class relationship resolution uses the collected base,
variable, and method type information to populate the class-diagram
relationships.

## Source filtering

Before dispatch, the parser omits entities located in system or external paths,
or entities in excluded namespaces such as `std`. Namespace cursors are
traversal containers; visitors derive namespace and type ownership from each
entity's semantic-parent chain rather than retaining mutable namespace state.

This filtering is distinct from the main-file rule: source filtering controls
whether an entity belongs in the parsed model at all, while the main-file rule
prevents duplicated callable definitions from project headers.
