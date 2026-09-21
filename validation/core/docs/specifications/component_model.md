<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Component Model Specification

## Purpose

`architectural_design.static` may list more than one PlantUML file (e.g. a
boundary overview diagram plus one or more detail diagrams). This validator
builds the merged `ComponentDiagramArchitecture` that every other
component-diagram validator (`bazel_component`, `component_internal_api`,
`component_public_api`, `component_sequence`, `sequence_internal_api`)
operates on, and enforces that the merge itself is unambiguous.

## What is Validated

Entities are matched across files by id — the full dot-path of parent
aliases — not by bare alias, so two entities that share an alias under
different parents are never conflated.

### Cross-File Declaration Consistency

Re-declaring the same id in more than one `static` file is allowed as long as
every declaration agrees on stereotype and element type; their relations are
merged (duplicate relations, compared structurally rather than by source
location, are not repeated). This is what lets an overview file bare-declare
an entity that a detail file elaborates further.

*(Requirement: {requirement:downstream-ref}`Tools.ComponentModelCrossFileDeclarationConsistency`)*

```text
[Design] Unit "shared_thing" is re-declared with a conflicting stereotype in another component diagram file.
```

```text
[Design] Component "shared_thing" is re-declared with a conflicting element type in another component diagram file.
```

Declarations of the same entity are compared by name/alias/id only through
the id itself — since two declarations sharing the exact same id are
guaranteed to share the same alias and immediate parent, disagreement is only
possible on stereotype or element type. Re-nesting an entity under a
*different* parent in another file is not detected here: a different parent
means a different id, hence a different entity. That mistake instead surfaces
as an extra/missing entity in the `bazel_component` check.

### Single-Home Decomposition

A parent whose children (nested components/units) are declared across more
than one file is only allowed if a single file contains the full set of
children declared for that parent anywhere — i.e. one file is the "home" file
and the others only re-declare a benign subset (or none). If the children are
genuinely split, with no file containing all of them, this is an error.

*(Requirement: {requirement:downstream-ref}`Tools.ComponentModelSingleHomeDecomposition`)*

```text
[Design] Entity "component_a" has children declared across more than one component diagram file, with no single file containing all of them.
```

Interfaces are exempt from this check: they aren't compared against the
Bazel build graph, so an overview file may declare a subset of a parent's
interfaces (e.g. its public ones) while a detail file declares others (e.g.
its internal ones), without that counting as a split decomposition.

### Determinism

Errors are ordered by source location (file, then line), not by which
declaration happened to be visited first while merging — so the reported
error is identical regardless of the order files are listed in `static`.

### Not Validated Here

Each `.puml` file is parsed and resolved independently, before the id-based
merge above ever runs. A relation may only reference an alias declared in
that same file: referencing an alias that's only declared in another
`static` file fails at PlantUML parse time (`Element Resolver:
UnresolvedReference: <alias>`), not as a Design validation error from this
validator.

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Same id declared in two files with conflicting stereotype or element type | Cross-File Declaration Consistency |
| A parent's children declared across files with no single file containing all of them | Single-Home Decomposition |

## Debug Output

The validator emits debug output containing the total number of SEooC
packages, components, and units after the cross-file merge.
