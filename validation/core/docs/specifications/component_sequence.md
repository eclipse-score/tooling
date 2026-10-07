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

### Identifier Consistency

Every unit id from the component diagram must be referenced by a participant
uid in the sequence diagrams, and every participant uid must reference a unit
id.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceIdentifierConsistency`)*

A participant uid is matched against unit ids as follows:

- A qualified uid (containing `.` or `::`) must equal a unit id exactly.
- A single-segment uid is a leaf reference: it matches the unit whose id ends
  in that segment. It must match exactly one unit; several matches are
  reported as an ambiguous participant. Write the qualified path as the
  participant label to disambiguate, for example
  `participant "component_a.unit_1" as unit_1`.

A participant uid is the participant's name as an identifier path; an `as`
alias is a local key for messages and does not take part in the check
(`plantuml/parser/docs/element-identifiers.md`, Rule A′).
The special participant name `ExternalEndpoint` represents an external
caller/callee outside the modeled units; it is exempt from Identifier
Consistency and may appear in sequence diagrams without a matching
component-diagram unit.

```text
' component diagram
component unit_1 <<unit>>
component unit_2 <<unit>>
```

```text
' sequence diagram
participant unit_1
participant unit_2
```

### Interface-Connection Consistency

Every pair of units connected through an interface in the component diagram
must have at least one corresponding function-call interaction in the sequence
diagrams, and every cross-unit function call in a sequence diagram must
correspond to an interface connection in the component diagram. A cross-unit
call is one where the caller and callee are different units; self-calls
(caller and callee are the same unit) and any call involving
`ExternalEndpoint` are excluded from this check.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceInterfaceConnectionConsistency`)*

Two units are considered interface-connected if either one requires an
interface that the other provides (the require/provide match works in both
directions). The check itself is undirected and per-pair: a call from unit_1
to unit_2 satisfies the same requirement as a call from unit_2 to unit_1, and
once one call has been found for a pair, additional calls between the same two
units are not required and do not produce additional errors.

```text
' component diagram
component unit_1 <<unit>>
component unit_2 <<unit>>
interface IData
unit_1 -( IData
unit_2 )- IData
```

```text
' sequence diagram
participant unit_1
participant unit_2
unit_1 -> unit_2 : GetData()
```

Interface IDs used in error messages may be package-qualified (e.g.
`package_a.IData`) when the interface is declared inside a package or
component in the component diagram.

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Missing sequence participant | Identifier Consistency |
| Unexpected sequence participant | Identifier Consistency |
| Sequence participant matches multiple units ambiguously | Identifier Consistency |
| Missing sequence interaction for interface-connected units | Interface-Connection Consistency |
| Missing interface connection for sequence-connected units | Interface-Connection Consistency |

## Debug Output

The validator emits debug output containing:

- unit ids derived from the component diagram
- observed participants
- observed sequence calls (`caller -> callee : method`)
- unit interface targets derived from the component diagram
- interface-connected unit pairs derived from the component diagram
