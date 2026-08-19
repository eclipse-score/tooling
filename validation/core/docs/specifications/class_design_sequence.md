<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

---
orphan: true
---

# Class Design Sequence Specification

## Purpose

This validator enforces consistency between the class model in the unit design
and the interactions modeled in sequence diagrams.

It shall make sure that the runtime interactions declared in sequence diagrams
are supported by the designed classes and methods.

The validator is intentionally design-level. It validates that the sequence is
compatible with the class design contract; it does not try to prove full
runtime behavior or implementation control flow.

## What is Validated

All comparisons are case-sensitive unless otherwise stated by the sequence
parser or the class-model normalizer.

The validator compares two indexed design inputs:

| Input | Source | Meaning |
|---|---|---|
| `design_classes` | `unit_design` class diagram FlatBuffers | Required structural design model |
| `sequence_diagrams` | sequence diagram FlatBuffers | Required runtime interaction model |

### Participant-Class Consistency

Every sequence participant that represents a designed runtime element must map
to exactly one class in the design class model.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignSequenceParticipantClassConsistency`)*

This check validates that the sequence does not reference unknown or ambiguous
classes.

Participant resolution shall follow this order:

1. Match the participant reference itself against a class id.
2. If the participant has a different display name, match that display name
  against a class id.
3. If the display name still does not resolve, match the display name against a
  unique class short name.
4. If the display name uses one supported special form, derive additional class
  candidates from that form.
5. If none of the above resolves uniquely, fall back to matching the
  participant reference against a unique class short name.

The supported special display forms are:

- `:Name`, which contributes `Name` as a short-name candidate.
- `prefix:qualified::Type`, which contributes `qualified::Type` as an id
  candidate and both `qualified::Type` and `Type` as short-name candidates.

Only the first non-empty display line participates in class matching. If the
display name contains additional non-empty lines or escaped line fragments
after the primary line, they shall be ignored for matching and may be reported
through debug or warning output.

The following participant display forms are invalid and shall be rejected as
participant-class failures:

- a primary display line containing more than one standalone `:` separator
- a primary display line containing `:` without a non-empty right-hand side

```text
' class diagram
class Controller
class Repository

' sequence diagram
participant Controller
participant Repository
Controller -> Repository : FindById(id)
```

### Message-Operation Consistency

Every message sent to a participant must correspond to an operation on the
target class. The validator should match at least the operation name and may
optionally compare parameter arity and normalized parameter types when that
information is available in both diagrams.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignSequenceMessageOperationConsistency`)*

This includes both cross-class messages and self-calls. Operation lookup may
resolve either on the target class itself or on inherited operations available
through its base classes or interfaces. The sequence may only invoke behavior
that the class design actually declares or inherits.

Operation lookup shall follow these rules:

1. Check the target class itself for a method with the requested name.
2. If not found locally, traverse outgoing `Inheritance` and `Implementation`
  relations recursively.
3. Track visited class ids while traversing to avoid infinite recursion caused
  by cycles in the resolved relationship graph.
4. Treat inherited `private` methods as not accessible to the target class.
5. Treat inherited non-`private` methods as valid matches.

As a result, a sequence call is valid when the target class declares the method
itself or inherits an accessible method from a base class or implemented
interface. A method that exists only as a private inherited member shall not be
accepted as a valid target operation.

```text
' class diagram
class Repository {
  + FindById(id: Id) : Entity
}

' sequence diagram
participant Controller
participant Repository
Controller -> Repository : FindById(id)
```

```text
' class diagram
class Controller {
  + Execute()
  - Validate()
}

' sequence diagram
participant Controller
Controller -> Controller : Validate()
```

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Sequence participant has no matching design class | Participant-Class Consistency |
| Sequence participant matches multiple design classes ambiguously | Participant-Class Consistency |
| Sequence participant uses a disallowed special display form | Participant-Class Consistency |
| Sequence message targets a class that does not declare or accessibly inherit the called operation | Message-Operation Consistency |
| Sequence self-call targets a class that does not declare or accessibly inherit the called operation | Message-Operation Consistency |
| Sequence message targets a method that exists only as a private inherited operation | Message-Operation Consistency |

## Debug Output

The validator should emit debug output containing:

- resolved design classes
- observed sequence participants
- participant-to-class mapping decisions
- observed sequence messages (`caller -> callee : method`)
- matched and unmatched class operations
