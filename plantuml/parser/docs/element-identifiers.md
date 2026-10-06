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

## 0. Implementation status

This guide describes the target design. As of this writing, only part of it is
implemented on `main`; the rest lands incrementally. The
[cross-diagram test suite](../integration_test/cross_diagram/) pins down
exactly what is true today with executable goldens — when a row below changes,
that suite's goldens change with it.

| Topic | Today (`main`) | Target (this guide) | Test case |
|-------|----------------|----------------------|-----------|
| Root anchor ([§1](#1-the-three-inputs), [Rule D](#rule-d)) | **not implemented** — identifiers have no Bazel-package prefix | `ctx.label.package` prepended to every identifier | — |
| Component id leaf ([§2](#2-component-diagrams), [Rule A](#rule-a)) | implemented — alias when present, else name | alias when present, else name | `component_nesting` |
| Class id leaf ([§3](#3-class-diagrams), [Rule A](#rule-a)) | implemented — alias when present, else name | alias when present, else name | `class_alias_wins` |
| `name` field of a class/component entity ([Rule A](#rule-a)) | implemented — the written name with its spelling kept and one leading root marker removed; a component with an alias keeps its `name` verbatim (display text) | same | `rooted_declaration_in_package` |
| Class/component reference resolution ([§5](#referring-to-another-element), [Rule C](#rule-c)) | class and component: implemented per Rule C (class: declaration-order visibility; component: whole diagram, ports resolve to their owner; ambiguity is an error) | one lookup for both: `S.r`, else the unique leaf; qualified: `S.r` or the root path; leading `.` or `::` = root; class references see earlier declarations only; ambiguity is an error | `qualified_reference`, `relation_simple_name_prefers_direct_hit` |
| Sequence participant identity ([§4](#4-sequence-diagrams), [Rule B](#rule-b)) | implemented — `uid` is the qualified label, else the alias, else the bare name; without a root anchor | same, plus the root anchor | `sequence_forms`, `prose_without_alias`, `uid_*` resolver cases |
| Sequence ↔ component/class linking ([§5](#5-linking-the-three-diagrams)) | partly implemented — the resolvers emit component/class id == participant uid (idmap links); the validators still compare aliases and display names | component/class id == participant uid in the resolvers and the validators | `linking_three_diagrams`, `component_nesting` |
| `ExternalEndpoint` marker ([§6](#6-special-cases), [Rule E](#rule-e)) | implemented — emitted verbatim | emitted verbatim, never anchored | `doc_6_external_endpoint` |
| Errors in [§7](#7-errors-you-may-hit) (`free-text participant display names require an alias…`, `is the display name of an aliased participant…`, `Duplicate entity id`, `Ambiguous reference`, `Unresolved reference`, `duplicate sequence participant id`, `duplicate sequence participant name`, `unknown sequence participant`) | implemented | as described | `prose_without_alias`, `errors_participants` |
| Id normalization (`::` / `.` equivalence, [Definitions](#definitions)) | implemented for class and component ids, scope paths, ports and relationship endpoints (incl. `extends`/`implements`); both diagram parsers accept `.`, `::` and the leading root marker in every name position ([Separators and root markers](#separators-and-root-markers)); the component merge compares names after normalization; sequence participant uids are normalized the same way | works everywhere an identifier is read or written, sequence included | `namespace_and_package`, `qualified_reference`, `relation_quoted_name`, `separator_equivalence` |
| Label markup stripping (creole tags in labels) | implemented for activity diagram labels and sequence participant labels (before Rule B derivation) | same | — |
| Qualified name inside a nested declaration ([Rule C](#rule-c)) | implemented — nests under the enclosing scope; a leading `.` or `::` is rooted (class and component diagrams); the parent is the id minus its last segment, whether or not it is declared in this diagram | nests under the enclosing scope; a leading `.` or `::` roots it | `qualified_in_nested_scope`, `rooted_declaration_in_package`, `rooted_dotted_declaration`, `qualified_interface_top_level`, `qualified_port_owner` |
| Cross-diagram hyperlinks (`idmap`) for sequence participants | implemented — identifier-based, same as component/class | same | `linking_three_diagrams` |

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

### Separators and root markers

`.` and `::` spell the same name, and a leading `.` or `::` roots it
([Rule C](#rule-c)). `a.b.X`, `a::b::X` and `a.b::X` are one identifier;
`.a.X` and `::a::X` ignore the enclosing scope. The parser accepts every
spelling wherever a name is written: declarations, `package` / `namespace`
names, ports, `extends` / `implements` targets and relationship endpoints.
Malformed paths (`a..b`, `a::`, `a:::b`, a lone `::`) are parse errors.

| Spelling | Class diagram | Component diagram | Sequence diagram |
|----------|---------------|-------------------|------------------|
| `a.b.X`, `.a.X` | parsed and rendered by PlantUML | parsed and rendered by PlantUML | not yet |
| `a::b::X` in a declaration, port or `() a::I` | parsed and rendered | parsed, **not rendered** by PlantUML | not yet |
| `A --> a::X` (endpoint) | parsed and rendered | parsed and rendered | not yet |
| `::a::X` (rooted, any position) | parsed and rendered | parsed, **not rendered** by PlantUML | not yet |

The parser is deliberately more permissive than PlantUML in component
diagrams. Write `.` in component diagrams if the diagram must also render; the
identifier is the same.

Two spellings in component diagrams need care:

- `component .a.C` declares `a.C`. A lone `.` between two endpoints stays
  PlantUML's dotted arrow.
- In a relation, `A --> B::text` is the qualified endpoint `B::text`, while
  `A --> B:: text` and `A --> B ::text` are endpoint `B` with a description.
  Write the label separator as ` : ` with spaces.

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

The same holds for a qualified label: `class "ns::X" as X` is
`score.mw.log.X`, not `score.mw.log.ns.X`. To get the nested identifier, declare
the class inside `namespace ns { … }` or write `class ns::X` without an alias.

A qualified name in a nested declaration is appended to the enclosing scope,
like a relative name in C++. A leading `::` (or `.`) roots it at the root
anchor, like a leading `::` in C++:

```text
package outer {
    class core::Circle      ' → score.mw.log.outer.core.Circle
    class ::core::Square    ' → score.mw.log.core.Square
}
```

---

## 4. Sequence diagrams

This is the one that behaves differently, and the one most likely to trip you
up.

> **In a sequence diagram the alias is not the identity when the label is a
> qualified name.** The alias is a local shortcut for drawing arrows. The
> identity is read out of the **quoted label** if that is a qualified path, else
> out of the alias ([Rule B](#rule-b)).

Sequence diagrams have no nesting, so the whole scope has to be written into
the label.

### The label forms

| What you write | Identity is taken from | Resulting identifier |
|----------------|------------------------|----------------------|
| `participant "logging::Recorder::Backend" as Backend` | the label (a qualified path wins over the alias) | `score.mw.log.logging.Recorder.Backend` |
| `participant "logging.IBackend" as IBackend` | the label, `.` and `::` are equivalent | `score.mw.log.logging.IBackend` |
| `participant "Log Client" as Client` | label is prose → the **alias** | `score.mw.log.Client` |
| `participant "backend : logging::Recorder::Backend" as Backend` | a label with spaces or `:` is prose → the **alias** | `score.mw.log.Backend` |
| `actor Client` | the bare name | `score.mw.log.Client` |

All participant kinds behave identically — `participant`, `actor`, `boundary`,
`control`, `entity`, `queue`, `database`, `collections`.

Messages, `activate`, `deactivate`, `destroy`, `create` and `ref over` name a
participant by its alias, or by its declared name when it has no alias. The
display name of an aliased participant is not a name: after
`participant "Display Service" as DisplayService`, write `DisplayService`.

### The trap

The prose form is the old habit, and it is the one that silently fails to link:

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
participant "package_a::component_a::unit_1" as unit_1 <<unit>>
```

The arrows still use the short alias and the identifier now matches the
component diagram. The label carries the path, so it is no longer free to hold
readable prose.

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
participant "logging::Recorder::Backend" as Backend
participant "logging::IBackend" as IBackend
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

### Referring to another element

Relationship endpoints, `extends` / `implements` targets and component
relation endpoints refer to other elements by name. The name is looked up from
the scope `S` it is written in ([Rule C](#rule-c)):

| You write | Resolves to |
|-----------|-------------|
| simple name `r` | `S.r` if it exists, else the only element in the diagram whose leaf is `r` |
| qualified name `a.r` or `a::r` | `S.a.r` or `a.r` from the root anchor; exactly one of them must exist |
| `.r`, `.a.r`, `::r` or `::a::r` | `r` or `a.r` from the root anchor, even if `S` has its own `r` |

No match, several matches, or both paths of a qualified name existing is an
error. The label of an aliased element is not a name: after
`class "Foo" as F`, write `F`. In class diagrams a reference only sees elements
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

The reference `logging::IBackend` resolves to `score.mw.log.logging.IBackend`.

**How this compares.** The lookup follows PlantUML: a reference resolves to the
element PlantUML draws the arrow to. Where PlantUML would create a new, empty
element instead, or where its choice depends on declaration order, the resolver
reports an error. This differs from C++ name lookup, which walks outwards
through the enclosing scopes. `package` and `namespace` behave the same.

In the table below:

- Examples use class syntax: `a { … }` is a package or namespace, a bare name
  such as `X` declares a class, and `C` is a class declared next to the
  reference.
- Identifiers are shown without the root anchor.
- The PlantUML column was observed with PlantUML 1.2025.9. "New element"
  means PlantUML draws the arrow to a newly created, empty element.

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
| 15 | `class "Foo" as F`; `C --> Foo` | — | new element | error: unresolved |
| 16 | `a { C --> Y }`; then `b { Y }` | error | new element | error: unresolved |

- The resolver never picks a different element than PlantUML.
- Where C++ and the resolver both find an element, it is the same one, with
  one exception. If both `a.x.Y` and a root `x.Y` exist, `x.Y` written inside
  `a.b` is `a.x.Y` in C++ but `x.Y` here and in PlantUML.
- To fix an ambiguous reference, qualify it (rows 3–5) or start it with `.`
  (row 14).

**Component diagrams.** PlantUML keeps one flat name space per component
diagram: two `component X` in different packages are drawn as one box. Give
every element of a component diagram a unique alias, and refer to it by that
alias.

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

Only the alias or the bare declared name counts: `participant ExternalEndpoint`
and `participant "Outside" as ExternalEndpoint` are the marker, while
`participant "ExternalEndpoint" as ext` is an ordinary participant with uid
`ext`. Participant uids are unique, so a diagram declares the marker at most
once.

**FTA diagrams** — node aliases that are TRLC fully-qualified names
(`Package.Record`) address TRLC safety records, not architecture elements. They
are emitted verbatim and never anchored ([Rule E](#rule-e)). Activity diagrams
carry no identifiers at all.

---

## 7. Errors you may hit

| Message | Cause | Fix |
|---------|-------|-----|
| `free-text participant display names require an alias for uid derivation` | A quoted, prose participant label with no `as` alias, declared or used as a message endpoint | Add an alias, or write a qualified label |
| `Duplicate entity id: <id>` | Two elements resolve to the same identifier | Rename one — note `core::User` and `core.User` are the *same* identifier ([Rule F](#rule-f)) |
| `Ambiguous reference: <ref> -> <candidates>` | A reference matches more than one declared entity | Qualify the reference, or start it with `.` for the root path |
| `Unresolved reference: <ref>` | A reference matches no element: an aliased element's label, a path that does not exist, or (class diagrams) an element declared further down in another scope | Refer to the element by its alias or leaf, qualify the path, or declare the element first |
| `duplicate sequence participant id <uid>` | Two participants resolve to the same identifier | Rename one, or change its label or alias |
| `duplicate sequence participant name <name>` | Two participants share a reference name (an alias, or a declared name without alias) | Rename one of the aliases |
| `is the display name of an aliased participant, refer to it by its alias` | A message names the display name of an aliased participant | Use the alias |
| `unknown sequence participant <name>` | `activate`, `deactivate`, `destroy` or `ref over` names a participant that is not declared | Declare it first, and name it by its alias |

Two forms that PlantUML accepts but this toolchain now rejects:

```text
participant "Order Service"              ' ✗ prose label, no alias
Caller -> "Display Service" : call()     ' ✗ implicit participant by display name
```

Declare them explicitly instead:

```text
participant "orders::OrderService" as OrderService
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
   `"package::Component::Unit"`. Use the alias only for arrows.
4. **Mirror the nesting** between component and class diagrams — the scope
   segments must be identical on both sides.
5. **Use `ExternalEndpoint` verbatim** for out-of-scope actors.
6. **Treat identifiers as derived, not authored.** If you need a different
   identifier, change the structure (nesting, alias, or owning Bazel package) —
   there is no override.
7. **Make references unambiguous.** If a name exists in several scopes,
   qualify the reference (`core.User`) or anchor it at the root (`.User`).
   Keep aliases unique within a component diagram. In class diagrams, declare
   an element before referring to it from another scope
   ([§5](#referring-to-another-element)).

---

## 9. Current limitations

See the [implementation status table](#0-implementation-status) for what is and
isn't implemented yet.

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
- *Class:* `internal_scope` = the enclosing namespace/package chain, each
  segment its alias, else its name; `leaf` = alias, else name.
- *Sequence:* the identity is derived from the display name and the alias
  ([Rule B](#rule-b)); participant nesting does not exist.

The `name` field of a class or component is not part of the identifier. It is
the written name with its spelling kept (`a::b::C` stays `a::b::C`) and one
leading root marker removed. A component with an alias keeps its `name`
verbatim, because it is display text (`component ".NET" as dotnet`).

### Rule B

**Participant uid.** Take the first non-empty line of the display name, with
label markup stripped (only the literal `\n` splits lines; `/n` is text). Then,
in order:

1. the line is a qualified path — segments of letters, digits and `_`,
   separated by `.` or `::`, with an optional leading root marker → `text` =
   the path, even when an alias exists;
2. otherwise `text` = the alias;
3. no alias → `text` = the line, if it is a plain identifier (letters, digits,
   `_`, `-`, `@`, `.`);
4. otherwise (free-text label without alias) → error.

Then `uid = join(root_anchor, normalize(text))`, with a leading root marker
dropped; `ExternalEndpoint` is emitted verbatim ([Rule E](#rule-e)). A label
with spaces or a `:` is prose, there is no `instance : Type` form. The alias is
the local reference key that binds messages to participants. A participant is
referred to by its alias, else by its declared name; the display name of an
aliased participant is not a reference, naming it in a message is an error. Two
participants with the same reference name or the same uid are an error, as is an
`activate`, `deactivate`, `destroy` or `ref over` on a name no participant
declares.

### Rule C

**Explicit scope path.** If the name selected by [Rule A](#rule-a) /
[Rule B](#rule-b) contains `.` or `::`, it is a qualified name. A declaration
nests it under the enclosing internal scope:
`uid = join(root_anchor, internal_scope, normalize(name))`. A name with a
leading `.` or `::` is rooted instead: `uid = join(root_anchor, normalize(name))`,
the enclosing scope is ignored. The parent / enclosing
namespace of a declaration is all segments of the resulting identifier except
the leaf. It is a path derived from the identifier, not a reference: the parent
need not be declared in this diagram (it may be declared in another diagram or
nowhere). A qualified port or interface `a::I` therefore has parent `a`; a
top-level one still has a parent, so it counts as internal API. Sequence
participants have no enclosing scope, so their path is
always below the root anchor.

**References.** A reference `r` written in internal scope `S` (class
relationship endpoint, `extends`, `implements`, component relation endpoint)
resolves to an existing element. Let
`local = join(root_anchor, S, normalize(r))` and
`rooted = join(root_anchor, normalize(r))`.

1. `r` starts with `.` or `::`: `rooted`.
2. `r` contains no `.` or `::`: `local` if it exists, else the unique element
   whose leaf ([Rule A](#rule-a)) is `r` — or, when that leaf itself has
   separators (an unaliased qualified declaration), whose leaf's last segment
   is `r`.
3. Otherwise: `local` or `rooted`, whichever exists.
4. Component diagrams: if no element matches, steps 2 and 3 apply to ports,
   and a match resolves to the port's owning component.

No candidate is an unresolved reference. Two different candidates in step 2 or
3 are an ambiguous reference. In class diagrams only elements declared before
the reference count, plus `local` declared after it. Component diagrams
consider the whole diagram. [§5](#referring-to-another-element) compares the
result with C++ and PlantUML.

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
validation compares. When component diagrams are merged, declarations of one
identifier merge, and two names compare equal if they are the same identifier
path in either spelling (`a::b::C` and `a.b.C`).
