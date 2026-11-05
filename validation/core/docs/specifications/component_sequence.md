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
uid in the sequence diagrams, and every participant uid must reference a unit,
component or SEooC id.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceIdentifierConsistency`)*

A participant uid is matched against unit ids as follows:

- A qualified uid (containing `.` or `::`) must equal a unit id exactly.
- A single-segment uid is a leaf reference: it matches the unit whose id ends
  in that segment. It must match exactly one unit; several matches are
  reported as an ambiguous participant. Write the qualified path as the
  participant label to disambiguate, for example
  `participant "component_a.unit_1" as unit_1`.
- A uid that matches no unit is matched against component and SEooC ids by
  the same rules, including the ambiguity report.
- A uid that matches both a unit and a component or SEooC is reported as
  ambiguous.

A participant uid is the participant's name as an identifier path; an `as`
alias is a local key for messages and does not take part in the check
(`plantuml/parser/docs/element-identifiers.md`, Rule A′).
Environment participants (`actor`, or a participant with the `<<external>>`
stereotype, in any letter case) are exempt from Identifier Consistency. They
shall not use the uid of a unit, component or SEooC of the component diagram;
such a declaration is reported as a naming error, because it would hide that
element from the checks. A participant that matches a component or SEooC id is
a black-box lifeline: it is accepted, but it does not count as a reference to
the units inside it. See `sequence_design.md`.

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
diagrams, and every function call between two elements in a sequence diagram
must correspond to a shared interface in the component diagram. A call is
between two elements when the caller and callee are different units, or when
one end is a component or SEooC lifeline. Self-calls, calls whose ends contain
each other, and any call involving an environment participant or a gate are
excluded from this check.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentSequenceInterfaceConnectionConsistency`)*

A component or SEooC lifeline is a black box: its interfaces are the union of
the interfaces of the units it contains. A call with such a lifeline needs a
shared interface with its partner, but it does not exercise the interface
connection between two units inside the component. Only a call between exactly
those two units does.

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
unit_2 - IData
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
| Sequence participant matches multiple components ambiguously | Identifier Consistency |
| Actor or `<<external>>` participant uses the uid of a unit, component or SEooC | Identifier Consistency |
| Missing sequence interaction for interface-connected units | Interface-Connection Consistency |
| Missing interface connection for sequence-connected units or elements | Interface-Connection Consistency |

## Debug Output

The validator emits debug output containing:

- unit ids derived from the component diagram
- observed participants
- observed sequence calls (`caller -> callee : method`)
- unit interface targets derived from the component diagram
- interface-connected unit pairs derived from the component diagram
