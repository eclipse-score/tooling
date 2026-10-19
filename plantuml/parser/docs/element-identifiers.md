<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Element Identifiers — Authoring Guide

Every architecture element in a component, class, or sequence diagram gets a
**canonical identifier**. Two elements are considered *the same architecture
element* exactly when their identifiers are equal — that is how the validators
link a component to its detailed design, and a sequence participant to the unit
it represents.

> You never write an identifier yourself. The parser never invents one either —
> it is computed during resolution from the nesting and the written name.

The [cross-diagram test suite](../integration_test/cross_diagram/) pins the
behaviour described here with executable goldens.

---

## 0. Status

| Topic | State | Test case |
|-------|-------|-----------|
| The written name is the id leaf (component, class, interface, enum, struct, port, package, namespace, participant) | implemented | `component_nesting`, `class_name_wins`, `alias_is_local_key` |
| Name must be an identifier path; prose is an error | implemented | `prose_names`, `invalid_prose_label` cases |
| Alias is a local reference key; names of aliased elements are not references | implemented | `invalid_reference_by_label` cases |
| Reference resolution (`S.r`, else unique leaf; leading `.`/`::` roots) | implemented | `qualified_reference`, `relation_simple_name_prefers_direct_hit` |
| `ExternalEndpoint` matched on the name | implemented | `doc_6_external_endpoint` |
| Several files of one view resolve as one group (reparenting, peer references) | implemented | `group_*` cases |
| Root anchor (Bazel-package prefix) | not implemented; ids are unique per dependable element | — |

---

## 1. What an identifier is built from

| # | Input | Where it comes from | Example |
|---|-------|---------------------|---------|
| 1 | **Internal scope** | The `package` / `component` / `namespace` blocks you nest the element in | `logging.Recorder` |
| 2 | **Name** | The name written in the declaration | `Backend` |

They are joined with `.`:

```text
logging.Recorder  .  Backend
└ internal scope ┘   └ name ┘
```

Normalization applies everywhere:

- `::` and `.` are **equivalent** separators — `logging::Recorder` and
  `logging.Recorder` produce the identical identifier.
- Leading and trailing dots and empty segments are dropped.

### Separators and root markers

