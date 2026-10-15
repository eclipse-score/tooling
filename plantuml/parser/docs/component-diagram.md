<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Component Diagram Support Guide

This guide describes the syntax and semantics currently supported by the PlantUML component
diagram parser and resolver. The goal is to create component, interface, port, and relation
data that can be consumed by the logical model, rather than only rendering a diagram.

**Recommended guidelines:**

- Use identifier-path names for all elements, especially those referenced or linked across diagrams.
- Declare an alias for a qualified or otherwise inconvenient identifier-path name, then use that
  alias consistently.
- For a typical SEooC hierarchy, use a `package` stereotyped as <<SEooC>> for the outermost
  boundary, and use nested `component` elements stereotyped as <<component>> and <<unit>> for
  component and unit layers.
- Prefer explicit `interface`, `port`, `portin`, and `portout` declarations, and use the element
  form that matches the intended logical model.

## Minimal valid example

The following self-contained diagram models an SEooC, a component, a unit, and a required
interface binding:

```text
@startuml SampleArchitecture

package SampleSEooC <<SEooC>> {
  component ComponentA <<component>> {
    component UnitA1 <<unit>>
    interface InternalApi
    UnitA1 -( InternalApi : requires
  }
}

@enduml
```

After resolution, the logical model contains the identities `SampleSEooC`,
`SampleSEooC.ComponentA`, `SampleSEooC.ComponentA.UnitA1`, and
`SampleSEooC.ComponentA.InternalApi`, including their parent relationships and interface binding.

## Supported and modeled content

### Elements, nesting, and identifiers

Component diagrams model `package`, `component`, and `interface`. Other accepted component or
deployment keywords, such as `database` and `node`, are preserved as generic logical elements.
They retain their kind, name, stereotype, parent, and relations, but do not carry the component or
interface semantics required for interface bindings. Elements may be nested within `package`,
`component`, or other element blocks. An element's qualified identity is derived from its nesting
path and declared name; an alias does not contribute to that identity.

