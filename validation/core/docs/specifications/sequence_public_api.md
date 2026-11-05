<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Sequence Public API Specification

## Purpose

This validator enforces consistency between sequence diagrams and the public
API of the SEooC:

- **Sequence diagrams**
- **Public API diagrams**

It runs whenever sequence and public API diagrams are provided. It needs
neither component nor internal API diagrams.

## What is Validated

All comparisons are case-sensitive.

Only operation calls (synchronous `->` and asynchronous `->>` messages) between
an `actor` and the SEooC are validated. Replies, create and delete messages,
calls to or from `<<external>>` participants and gate messages are not checked.
Method names are extracted as described in `sequence_internal_api.md`.
Repeated identical calls are reported once.

### Public API Boundary

A call from an actor to the SEooC shall use a method declared on a public API
interface the called element provides. A callback, that is a call (synchronous
or asynchronous) from an element of the SEooC to an actor, shall be labelled
with the name of a public API interface the calling element requires, or one of
its methods.
*(Requirement: {requirement:downstream-ref}`Tools.SequencePublicApiBoundary`)*

The scope of an element comes from the component diagram: it provides a public
interface when bound with `)-` and requires it when bound with `-(`. A
component or SEooC lifeline has the interfaces of the units inside it. When no
component diagram is provided, or the element binds no public interface, the
call is checked against the whole public API by name. A scoped error names the
interfaces that were searched.

```text
' sequence diagram
actor User
participant unit_1
User -> unit_1 : Start()
unit_1 ->> User : OnFinished()
```

```text
' public API diagram
interface IController <<interface>> {
  {abstract} Start(): void
}
interface IListener <<interface>> {
  {abstract} OnFinished(): void
}
```

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Actor call to a method not declared in the public API | Public API Boundary |
| Callback to an actor labelled with neither a public API interface nor method | Public API Boundary |
