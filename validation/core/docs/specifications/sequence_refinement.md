<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Sequence Refinement Specification

## Purpose

A call into a SEooC, component or unit can be refined by a sequence diagram
that opens it ("zoom"). This validator checks that the refining diagram keeps
the boundary of the call it refines. It runs in the `dependable_element`
profile on the sequence and component diagrams of all its architectural
designs, so an overview and its detail may live in different designs. An
architectural design alone is not checked for zoom.

Sequence diagrams are not merged. A refinement is a link between two calls,
found by participant uid and the static architecture. No marker is needed in
the diagram.

## Terms

- **Entry**: a call `X -> C : op()` (`->` or `->>`) where `C` is a SEooC,
  component or unit of the component diagram and `X` is neither `C` nor inside
  `C`. `X` may be an actor, an `<<external>>` participant, a gate or another
  element.
- **Window** of an entry: the calls after it in the same diagram, through
  `alt`, `opt`, `loop`, `par` and `break`, up to the reply from `C` (or from
  an element inside `C`) to `X`, or the `deactivate` of `C`. A synchronous
  call into `C` inside an open window is a nested entry that needs its own
  reply; a reply
  from a nested call to its own caller, or the `deactivate` of a nested
  activation, does not end the window. A `deactivate` of `C` ends the window
  even if `C` was already active before the entry. A reply or `deactivate`
  ends the window only if it applies to every path through the groups
  enclosing it:
  - `alt` with `else`: in every case.
  - `par`: in at least one branch.
  - `opt`, `alt` without `else`, `loop`, `break`: never, as the group may be
    skipped. `alt` without `else` is read as `opt`.
  An entry inside a case of an `alt` ignores the other cases.
  When the cases of an `alt` end with different open calls, the window keeps
  the longest list of open calls (the first one on a tie); a `par` keeps the
  shortest.
- **Exit** of an entry: a call in its window from `C` to an architecture
  element outside `C`, to an actor or `<<external>>` participant, or to a
  gate. Replies, self calls and calls after the window are no exits.
- **Detail** of an entry `X -> C : op()`: an entry `X -> u : op()` in any
  sequence diagram of the dependable element where `u` is a **direct child** of `C`, with the same sender uid and
  the same method name, and which is the **first call** of its diagram. Argument
  text is ignored. An entry can have no, one or several details (variants). A
  call that does not lead its diagram is no detail and is never linked, so a
  diagram that starts with another call stays unlinked and unchecked. A call
  is never refined by its own diagram: a detail in the diagram of the entry
  is dropped. A diagram given more than once (same source and name) counts
  once.

Participants are matched by uid, never by alias. A participant that is not
exactly one element of the component diagram is skipped. Such a participant is
not checked against the boundary of `C` and its calls are no exits, so a
misspelled uid in a detail is not found by this validation.

## What is Validated

A call without a detail stays a black box and is not checked.

For every detail of an entry into `C`:

- Each participant of the detail window is inside `C`, is the sender `X`, is
  an actor, `<<external>>` participant or gate, is an exit partner of the
  entry (or inside one), or is called by such a partner (transitively, within
  the detail window). An exit partner that is an ancestor of `C` matches
  itself only, not the other elements inside it.
- Every exit of the entry has a counterpart in at least one detail: a call
  with the same method from an element inside `C` to the same partner (or to an
  element inside it). The details of one entry may share the exits.

A detail may add exits to actors and `<<external>>` participants that the
entry does not show, and calls between the children of `C`.

The detail does not show `C` itself; it shows the children of `C`.

Every refined synchronous entry and every synchronous detail needs a closed
window: a reply to the sender (`-->` or `return`) or a `deactivate`. A `return`
replies only after a call with `++` or an `activate`; `autoactivate` is not
evaluated. A reply written as `--> X --` (reply and deactivate in one message)
ends one activation only. An asynchronous call (`->>`) needs no reply: its
window ends at a reply or `deactivate` if there is one, else at the end of the
diagram, and the exits of that window are checked like those of any other. An
asynchronous call into `C` inside a window needs no reply either.

*(Requirement: {requirement:downstream-ref}`Tools.SequenceRefinement`)*

Because each link goes from `C` to a direct child, a chain of refinements
always descends the architecture tree.

The same links make the calls clickable in the documentation: each entry links
to its details (`diagram_links.json`). Only details that pass this check are
linked: a detail that is rejected by a finding, or whose entry has a finding
(unclosed window, missing exit), gets no link. The links are written even when
the design has findings (`validation_cli --links-only` writes them without
validating or reporting).

### Example

```text
' overview: 1 call, 1 exit
actor App
participant "sl" as sl
participant Platform <<external>>
App -> sl : Get()
sl -> Platform : Log()
sl --> App : value
```

```text
' detail: opens sl at its child component
actor App
participant "sl::comp" as comp
participant Platform <<external>>
App -> comp : Get()
comp -> Platform : Log()
comp --> App : value
```

## Failure Cases

| Failure case | Finding |
|---|---|
| A detail uses an architecture element that is outside `C`, not the sender, no exit partner and not called by one | `ZoomParticipantOutsideContainer` |
| No detail keeps an exit of the entry | `ZoomExitMissing` |
| A detail shows the refined element `C` itself | `ZoomContainerInDetail` |
| A synchronous refined entry or detail is neither replied to nor deactivated | `ZoomWindowNotClosed` |
