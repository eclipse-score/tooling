<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Component Internal API Specification

## Purpose

This validator enforces consistency between two diagram types:

- **Component diagrams**
- **Internal API diagrams**

It shall make sure that every interface declared by the component design is
also declared by the internal API design.

## What is Validated

All comparisons are case-sensitive.

### Interface Declaration Consistency

Every interface declared in the component diagram must resolve to an interface
declared in the internal API diagram.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentInternalApiInterfaceDeclarationConsistency`)*

```text
' component diagram
component "Unit 1" as unit_1 <<unit>>
interface "IData" as IData
unit_1 -( IData

' internal_api diagram
interface "IData" as IData <<interface>> {
  {abstract} GetData(): Data*
}
```

The component interface is matched against the internal API interface ID. The
match is exact and case-sensitive. This check applies even when a component
interface is not referenced by a unit relation.

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Missing internal API interface | Interface Declaration Consistency |

## Debug Output

The validator emits debug output containing:

- component interfaces checked against the internal API
- internal API interfaces available for component interfaces