`.` and `::` spell the same name, and a leading `.` or `::` roots it
([Rule C](#rule-c)). `a.b.X`, `a::b::X` and `a.b::X` are one identifier;
`.a.X` and `::a::X` ignore the enclosing scope. The parser accepts every
spelling wherever a name is written: declarations, `package` / `namespace`
names, ports, `extends` / `implements` targets and relationship endpoints.
Malformed paths (`a..b`, `a::`, `a:::b`, a lone `::`) are errors.

The parser is deliberately more permissive than PlantUML in component
diagrams: `a::b::X` and `::a::X` parse but are not rendered by PlantUML there.
Write `.` if the diagram must also render; the identifier is the same.

Two spellings in component diagrams need care:

- `component .a.C` declares `a.C`. A lone `.` between two endpoints stays
  PlantUML's dotted arrow.
- In a relation, `A --> B::text` is the qualified endpoint `B::text`, while
  `A --> B:: text` and `A --> B ::text` are endpoint `B` with a description.
  Write the label separator as ` : ` with spaces.

---

## 2. The name is the identity

The name of an element is its identity. The quoted label of a declaration is
the name, not display text. It must be an identifier path: segments of letters,
digits and `_`, separated by `.` or `::` ([Rule A′](#rule-a)).

```text
component unit_1 <<unit>>                     ' id: unit_1
class "score::mw::com::Proxy" as Proxy        ' id: score.mw.com.Proxy, key: Proxy
component "Unit 1" as unit_1 <<unit>>         ' error: free-text name
```

The alias (`as X`) is a local reference key. It never takes part in the id.
Prose belongs in a note, a stereotype or a relation description, not in a
name. The rule covers every element kind of the component grammar (`node`,
`cloud`, `database`, `actor`, `usecase`, `rectangle`, `frame`, …), so
`node "ECU1 (Provider)"` is an error too.

### Component diagrams

```text
@startuml component_diagram
package logging {
    component Recorder <<component>> {
        component Backend <<unit>>
    }
}
@enduml
```

| Element | Identifier |
|---------|-----------|
| `logging` | `logging` |
| `Recorder` | `logging.Recorder` |
| `Backend` | `logging.Recorder.Backend` |

`<<SEooC>>` package, `<<component>>` and `<<unit>>` names match the Bazel
target names (case-insensitive).

### Class diagrams

`package` and `namespace` blocks both contribute scope.

```text
@startuml class_diagram
package logging {
    interface IBackend {
        + Write() : void
    }
}
@enduml
```

→ `logging.IBackend`

A qualified name in a nested declaration is appended to the enclosing scope,
like a relative name in C++. A leading `::` (or `.`) roots it:

```text
package outer {
    class core::Circle      ' → outer.core.Circle
    class ::core::Square    ' → core.Square
}
```

A class-like name drops one trailing template argument list:
`class "ProxyContainer<ProxySpec...>" as P` is `ProxyContainer`. Only the first
line of a label counts (literal `\n`), and label markup is stripped.

`X as "Label"` follows PlantUML: the quoted side is the name, the bare side the
alias.

### Sequence diagrams

A participant has no nesting, so the whole path is written into the name:

| What you write | Identifier | Reference key |
|----------------|-----------|---------------|
| `participant Backend` | `Backend` | `Backend` |
| `participant "logging::Recorder::Backend" as Backend` | `logging.Recorder.Backend` | `Backend` |
| `participant "logging.IBackend" as IBackend` | `logging.IBackend` | `IBackend` |
| `actor Client` | `Client` | `Client` |
| `participant "Log Client" as Client` | error: free-text name | — |

All participant kinds behave identically — `participant`, `actor`, `boundary`,
`control`, `entity`, `queue`, `database`, `collections`.

Messages, `activate`, `deactivate`, `destroy`, `create` and `ref over` name a
participant by its alias, or by its declared name when it has no alias. The
name of an aliased participant is not a reference: after
`participant "orders::OrderService" as os`, write `os`.

---

## 3. Linking the three diagrams

To link an architecture design to its detailed design, both must resolve to the
same identifier: **same nesting path**, **same name**.

**component_diagram.puml**

```text
@startuml component_diagram
package logging {
    component Recorder <<component>> {
        component Backend <<unit>>
    }
}
@enduml
```

**class_diagram.puml**

```text
@startuml class_diagram
package logging {
    interface IBackend {
        + Write() : void
    }
}
@enduml
```

**sequence_diagram.puml**

```text
@startuml sequence_diagram
participant "logging::Recorder::Backend" as Backend
participant "logging::IBackend" as IBackend
Backend -> IBackend : Write()
@enduml
```

| Diagram | Element | Identifier | Links to |
|---------|---------|-----------|----------|
| component | `Backend` | `logging.Recorder.Backend` | ← sequence `Backend` |
| sequence | `Backend` | `logging.Recorder.Backend` | ✅ |
| class | `IBackend` | `logging.IBackend` | ← sequence `IBackend` |
| sequence | `IBackend` | `logging.IBackend` | ✅ |

A different identifier means a different element ([Rule F](#rule-f)): there is
no parse error, cross-diagram validation reports the mismatch.

### Referring to another element

Relationship endpoints, `extends` / `implements` targets, port owners and
component relation endpoints refer to other elements by **reference path**:
the alias when there is one, else the name. The reference is looked up from the
scope `S` it is written in ([Rule C](#rule-c)):

| You write | Resolves to |
|-----------|-------------|
| simple key `r` | `S.r` if it exists, else the only element in the diagram whose key leaf is `r` |
| qualified key `a.r` or `a::r` | `S.a.r` or `a.r`; exactly one of them must exist |
| `.r`, `.a.r`, `::r` or `::a::r` | `r` or `a.r` from the root, even if `S` has its own `r` |

No match, several matches, or both paths of a qualified key existing is an
error. The name of an aliased element is not a reference: after
`class "a::Foo" as F`, write `F`; `a::Foo` is a
`NameOfAliasedElement` error. In class diagrams a reference only sees elements
declared above it; only `S.r` may also be declared below.

```text
@startuml class_diagram
package logging {
    class Recorder
    interface IBackend
}
Recorder --> logging::IBackend
@enduml
```

The reference `logging::IBackend` resolves to `logging.IBackend`.

**How this compares.** The lookup follows PlantUML: a reference resolves to the
element PlantUML draws the arrow to. Where PlantUML would create a new, empty
element instead, or where its choice depends on declaration order, the resolver
reports an error. This differs from C++ name lookup, which walks outwards
through the enclosing scopes. `package` and `namespace` behave the same.

In the table below, examples use class syntax: `a { … }` is a package or
namespace, a bare name such as `X` declares a class, and `C` is a class
declared next to the reference. The PlantUML column was observed with PlantUML
1.2025.9; "new element" means PlantUML draws the arrow to a newly created,
empty element.

| # | Example | C++ | PlantUML | Resolver |
|---|---------|-----|----------|----------|
| 1 | `X`; `a { X; C --> X }` | `a.X` | `a.X` | `a.X` |
| 2 | `X`; `a { C --> X }` | `X` | `X` | `X` |
| 3 | `X`; `a { X; b { C --> X } }` | `a.X` | new element | error: ambiguous (`X`, `a.X`) |
| 4 | `X`; `q { X }`; `a { C --> X }` | `X` | new element | error: ambiguous (`X`, `q.X`) |
| 5 | `X`; `a { m { X }; C --> X }` | `X` | new element | error: ambiguous (`X`, `a.m.X`) |
| 6 | `p { X }`; `C --> X` | error | `p.X` | `p.X` |
| 7 | `X`; `p { X }`; `C --> X` | `X` | `X` | `X` |
| 8 | `p { X }`; `q { X }`; `C --> X` | error | new element | error: ambiguous (`p.X`, `q.X`) |
| 9 | `a { x { Y }; C --> x.Y }` | `a.x.Y` | `a.x.Y` | `a.x.Y` |
| 10 | `x { Y }`; `a { C --> x.Y }` | `x.Y` | `x.Y` | `x.Y` |
| 11 | `x { Y }`; `a { x { Y }; C --> x.Y }` | `a.x.Y` | `x.Y` (the one declared first) | error: ambiguous (`a.x.Y`, `x.Y`) |
| 12 | `a { x { Y }; b { C --> x.Y } }` | `a.x.Y` | new element | error: unresolved |
| 13 | `p { x { Y } }`; `C --> x.Y` | error | new element | error: unresolved |
| 14 | `X`; `a { X; C --> .X }` | `X` (written `::X`) | `X` | `X` |
| 15 | `class "Foo" as F`; `C --> Foo` | — | new element | error: the name of aliased element `F` |
| 16 | `a { C --> Y }`; then `b { Y }` | error | new element | error: unresolved |

- The resolver never picks a different element than PlantUML.
- Where C++ and the resolver both find an element, it is the same one, with
  one exception. If both `a.x.Y` and a root `x.Y` exist, `x.Y` written inside
  `a.b` is `a.x.Y` in C++ but `x.Y` here and in PlantUML.
- To fix an ambiguous reference, qualify it (rows 3–5) or start it with `.`
  (row 14).

**Component diagrams.** PlantUML keeps one flat name space per component
diagram: two `component X` in different packages are drawn as one box. Give
every element of a component diagram a unique name, and refer to it by that
name.

---

## 4. Several files, one view

A view (`static`, `dynamic`, `public_api`, `internal_api`) may consist of
several files, for example an overview and a detail diagram. The parser
resolves all files of one view and diagram kind as one **group**:
`architectural_design` runs one group per view, `unit_design` one for `static`
and one for `dynamic`. Principle: a **declaration is authoritative** and gets
the id of where it is written; a **reference** that finds nothing in its own
file is resolved against the group. Each file still produces its own output;
only the ids are shared.

| Kind | Declarations | References |
|------|--------------|------------|
| component | A top-level element with a body is reparented onto a matching nested declaration | A relation target may be declared in another file |
| class | A top-level class, struct, interface, enum, package or namespace with a body is reparented onto a matching nested declaration | A relationship, `extends` or `implements` target may be declared in another file |
| sequence | No reparenting: a participant is flat and its name is its full path | A message, lifecycle or `ref over` name may be declared in another file |
| activity | No names, nothing to merge | — |

### Reparenting

```text
' overview.puml                          ' detail.puml (no wrapper)
package logging <<SEooC>> {              component Recorder <<component>> {
    component Recorder <<component>>         component Backend <<unit>>
}                                        }
```

| Written in | Element | Identifier |
|------------|---------|-----------|
| `overview.puml` | `Recorder` | `logging.Recorder` |
| `detail.puml` | `Recorder` | `logging.Recorder` (reparented) |
| `detail.puml` | `Backend` | `logging.Recorder.Backend` |

The top-level `Recorder` of the detail file has a body and matches the nested
`Recorder` of the overview by **name**, **element type** and **stereotype**, so
its id moves under `logging`. Everything it contains follows. The alias plays no
part: the overview may write `component Recorder as R` and the detail file
still matches.

| Case | Result |
|------|--------|
| Declaration without a body in the detail file | Not reparented: a separate top-level element, no error here; validation reports it as an extra element |
| Different element type or stereotype | Not a match; stays top-level |
| Rooted name `::X` | Never reparented |
| Matches under two different scopes | `AmbiguousImplicitParent` |
| The parent is itself reparented | Resolved transitively; a cycle is `ImplicitParentCycle` |

In component diagrams validation then checks that every `<<component>>` and
`<<unit>>` is nested under a `<<SEooC>>` or `<<component>>`. A top-level one
means the reparenting did not happen.

### References

A reference is first resolved in its own file. Only when nothing matches there
is it looked up in the other files, by the reference path the declaring file
wrote (its alias, else its name):

- Found in one other file: that element's id.
- Found with different ids in several other files: `AmbiguousReference`.
- A relation **source** must be declared in the file that writes it, otherwise
  `CrossFileRelationSource`. An undirected component relation has no source,
  its ends are swapped.

### Class diagrams

The same reparenting applies; the overview entry is written without a body:

```text
' overview.puml                          ' detail.puml (no wrapper)
package Lib {                            class Foo {
    package Sub {                            + run() : void
        class Foo                        }
    }
}
```

Both declare `Lib.Sub.Foo`; the detail file's `Foo` is reparented onto the
overview's. Validation merges the declarations of one id across the files of a
view:

| Case | Result |
|------|--------|
| One declaration has a body, the other has none | Merged; the declaration with the body wins |
| Two declarations both have a body | `class "<id>" is defined more than once` |
| Same id, different element type or stereotype | `re-declared with a conflicting entity type` / `stereotype` |

For reparenting, a body is any member, type alias, `extends` or `implements`
(enum: any literal). For the merge, any member, type alias or enum literal;
the relationships of all declarations are united, so an overview may draw them
while the class stays a reference.

### Sequence diagrams

Participants are flat, so there is nothing to reparent. Declare the cast once
in a shared file with qualified names and aliases; scenario files then write
messages using only the aliases:

```text
' cast.puml
participant "logging::Recorder::Backend" as Backend

' scenario.puml (declares no participant)
Backend -> Backend : Write()
activate Backend
```

`Backend` in the scenario resolves to `logging.Recorder.Backend` through the
cast. Rules:

- Only explicit `participant`-style declarations of the other files are
  offered; an undeclared message endpoint of another file is not.
- A message endpoint or a `create` that no declaration of the file covers is
  looked up in the other files. A local declaration always wins.
- Two other files declaring the same reference key with different ids are
  `AmbiguousParticipantReference`; the same id twice is fine.
- `activate`, `deactivate`, `destroy` and `ref over` do not consult other
  files: they name a participant the scenario already uses.
- A bare name is never completed to a qualified one that another file
  declares.

### Activity diagrams

Activity diagrams carry no names, so a group is just the independent
documents: there is nothing to merge or reference.

---

## 5. Special cases

**`ExternalEndpoint`** — the reserved marker for an actor outside the described
architecture. It is matched on the name and emitted verbatim, with no scope
([Rule E](#rule-e)).

```text
participant ExternalEndpoint
participant "ExternalEndpoint" as ext
ExternalEndpoint -> Backend : Notify()
```

→ `ExternalEndpoint`, in every diagram, so it always matches itself.
`participant "Outside World" as ExternalEndpoint` is a free-text error.
Participant uids are unique, so a diagram declares the marker at most once.

**FTA diagrams** — node aliases that are TRLC fully-qualified names
(`Package.Record`) address TRLC safety records, not architecture elements. They
are emitted verbatim ([Rule E](#rule-e)). Activity diagrams carry no
identifiers at all.

**Implicit participants** — PlantUML accepts a message to an undeclared quoted
participant (`Caller -> "Display Service" : call()`); this toolchain rejects
it. Declare the participant explicitly.

**Diagram kind detection** — unless a view fixes the kind (`static` of
`architectural_design` is component-only), each file is tried as component,
activity, class, then sequence diagram, and the first parser that accepts it
wins. Some files are valid for several kinds: `package p { interface I }` and a
file of only `a -> b : call()` parse as *component* diagrams. Give every file a
construct of its own kind, so that it joins the right group:

| Kind | Construct that settles it |
|------|---------------------------|
| class | the `class`, `struct` or `abstract class` keyword, or a body with members |
| sequence | `participant`/`actor`, or `activate`, `deactivate`, `alt` … |

---

## 6. Errors you may hit

| Message | Cause | Fix |
|---------|-------|-----|
| `Invalid identifier: <text>: name is free text, expected an identifier path …` | A name with spaces or other characters outside letters, digits, `_` and the separators | Use an identifier path; move prose into a note or stereotype |
| `… is not a valid qualified path …` | A name with `.` or `::` and an empty segment or other characters (`a..b`, `my-pkg::unit`) | Fix the path |
| `Duplicate entity id: <id>` | Two elements resolve to the same identifier, also with different aliases | Rename one — `core::User` and `core.User` are the *same* identifier ([Rule F](#rule-f)) |
| `DuplicateAlias` | Two declarations share a reference key in one scope | Rename one alias |
| `Ambiguous reference: <ref> -> <candidates>` | A reference matches more than one declared entity | Qualify the reference, or start it with `.` |
| `Unresolved reference: <ref>` | A reference matches no element: a path that does not exist, or (class diagrams) an element declared further down in another scope | Qualify the path, or declare the element first |
| `NameOfAliasedElement` | A reference uses the name of an element that has an alias | Refer to the element by its alias |
| `duplicate sequence participant id <uid>` | Two participants resolve to the same identifier | Rename one |
| `duplicate sequence participant name <name>` | Two participants share a reference key | Rename one of the aliases |
| `unknown sequence participant <name>` | `activate`, `deactivate`, `destroy` or `ref over` names a participant that is not declared | Declare it first, and name it by its alias |
| `AmbiguousImplicitParent` | A top-level declaration matches nested declarations under several scopes | Make the overview unambiguous, or root the name with `::` |
| `ImplicitParentCycle` | Files reparent each other's declarations | Remove one of the wrappers |
| `CrossFileRelationSource` | A relation source is declared in another file | Write the relation in the file that declares its source |
| `AmbiguousParticipantReference` | Two other files declare the reference key with different ids | Declare the participant in the file, or declare it once |
| `class "<id>" is defined more than once` | Two files declare a class with a body | Keep the body in one file; write the other without |
| `class "<id>" is re-declared with a conflicting …` | Two files declare one id with a different element type or stereotype | Make the declarations agree |

---

## 7. Best practices

1. **Name elements as in Bazel and C++.** `<<SEooC>>`, `<<component>>` and
   `<<unit>>` names equal the Bazel target names; class names equal the C++
   names.
2. **Skip the alias unless needed.** Use one for long or qualified names
   (`class "score::mw::com::Proxy" as Proxy`).
3. **Keep prose out of names.** Use a note, a stereotype or a relation
   description.
4. **In sequence diagrams, write the full path as the name** when the unit is
   nested: `participant "package::Component::Unit" as unit`.
5. **Mirror the nesting** between component and class diagrams — the scope
   segments must be identical on both sides.
6. **Use `ExternalEndpoint` verbatim** for out-of-scope actors.
7. **Treat identifiers as derived, not authored.** To change one, change the
   structure (nesting or name) — there is no override.
8. **Make references unambiguous.** If a name exists in several scopes, qualify
   the reference (`core.User`) or anchor it at the root (`.User`). Keep names
   unique within a component diagram. In class diagrams, declare an element
   before referring to it from another scope.
9. **Split a view by detail, not by identity.** Write the nesting once in an
   overview and the content in a detail file whose top-level element carries
   the same name, type and stereotype. Do not repeat the wrapper.
10. **Declare a cast once.** Put the participants of a unit design in one
    file; scenario files use the aliases and contain a sequence-only construct
    (`activate`, `alt`, …) so they are detected as sequence diagrams.

---

## Appendix — the formal rules

The parser never derives an identifier. It records the written name, the alias
and the enclosing scope. All identifier construction happens in the resolver.
The resulting field is `LogicComponent.id` for components, `SimpleEntity.id`
for classes, and `SequenceParticipant.uid` for sequence participants.

### Definitions

`normalize(s)` = `s.replace("::", ".").trim('.')` — `::` and `.` are equivalent
separators and there are no leading or trailing dots.

`join(a, b, …)` = the non-empty arguments joined with `.`.

### Rule A

**Identity text** (Rule A′) for class, interface, enum, struct, component, port,
package, namespace, every other component-grammar element (`node`, `cloud`,
`database`, `actor`, `usecase`, `rectangle`, …) and sequence participant:

1. Text = the written name with label markup stripped, first non-empty line
   (only the literal `\n` splits lines; `/n` is text).
2. Class-like kinds: strip one trailing balanced template argument list.
3. The text must be an identifier path — segments of letters, digits and `_`,
   separated by `.` or `::`, with an optional leading root marker. A single
   token with a separator that is not a valid path (`a..b`, `a.`,
   `my-pkg::unit`) is a malformed-path error; anything else is a free-text
   error.

`uid = join(internal_scope, normalize(text))`, with a leading root marker
dropped. A participant has no internal scope.

### Rule B

**Participant reference key.** A participant is referred to by its alias, else
by its declared name. The name of an aliased participant is not a reference.
Two participants with the same key or the same uid are an error, as is an
`activate`, `deactivate`, `destroy` or `ref over` on a name no participant
declares.

### Rule C

**Explicit scope path.** A name containing `.` or `::` is a qualified name. A
declaration nests it under the enclosing internal scope:
`uid = join(internal_scope, normalize(name))`. A name with a leading `.` or
`::` is rooted instead: `uid = join(normalize(name))`, the enclosing scope is
ignored. The parent of a declaration is all segments of the resulting
identifier except the last. It is a path derived from the identifier, not a
reference: the parent need not be declared in this diagram. A qualified port or
interface `a::I` therefore has parent `a`; a top-level one still has a parent,
so it counts as internal API.

**Reference paths.** Every declaration has two paths:

| Path | Built from | Used for |
|------|------------|----------|
| id path | enclosing id path + name segments; a root marker roots | ids, `parent_id`, output |
| reference path | enclosing reference path + key; key = alias, else the name segments | reference lookup |

Without aliases both paths are equal. With an alias, only the element's own
last segment differs: `package p { class "score::x::Y" as Y }` has id
`p.score.x.Y` and reference path `p.Y`.

**References.** A reference `r` written in internal scope `S` resolves to an
existing element. Let `local = join(S, normalize(r))` and
`rooted = normalize(r)`, both over reference paths.

1. `r` starts with `.` or `::`: `rooted`.
2. `r` contains no `.` or `::`: `local` if it exists, else the unique element
   whose reference leaf is `r`.
3. Otherwise: `local` or `rooted`, whichever exists.
4. Component diagrams: if no element matches, steps 2 and 3 apply to ports,
   and a match resolves to the port's owning component.

The hit maps to its id. No candidate is an unresolved reference; if a lookup
over the id paths of aliased declarations finds one, it is
`NameOfAliasedElement`. Two different candidates in step 2 or 3 are an
ambiguous reference. In class diagrams only elements declared before the
reference count, plus `local` declared after it. Component diagrams consider
the whole diagram.

### Rule D

**Root anchor.** Not applied. Ids are unique per dependable element. A root
prefix for aggregated builds is parked; the intended shape is explicit roots
(the `<<SEooC>>` name is the first segment, class ids are the C++ names).

### Rule E

**Exemptions.** Emitted verbatim, with no scope: FTA node aliases that are
valid TRLC FQNs (they address TRLC records, not architecture elements), and the
reserved sequence participant `ExternalEndpoint`, matched on its name.

### Rule F

**Uniqueness.** Within one diagram, two elements resolving to the same
identifier is an error, also when their aliases differ. Equal reference paths
in one scope are `DuplicateAlias`. Across diagrams, an identical identifier
means "the same architecture element", which is exactly what cross-diagram
consistency validation compares. When component diagrams are merged,
declarations of one identifier merge, and two names compare equal if they are
the same identifier path in either spelling (`a::b::C` and `a.b.C`).

### Rule G

**Group resolution.** The files of one view and diagram kind resolve together.

1. *Reparenting.* A top-level declaration `D` with a body that is not rooted
   has a candidate scope for every nested declaration `N` of another file with
   the same normalized name, element type and stereotype: the id of `N`
   without its last segment, itself reparented if the top-level ancestor of `N`
   is. No candidate keeps `D` top-level, one candidate becomes `D`'s parent
   scope, several are `AmbiguousImplicitParent`, a cyclic chain is
   `ImplicitParentCycle`. Only the id path moves; reference paths stay
   file-local. Component and class diagrams only.
2. *Peer references.* A reference that is unresolved in its file is resolved
   by Rule C at the root of every other file. One distinct id is the result,
   several are an ambiguous reference. A name that matches an aliased element
   of its own file stays `NameOfAliasedElement`. A relation source must
   resolve in its own file.
3. *Sequence.* No reparenting. A message endpoint or `create` that no
   declaration of the file covers resolves to the explicit participant
   declarations of the other files by reference key: one distinct uid is the
   result, several are `AmbiguousParticipantReference`. Lifecycle and `ref over`
   statements stay per file.
4. *Activity.* Every document resolves on its own.
