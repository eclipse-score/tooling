<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

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
| Sequence message targets a class that does not declare the called operation | Message-Operation Consistency |
| Sequence self-call targets a class that does not declare the called operation | Message-Operation Consistency |

## Debug Output

The validator should emit debug output containing:

- resolved design classes
- observed sequence participants
- participant-to-class mapping decisions
- observed sequence messages (`caller -> callee : method`)
- matched and unmatched class operations
