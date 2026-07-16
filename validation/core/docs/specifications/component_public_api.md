<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Component Public API Specification

## Purpose

This validator enforces consistency between public API interfaces in the
static design (component diagram) and interfaces declared by the public API
design (class diagram).

It shall make sure that every public API interface declared by the static design
is also declared by the public API and is connected to the SEooC boundary.

## What is Validated

The validator compares static design diagrams with public API design diagrams.
It receives two indexed diagram inputs:

| Input | Source | Meaning |
|---|---|---|
| `component_diagrams` | static design component diagram FlatBuffers | Public API interface references from the static design |
| `public_api_diagrams` | public API class diagram FlatBuffers | Public API interfaces declared by the class design |

For this validator, a public API interface is a top-level interface declared in
the static design. Interfaces declared inside the SEooC, components, or units
are treated as internal API interfaces.

### Interface Declaration Consistency

Every public API interface declared in the static design diagram must resolve
to an interface declared in the public API class diagram.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentPublicApiInterfaceDeclarationConsistency`)*

The component public API interface is matched against public API interface
entries derived from the public API diagram. Matching is exact and
case-sensitive.

```text
' static design diagram
package "Sample SEooC" as sample_seooc <<SEooC>> {
}

interface "Sample Library API" as SampleLibraryAPI
sample_seooc )- SampleLibraryAPI

' public API diagram
interface "Sample Library API" as SampleLibraryAPI <<interface>> {
  +GetNumber(): int
}
```

### SEooC Relationship Consistency

Every public API interface declared in the static design diagram must be
connected from the SEooC in the static design diagram.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentPublicApiSeoocRelationshipConsistency`)*

This item covers the SEooC boundary relation only. The interface declaration in
the public API diagram is covered by Interface Declaration Consistency.

```text
' static design diagram
package "Sample SEooC" as sample_seooc <<SEooC>> {
}

interface "Sample Library API" as SampleLibraryAPI
sample_seooc )- SampleLibraryAPI
```

The validator collects relationships from SEooC entities and checks whether the
public API interface is a relation target. Relations from components or units do
not satisfy this rule.

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Missing public API interface declaration | Interface Declaration Consistency |
| Public API interface not connected from the SEooC | SEooC Relationship Consistency |

## Debug Output

The validator emits debug output containing:

- public API interfaces declared in the static design
- public API interfaces referenced by SEooC relations
- public API identifiers available from the public API diagram

Failure messages list the public API interface IDs that are missing or not
connected from the SEooC.
