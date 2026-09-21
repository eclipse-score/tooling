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

This guide explains how that identifier is built from what you write, and how
to author diagrams so that the links actually resolve.

> You never write an identifier yourself. The parser never invents one either —
> it is computed during resolution from your diagram structure
> ([Rule 0](#rule-0)).

---

## 1. The three inputs

An identifier is assembled from exactly three things:

| # | Input | Where it comes from | Example |
|---|-------|---------------------|---------|
| 1 | **Root anchor** | The Bazel package of the `architectural_design` / `unit_design` target that owns the `.puml` file — **not** the file path | `score/mw/log` → `score.mw.log` |
| 2 | **Internal scope** | The `package` / `component` / `namespace` blocks you nest the element in | `logging.Recorder` |
| 3 | **Leaf** | The element's alias (`as X`), or its name when there is no alias | `Backend` |

They are joined with `.`:

```text
score.mw.log  .  logging.Recorder  .  Backend
└─ root anchor ┘ └ internal scope ┘   └ leaf ┘
```

Two normalization rules apply everywhere ([Definitions](#definitions)):

- `::` and `.` are **equivalent** separators — `logging::Recorder` and
  `logging.Recorder` produce the identical identifier.
- Leading and trailing dots and empty segments are dropped.

The root anchor is always prepended, also for nested elements
([Rule D](#rule-d)). An element in a `package` block does **not** lose the
package prefix.

All examples in this guide assume the owning target lives in Bazel package
`score/mw/log`, so the root anchor is `score.mw.log`.

---

## 2. Component diagrams

The identifier is the nesting chain plus the alias ([Rule A](#rule-a)).

```text
@startuml component_diagram
package "logging" as logging {
    component "Recorder" as Recorder <<component>> {
        component "Backend" as Backend <<unit>>
    }
}
@enduml
```

| Element | Identifier |
|---------|-----------|
| `logging` | `score.mw.log.logging` |
| `Recorder` | `score.mw.log.logging.Recorder` |
| `Backend` | `score.mw.log.logging.Recorder.Backend` |

The quoted label is display text only. `component "Recorder" as Recorder`
and `component "The Log Recorder" as Recorder` produce the same identifier.

---

## 3. Class diagrams

Same principle — `package` and `namespace` blocks both contribute scope
([Rule A](#rule-a)).

```text
@startuml class_diagram
package logging {
    interface "IBackend" as IBackend {
        + Write() : void
    }
}
@enduml
```

→ `score.mw.log.logging.IBackend`

```text
@startuml class_diagram
namespace logging {
    class Recorder
}
@enduml
```

→ `score.mw.log.logging.Recorder`

**The alias wins over the label**, including for enums:

```text
class "Sample Library API" as SampleLibraryAPI
enum "Event Level" as EventLevel
```

→ `score.mw.log.SampleLibraryAPI` and `score.mw.log.EventLevel` —
*not* `score.mw.log.Sample Library API`.

---

## 4. Sequence diagrams

This is the one that behaves differently, and the one most likely to trip you
up.

> **In a sequence diagram the alias is not the identity.** The alias is a local
> shortcut for drawing arrows. The identity is read out of the **quoted label**
> ([Rule B](#rule-b)).

Sequence diagrams have no nesting, so the whole scope has to be written into
the label.

### The four forms

| What you write | Identity is taken from | Resulting identifier |
|----------------|------------------------|----------------------|
| `participant "backend : logging::Recorder::Backend" as Backend` | text right of the single `:` | `score.mw.log.logging.Recorder.Backend` |
| `participant ":logging::IBackend" as IBackend` | same, instance name omitted | `score.mw.log.logging.IBackend` |
| `participant "logging::IBackend" as IBackend` | the whole label (no spaces → treated as a qualified name) | `score.mw.log.logging.IBackend` |
| `participant "Log Client" as Client` | label is prose → falls back to the **alias** | `score.mw.log.Client` |
| `actor Client` | the bare name | `score.mw.log.Client` |

All participant kinds behave identically — `participant`, `actor`, `boundary`,
`control`, `entity`, `queue`, `database`, `collections`.

### The trap

The last form is the old habit, and it is the one that silently fails to link:

```text
participant "Log Client" as Client        ' → score.mw.log.Client
```

If the component diagram declares that unit as
`score.mw.log.logging.Recorder.Backend`, these do not match. There is no parse
error — you only find out when cross-diagram validation reports a mismatch,
which is the intended behaviour: a different identifier means a different
element ([Rule F](#rule-f)).

### Migration pattern

Rewrite

```text
participant "Unit 1" as unit_1 <<unit>>
```

as

```text
participant "Unit 1 : package_a::component_a::unit_1" as unit_1 <<unit>>
```

You keep the readable label, the arrows still use the short alias, and the
identifier now matches the component diagram.

---

## 5. Linking the three diagrams

To link an architecture design to its detailed design, both must resolve to the
same identifier. In practice that means three things must line up: **same Bazel
package**, **same nesting path**, **same leaf name**.

```text
score/mw/log/BUILD
├── architectural_design(static = component_diagram.puml,
│                        dynamic = sequence_diagram.puml)
└── unit_design(static = class_diagram.puml)
```

**component_diagram.puml**

```text
@startuml component_diagram
package "logging" as logging {
    component "Recorder" as Recorder <<component>> {
        component "Backend" as Backend <<unit>>
    }
}
@enduml
```

**class_diagram.puml**

```text
@startuml class_diagram
package logging {
    interface "IBackend" as IBackend {
        + Write() : void
    }
}
@enduml
```

**sequence_diagram.puml**

```text
@startuml sequence_diagram
participant "backend : logging::Recorder::Backend" as Backend
participant ":logging::IBackend" as IBackend
Backend -> IBackend : Write()
@enduml
```

Resulting identifiers:

| Diagram | Element | Identifier | Links to |
|---------|---------|-----------|----------|
| component | `Backend` | `score.mw.log.logging.Recorder.Backend` | ← sequence `Backend` |
| sequence | `Backend` | `score.mw.log.logging.Recorder.Backend` | ✅ |
| class | `IBackend` | `score.mw.log.logging.IBackend` | ← sequence `IBackend` |
| sequence | `IBackend` | `score.mw.log.logging.IBackend` | ✅ |

> **Put the `architectural_design` and the `unit_design` for one subsystem in
> the same Bazel package.** Different packages mean different root anchors, and
> their identifiers can never match.

### Referring to an element by a qualified name

When you *reference* an element with a dotted or `::`-qualified name, that name
is read as a path starting at the root anchor — it is never an absolute
identifier ([Rule C](#rule-c)).

```text
@startuml class_diagram
package logging {
    class Recorder
}
Recorder --> logging::IBackend
@enduml
```

The reference `logging::IBackend` resolves to `score.mw.log.logging.IBackend`.

---

## 6. Special cases

**`ExternalEndpoint`** — the reserved marker for an actor outside the described
architecture. It is emitted verbatim: no root anchor, no scope
([Rule E](#rule-e)).

```text
participant ExternalEndpoint
ExternalEndpoint -> Backend : Notify()
```

→ `ExternalEndpoint`, in every diagram and every Bazel package, so it always
matches itself.

**FTA diagrams** — node aliases that are TRLC fully-qualified names
(`Package.Record`) address TRLC safety records, not architecture elements. They
are emitted verbatim and never anchored ([Rule E](#rule-e)). Activity diagrams
carry no identifiers at all.

---

## 7. Errors you may hit

| Message | Cause | Fix |
|---------|-------|-----|
| `free-text participant display names require an alias for uid derivation` | A quoted, prose participant label with no `as` alias | Add an alias, or write a qualified label |
| `multiple standalone ':' separators are not allowed` | e.g. `participant "a : b : c"` | Use at most one `:` — `"instance : Qualified::Type"` |
| `standalone ':' must have a non-empty right-hand side` | e.g. `participant "backend :"` | Write the type after the `:` |
| `Duplicate entity id: <id>` | Two elements resolve to the same identifier | Rename one — note `core::User` and `core.User` are the *same* identifier ([Rule F](#rule-f)) |
| `duplicate sequence participant id <uid>` | Two participants resolve to the same identifier | Same as above |

Two forms that PlantUML accepts but this toolchain now rejects:

```text
participant "Order Service"              ' ✗ prose label, no alias
Caller -> "Display Service" : call()     ' ✗ implicit participant by display name
```

Declare them explicitly instead:

```text
participant "Order Service : orders::OrderService" as OrderService
participant "Display Service" as DisplayService
```

---

## 8. Best practices

1. **Same Bazel package** for the `architectural_design` and `unit_design` of
   one subsystem.
2. **Always give an explicit `as` alias** to architecture-relevant elements, and
   make it a valid identifier (letters, digits, `_`). Never rely on a prose
   label.
3. **In sequence diagrams, always write the full path in the label**:
   `"instance : package::Component::Unit"`. Use the alias only for arrows.
4. **Mirror the nesting** between component and class diagrams — the scope
   segments must be identical on both sides.
5. **Do not mix nesting with qualified names.** Either nest the element in
   `package`/`namespace` blocks, or declare it at top level with a qualified
   name — not both (see limitations below).
6. **Use `ExternalEndpoint` verbatim** for out-of-scope actors.
7. **Treat identifiers as derived, not authored.** If you need a different
   identifier, change the structure (nesting, alias, or owning Bazel package) —
   there is no override.

---

## 9. Current limitations

Known gaps between this guide and the present implementation:

- **A qualified name used in a *declaration* inside a block is appended to the
  enclosing scope instead of replacing it.** For example
  `package outer { class core::geometry::Circle }` yields
  `score.mw.log.outer.core.geometry.Circle`, whereas the rule would call for
  `score.mw.log.core.geometry.Circle`. Avoid this combination (best practice 5).
- **Cross-diagram hyperlinks (`idmap`) are not yet identifier-based for
  sequence diagrams**, so clickable links from a sequence participant to its
  component may not resolve.
- **Cross-diagrams in Class Diagrams** also have a bug currently

---

## Appendix — the formal rules

The normative rules the resolvers implement for ID Generation. The parser never derives an identifier. It records
only alias, display name, and the enclosing scope as written. All identifier
construction happens in the resolver. The resulting field is
`LogicComponent.id` for components, `SimpleEntity.id` for classes, and
`SequenceParticipant.uid` for sequence participants.

### Definitions

`normalize(s)` = `s.replace("::", ".").trim('.')` — `::` and `.` are equivalent
separators and there are no leading or trailing dots.

`join(a, b, …)` = the non-empty arguments joined with `.`.

`root_anchor` = `normalize(ctx.label.package)` of the owning `unit_design` /
`architectural_design` target, and `""` when not supplied (unit tests and
standalone CLI runs).

### Rule A

**Leaf and internal scope, per diagram kind.**

- *Component:* `internal_scope` = the enclosing package/component nesting;
  `leaf` = alias, else name (error if both absent).
- *Class:* `internal_scope` = the enclosing namespace/package FQN; `leaf` =
  alias, else name.
- *Sequence:* both are derived from the label ([Rule B](#rule-b)); participant
  nesting does not exist.

### Rule B

**Sequence label (`type_text`).** Take the first non-empty line of the display
name, then, in order:

1. exactly one standalone `:` → `type_text` = the text right of it;
2. no standalone `:` and a bare qualified identifier (no spaces) → `type_text`
   = the whole line;
3. otherwise (free-text label) → `type_text` = the alias.

Then `internal_scope` = all segments of `normalize(type_text)` except the last,
and `leaf` = the last segment. The alias is a local reference key used to bind
messages to participants; it contributes to the identifier only in case 3. More
than one standalone `:`, or a `:` with an empty right-hand side, is an error.

### Rule C

**Explicit scope path.** If the name selected by [Rule A](#rule-a) /
[Rule B](#rule-b) contains `.` or `::`, it is an explicit scope path expressed
*relative to* `root_anchor`, never an absolute identifier:
`uid = join(root_anchor, normalize(name))`. The explicit path replaces the
enclosing internal scope; it is not appended to it. This applies identically to
component relation endpoints, class references, and sequence participants.

### Rule D

**Root anchor, applied unconditionally.** For every element not covered by
[Rule C](#rule-c): `uid = join(root_anchor, internal_scope, leaf)`. The root
anchor is a prefix, not a fallback — it is prepended whether or not the internal
scope is empty. Because `join()` drops empty parts, a build with no root anchor
yields the bare `internal_scope + leaf`, which is why unit tests observe
identifiers such as `domain_b.Controller`.

### Rule E

**Exemptions.** Emitted verbatim, with no root anchor and no scope: FTA node
aliases that are valid TRLC FQNs (they address TRLC records, not architecture
elements), and the reserved sequence participant `ExternalEndpoint`.

### Rule F

**Uniqueness.** Within one diagram, two elements resolving to the same
identifier is an error. Across diagrams, an identical identifier means "the same
architecture element", which is exactly what cross-diagram consistency
validation compares.
