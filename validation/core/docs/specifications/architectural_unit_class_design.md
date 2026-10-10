<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Architectural Unit Class Design Specification

## Purpose

This validator enforces refinement consistency between a unit declared in an
architectural component diagram and the static class diagram of the
`unit_design` bound to that unit.

It shall make sure that each architectural unit is refined by at least one
class, struct, interface, or abstract class owned by that unit in its detailed
design. Enums alone do not satisfy this refinement requirement.

## What is Validated

The validator compares two inputs associated through the Bazel `unit` target:

| Input | Source | Meaning |
|---|---|---|
| Architectural unit | `architectural_design.static` component diagram | A `<<unit>>` entity and its canonical identifier |
| Detailed class design | `unit_design.static` class diagram bound to that Bazel `unit` | Class, struct, interface, and abstract-class entities that refine the unit |

The existing Bazel-component validation establishes the mapping from an
architectural `<<unit>>` to its Bazel `unit` target. This validator then uses
that target's `unit_design` binding to select the detailed class diagram to
compare. It does not validate the Bazel-to-component mapping itself.

### Unit-Class Refinement Consistency

For every architectural `<<unit>>`, the static class diagrams bound to its
corresponding Bazel `unit` target must declare at least one class, struct,
interface, or abstract class in that unit's canonical identifier path. Matching
uses the full architectural unit ID as a complete path prefix, followed by at
least one entity-name segment. An enum does not satisfy this requirement.

*(Requirement: {requirement:downstream-ref}`Tools.ArchitecturalUnitClassDesignConsistency`)*

For example, `Filter` is an architectural unit with canonical identifier
`system.control.Filter`. Its Bazel target has design name `Filter`; the unit
design bound to that target refines it when it declares an entity such as
`system.control.Filter.FilterService`.

```text
' architectural component diagram
package system {
    package control {
        component Filter <<unit>>
    }
}
```

```text
@startuml
namespace system {
  namespace control {
    namespace Filter {
      class FilterService
    }
  }
}
@enduml
```

The following does not refine `system.control.Filter`, because it has the same
leaf name under a different parent path:

```text
@startuml
namespace system {
  namespace sensor {
    namespace Filter {
  class SensorDriver
    }
  }
}
@enduml
```

Class diagrams are evaluated only within the class-diagram files bound to the
matching Bazel unit. A collaborator stub in another unit's namespace does not
refine the current unit.

References to external entities are permitted, but do not satisfy this rule.
For example, a relationship from `FilterService` to
`system.sensor.Sensor.SensorDriver` is an external reference; only an eligible
entity declared in `Filter`'s unit design can satisfy the requirement.

### Identifier Matching

Identifiers are compared as canonical identifier paths. The eligible entity
identifier must begin with the full architectural unit ID followed by `.` and
at least one additional entity-name segment. A bare or lexical prefix match is not
sufficient: `system.control.Filtering.Service` does not refine
`system.control.Filter`.

Aliases and display names do not take part in matching.

### Not Validated Here

This validator does not validate:

- the correspondence between architectural units and Bazel `unit` targets;
  that is covered by the Bazel Component validator.
- component-diagram interface declarations against public or internal API
  diagrams; that is covered by the Component Public API and Component Internal
  API validators.
- class members, methods, or relationships against implementation code; that
  is covered by the Class Design Implementation validator.
- whether external class references are allowed architectural dependencies.

## Failure Cases

| Failure case | Validation rule |
|---|---|
| The bound static unit-design class diagrams declare no eligible entity below the full architectural unit ID | Unit-Class Refinement Consistency |
| The bound static unit-design class diagrams only declare eligible entities below another architectural unit ID | Unit-Class Refinement Consistency |
| The bound static unit-design class diagrams only reference external entities | Unit-Class Refinement Consistency |

## Debug Output

The validator shall emit debug output containing:

- the architectural unit identifier;
- the Bazel `unit` target and its bound `unit_design` target;
- the eligible entity identifiers read from the bound static class diagrams;
- the eligible entity identifiers selected as refinements of each architectural unit.