`package` and `component` retain distinct element kinds and render differently in PlantUML, but
both derive identifiers and parent relationships from their nesting paths. Choose the form that
matches the intended rendering; see [Interface binding constraints](#interface-binding-constraints)
for the additional endpoint restrictions.

```text
package SampleSEooC <<SEooC>> {
  component "OrderComponent" as Orders <<component>> {
    component OrderUnit <<unit>>
  }
  interface OrderApi
}
```

An element's declared name, together with its nesting path, determines its qualified identity.
An alias introduced with `as` is only a reference key and does not contribute to that identity.
Quoted names must still be valid identifier paths. See [Element Identifiers](element-identifiers.md)
for identifier construction and reference-resolution rules.

`together { ... }` is supported as a grouping construct. It does not create a separate logical
element; the contained declarations and relations keep their normal enclosing scope.

### Ports

`port`, `portin`, and `portout` declarations nested inside an element block are modeled:

```text
component Gateway <<component>> {
  portin request
  portout response
  port internal
}
interface RequestApi

request -( RequestApi : requires
```

Ports follow the same alias and reference-path rules as elements. A relation that resolves through
a port is lifted to the port's owning element in the logical model. `portin` represents a required
role, `portout` represents a provided role, and `port` has no role restriction unless a relation
form supplies one. Bare top-level port declarations are accepted for compatibility but ignored by
the logical model; declare a port inside its owning element to make it referenceable. See
[Element Identifiers](element-identifiers.md) for reference-path rules.

### Relations

#### Associations and dependencies

Association relations have no direction; dependency relations have direction. The resolver
normalizes reverse arrow spellings to the same source and target as their forward equivalents.

```text
A -- B : associated
A .. B : associated with a dotted line
A --> B : depends on
B <-- A : also A depends on B // not recommended
A ..> B : dotted dependency
B <.. A : also dotted dependency from A to B // not recommended
```

Line style does not change the logical relation category, and text after `:` is retained as the
relation annotation. For consistency and readability, prefer the forward forms `A --> B` and
`A ..> B`.

#### Interface bindings

Provided and required interface bindings are supported only with a `component` or qualifying
`package` source on the left and an interface target on the right:

```text
component Gateway <<component>> {
  portin incoming
  portout outgoing
}
interface RequestApi
interface ResponseApi

incoming -( RequestApi : requires
outgoing )- ResponseApi : provides
Gateway --() ResponseApi : also provides
```

`)-` and `--()` are provided-interface bindings. `-(` is a required-interface binding. The
generic lollipop form `Component --() Interface` is resolved as a provided binding. When a source
port is used, its role must agree with the decorator: `portin` with `-(` and `portout` with `)-`
or `--()`. PlantUML direction hints on interface-binding lines affect rendering only, not the
resolved role or endpoints.

## Content accepted but not included in the logical model

The parser accepts common PlantUML presentation constructs for compatibility, including titles,
legends, notes, styles, colors, `skinparam`, layout direction, `hide`/`show`, and footer text.
They do not create logical elements or relations, and visual styling does not change the meaning
of a modeled element or relation. Do not use visual-only content to express architecture that
downstream tools must understand.

## Resolver errors

### Identifiers and references

The resolver rejects duplicate element identities, duplicate aliases, invalid identifiers,
unresolved references, ambiguous references, and referring to an aliased element by its name
instead of its alias. Use a qualified identifier when a leaf name could identify more than one
element, and use the alias whenever one was declared.

### Interface binding constraints

An interface binding must have exactly one interface endpoint. Interface-to-interface bindings are
not allowed. The left endpoint must be a `component`, or a `package` stereotyped `<<SEooC>>` or
`<<component>>`; the right endpoint must be an interface.

For example, a component-stereotyped package may provide an interface. A package stereotyped as
`<<SEooC>>` is also accepted on the left:

```text
package Provider <<component>>
interface Api

Provider )- Api : provides
```

A plain `package` cannot be used as the left endpoint of an interface binding; see
[plain package binding rejection](../integration_test/component_diagram/invalid_package_no_stereotype_binding/invalid_package_no_stereotype_binding.puml).

The following forms are rejected even where a PlantUML renderer may display them:

```text
Interface )- Component
Interface -( Component
Interface --() Component
Component ()-- Interface
```

Mixed provided and required decorators in one relation are rejected. The resolver also rejects a
decorator whose role conflicts with the role of a referenced source port.

These errors include the relation or reference involved; source locations are retained where the
underlying parser supplies them. Representative negative cases include:

- [ambiguous reference](../integration_test/component_diagram/invalid_ambiguous_reference/invalid_ambiguous_reference.puml)
- [duplicate alias](../integration_test/component_diagram/invalid_duplicate_alias/invalid_duplicate_alias.puml)
- [interface-left decorator](../integration_test/component_diagram/invalid_interface_left_decorator/invalid_interface_left_decorator.puml)
- [interface binding between components](../integration_test/component_diagram/invalid_interface_decor_between_components/invalid_interface_decor_between_components.puml)
- [source port role mismatch](../integration_test/component_diagram/invalid_source_endpoint_role_mismatch/invalid_source_endpoint_role_mismatch.puml)

## Authoring checklist

Use the following checklist when submitting a component diagram:

1. The file starts with `@startuml` and ends with `@enduml`.
2. Elements use stable identifier-path names. Quoting a name does not permit free text; use an
  alias when an element needs a shorter local reference key.
3. A relation uses an alias consistently after that alias has been declared.
4. Nested elements, ports, and references use qualified paths whenever a simple name could be
   ambiguous.
5. Associations use `--` or `..`; dependencies use one directional arrow such as `-->` or `..>`.
6. Interface bindings put a `component` or qualifying `package` endpoint on the left and an
  interface on the right; `)-`/`--()` mean provided and `-(` means required.
7. `portin` is used with required bindings, `portout` with provided bindings, and generic `port`
   only when no port-role restriction is intended.
8. Architecture needed by downstream tools is represented by modeled elements and relations, not
   titles, styles, notes, or other visual-only directives.
