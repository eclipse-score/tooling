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
that opens it ("zoom") or shows additional interactions at the same level.
This validator checks the call windows and, for zoom, the boundary. It runs in
the `dependable_element` profile on the sequence and component diagrams of all
its architectural designs, so an overview and its detail may live in different
designs. An architectural design alone is not checked for refinement.

Sequence diagrams are not merged. A refinement is a link between two calls,
found by participant uid and the static architecture. Asynchronous same-level
windows require explicit end markers; other matching needs no marker.

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
- **Zoom detail** of an entry `X -> C : op()`: an entry `X -> u : op()`
  where `u` is a **direct child** of `C`, with the same sender and method.
- **Same-level detail**: an entry `X -> C : op()` with the same sender
  (a resolved element or environment), receiver, non-empty method and
  synchronous or asynchronous call kind. The overview window has no calls
  from `C` or its descendants; the detail window has at least one such call.
  Both synchronous windows must be closed; both asynchronous windows must
  have an explicit end marker. An unresolved sender cannot form a same-level
  refinement.

Each detail is the **first call** of another sequence diagram of the dependable
element. Argument text is ignored. An entry can have no, one or several
details (variants). A call that does not lead its diagram is no detail. A
call is never refined by its own diagram. A diagram given more than once
(same source and name) counts once.

Participants are matched by uid, never by alias. A participant that is not
exactly one element of the component diagram is skipped. Such a participant is
not checked against the boundary of `C` and its calls are no exits, so a
misspelled uid in a detail is not found by this validation.

## What is Validated

A call without a detail stays a black box and is not checked.

For every zoom detail of an entry into `C`:

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

A zoom detail does not show `C` itself; it shows the children of `C`.

A same-level detail keeps `C` and may call other architecture elements. Its
interactions remain subject to the normal sequence and API validators when
their inputs are provided. Same-level matching checks the window shape, not
semantic equivalence: arguments and reply labels are not compared. Unrelated
scenarios with matching calls can therefore link. Two windows with no calls
from `C` or its descendants, or two with such calls, do not form a same-level
refinement. Unclosed synchronous windows are not linked and do not produce a
same-level finding. Findings of zoom details of the same entry (such as
`ZoomExitMissing`) do not remove a same-level link.

Every synchronous zoom entry and detail needs a closed window: a reply to the
sender (`-->` or `return`) or a `deactivate`. A `return`
replies only after a call with `++` or an `activate`; `autoactivate` is not
evaluated. A reply written as `--> X --` (reply and deactivate in one message)
ends one activation only. An asynchronous call (`->>`) needs no reply: its
window ends at a reply or `deactivate` if there is one, else at the end of the
diagram, and the exits of that window are checked like those of any other. An
asynchronous call into `C` inside a window needs no reply either.

For same-level matching only, an asynchronous window ends at
`ref over C : score-window-end`, where `C` is the receiver's local reference.
The marker is required on every path in both diagrams; replies and
deactivations do not replace it. The marker appears as a reference box in the
rendered diagram. A marker must name exactly one receiver and match an open
asynchronous entry. A second entry into that receiver before the first window
ends is ambiguous and reported; neither overlapping asynchronous entry is
matched at the same level. A call into that receiver from an element that the
window itself called (a callback) is no second entry. Overlap detection is
conservative across branches:
entries appearing between an entry and its final end marker are treated as
overlapping even if they are in mutually exclusive alternatives. These markers
do not change zoom windows.

*(Requirements: {requirement:downstream-ref}`Tools.SequenceRefinement`,
{requirement:downstream-ref}`Tools.SequenceSameLevelRefinement`,
{requirement:downstream-ref}`Tools.SequenceSameLevelWindowClosure`,
{requirement:downstream-ref}`Tools.SequenceSameLevelAsyncBoundary`)*

Zoom descends the architecture tree. Same-level links go only from a window
without calls from `C` to one with such calls, so they cannot create a
refinement cycle.

The same links make the calls clickable in the documentation: each entry links
to its details (`diagram_links.json`). Only details that pass this check are
linked: a detail that is rejected by a finding, or whose entry has a finding
(unclosed window, missing exit), gets no link. The links are written even when
the design has findings (`validation_cli --links-only` writes them without
validating or reporting).

### Zoom example

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

### Same-level example

```text
' overview: the call is a black box
actor App
participant "sl::comp" as Service
App -> Service ++ : Get()
return value
```

```text
' detail: the same call shows a collaboration
actor App
participant "sl::comp" as Handler
participant "sl::other::unit_2" as Helper
App -> Handler ++ : Get()
Handler -> Helper ++ : Fetch()
return data
return value
```

For an asynchronous call, bound the window in each diagram:

```text
App ->> Service : Get()
ref over Service : score-window-end
```

```text
App ->> Handler : Get()
Handler ->> Helper : Fetch()
ref over Handler : score-window-end
```

## Failure Cases

| Failure case | Finding |
|---|---|
| A zoom detail uses an architecture element that is outside `C`, not the sender, no exit partner and not called by one | `ZoomParticipantOutsideContainer` |
| No zoom detail keeps an exit of the entry | `ZoomExitMissing` |
| A zoom detail shows the refined element `C` itself | `ZoomContainerInDetail` |
| A synchronous zoom entry or detail is neither replied to nor deactivated | `ZoomWindowNotClosed` |
| An asynchronous entry overlaps another entry to the same receiver before its end marker | `SameLevelAsyncWindowOverlap` |
| An end marker names multiple receivers or does not match an open asynchronous entry | `SameLevelAsyncWindowEndInvalid` |

`ZoomWindowNotClosed` retains its existing name for compatibility.
