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
class owned by that unit in its detailed design.

## What is Validated

The validator compares two inputs associated through the Bazel `unit` target:

| Input | Source | Meaning |
|---|---|---|
| Architectural unit | `architectural_design.static` component diagram | A `<<unit>>` entity and its canonical identifier |
| Detailed class design | `unit_design.static` class diagram bound to that Bazel `unit` | The class entities that refine the unit |

The existing Bazel-component validation establishes the mapping from an
architectural `<<unit>>` to its Bazel `unit` target. This validator then uses
that target's `unit_design` binding to select the detailed class diagram to
compare. It does not validate the Bazel-to-component mapping itself.

### Unit-Class Refinement Consistency

For every architectural `<<unit>>`, the static class diagram of the
`unit_design` bound to its Bazel `unit` target must declare at least one class
whose canonical identifier has the architectural unit identifier as a complete
path prefix, followed by at least one additional class-name segment.

*(Requirement: {requirement:downstream-ref}`Tools.ArchitecturalUnitClassDesignConsistency`)*

For example, `Filter` is an architectural unit with canonical identifier
`system.control.Filter`. The unit design bound to `Filter` refines it when it
declares a class such as `system.control.Filter.FilterService`.

```text
' architectural component diagram
package system {
    package control {
        component Filter <<unit>>
    }
}
```

```text
' static class diagram in Filter's unit_design
package system {
    package control {
        package Filter {
            class FilterService
        }
    }
}
```

The following does not refine `system.control.Filter`, because the only class
belongs to a different architectural unit:

```text
package system {
    package sensor {
        package Sensor {
            class SensorDriver
        }
    }
}
```

References to external classes are permitted, but do not satisfy this rule.
For example, a relationship from `FilterService` to
`system.sensor.Sensor.SensorDriver` is an external reference; only
`FilterService` can serve as the required class owned by `Filter`.

### Identifier Matching

Identifiers are compared as canonical identifier paths. The class identifier
must have the architectural unit identifier as a complete path prefix, followed
by at least one additional class-name segment. A bare prefix match is not
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
| The bound static unit-design class diagrams declare no class below the architectural unit identifier | Unit-Class Refinement Consistency |
| The bound static unit-design class diagrams only declare classes below another unit identifier | Unit-Class Refinement Consistency |
| The bound static unit-design class diagrams only reference external classes | Unit-Class Refinement Consistency |

## Debug Output

The validator shall emit debug output containing:

- the architectural unit identifier;
- the Bazel `unit` target and its bound `unit_design` target;
- the class identifiers read from the bound static class diagrams;
- the class identifiers selected as refinements of each architectural unit.
