<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Component Sequence Specification

## Purpose

This validator enforces consistency across entities in two diagram types:

- **Component diagrams**
- **Sequence diagrams**

It shall make sure that Architectural Elements are consistently named and related to each other.

## What is Validated

All comparisons are case-sensitive.

### Alias Consistency

Unit aliases from the component diagram must exactly match the set of
participant aliases used across all sequence diagrams.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceAliasConsistency`)*

```text
' component diagram
component "Unit 1" as unit_1 <<unit>>
component "Unit 2" as unit_2 <<unit>>

' sequence diagram
participant "Unit 1" as unit_1
participant "Unit 2" as unit_2
```

### Interface-Connection Consistency

Every pair of units connected through an interface in the component diagram
must have at least one corresponding function-call interaction in the sequence
diagrams, and every cross-unit function call in a sequence diagram must
correspond to an interface connection in the component diagram. Self-calls are
excluded from this check.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceInterfaceConnectionConsistency`)*

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
```

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Missing sequence participant | Alias Consistency |
| Unexpected sequence participant | Alias Consistency |
| Missing sequence interaction for interface-connected units | Interface-Connection Consistency |
| Missing interface connection for sequence-connected units | Interface-Connection Consistency |

## Debug Output

The validator emits debug output containing:

- expected unit aliases
- observed participants
- observed sequence calls (`caller -> callee : method`)
- unit interface targets derived from the component diagram
- interface-connected unit pairs derived from the component diagram
