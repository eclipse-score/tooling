<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Sequence Internal API Specification

## Purpose

This validator enforces consistency between sequence diagrams and Internal API
diagrams:

- **Sequence diagrams**
- **Internal API diagrams**

It checks Internal API method coverage with sequence plus Internal API inputs.
When a **Component diagram** is also provided, the validator uses it as optional
context to check sequence method names against the related shared interfaces of
the participating units.

## What is Validated

All comparisons are case-sensitive.

Method-name consistency is checked only when component context is available.
Without component context, the validator does not run a weak global method-name
existence check.

### Related Interface Method-Name Consistency With Component Context

When component context is available, every function used in a sequence
interaction must be declared in the related Internal API interface context.

For cross-unit calls, the method must be declared on a shared interface of the
participating units as defined in the component diagram. For self-calls, the
method must be declared on one of the available component or Internal API
interfaces.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceMethodNameConsistency`)*

```text
' component diagram
component "Unit 1" as unit_1 <<unit>>
component "Unit 2" as unit_2 <<unit>>
interface "IData" as IData
unit_1 -( IData
unit_2 )- IData

' sequence diagram
participant "Unit 1" as unit_1
participant "Unit 2" as unit_2
unit_1 -> unit_2 : GetData()

' internal_api diagram
interface "IData" as IData <<interface>> {
  {abstract} GetData(): Data*
}
```

### Interface Coverage

Every function declared in an Internal API interface must be called in at least
one sequence interaction. Self-calls count as valid usage.
*(Requirement: {requirement:downstream-ref}`Tools.SequenceInternalApiInterfaceCoverage`)*
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceInterfaceCoverage`)*

```text
' internal_api diagram
interface "IData" as IData <<interface>> {
  {abstract} GetData(): Data*
  {abstract} SetData(d: Data*): void
}

' sequence diagram
participant "Unit 1" as unit_1
participant "Unit 2" as unit_2
unit_1 -> unit_2 : GetData()
unit_1 -> unit_2 : SetData(d)
```

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Method not declared in related interface | Related Interface Method-Name Consistency With Component Context |
| Internal API interface function not exercised | Interface Coverage |

## Debug Output

The validator emits debug output containing:

- observed sequence calls (`caller -> callee : method`)
- unit interface targets derived from the component diagram, when component
  context is available
- Internal API interfaces available for sequence validation
