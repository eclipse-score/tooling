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

It shall also make sure that an interface declared in the component diagram
and specified in the Internal API diagram is represented in the detailed class
design of every unit that requires that interface. The detail-design interface shall
contain every method name specified by the Internal API. Provider units are
not checked by this rule because they do not consume the interface contract.
This check validates the required-side design model; it does not claim that the
provider implements the interface in C++.

## What is Validated

The validator uses four architectural inputs, with the detailed class design
associated through the Bazel `unit` target:

| Input | Source | Meaning |
|---|---|---|
| Architectural unit | `architectural_design.static` component diagram | A `<<unit>>` entity and its canonical identifier |
| Detailed class design | `unit_design.static` class diagram bound to that Bazel `unit` | Class, struct, interface, and abstract-class entities that refine the unit |
| Component interfaces and bindings | `architectural_design.static` component diagram | Interfaces and the units bound to each interface |
| Internal API | `architectural_design.internal_api` class diagram | Interface entities and their method names |

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

### Bound Interface and Method Consistency

For each interface declared in the component diagram and also declared in the
Internal API diagram, the validator shall inspect every unit whose interface
binding role is `Required`. Each such unit's bound static class diagram shall
declare a corresponding interface entity and that detail-design interface
shall contain every method name declared by the Internal API interface.

Units whose binding role is `Provided` are not checked by this rule. The
Internal API and component-interface consistency checks establish the existence
of the shared contract; this check ensures that a consumer's unit design
represents the contract it requires. It does not prove that a provider exists
or implements the interface in C++.

The component interface and Internal API interface are matched by canonical
identifier, including their enclosing package/component context. Matching only
by the interface's final name segment across the diagrams is insufficient.
Within each Required unit's detail class diagram, the corresponding entity
must be an `Interface` declaration with the same canonical interface
identifier as the Internal API interface. The interface remains in the shared package/component
namespace; it is not moved beneath the unit namespace. Each Required unit's
diagram is checked independently, so repeating the same interface declaration
in different unit-design files does not make one unit's declaration satisfy
another Required unit's check. A collaborator stub or an interface declaration
that only appears in another unit's design does not satisfy the check. If
multiple detail interfaces in the same Required unit match the canonical
identifier, the validator shall report an ambiguity.

Method names are compared case-sensitively. This rule does not compare method
parameters, return types, visibility, modifiers, or overload signatures.
Additional methods in the detail-design interface are permitted.

If the component interface is not declared in the Internal API diagram, the
Component Internal API validator reports that issue; this rule does not report
it again. An Internal API interface with no Required unit bindings is outside
this check because there is no consumer unit detail class diagram to inspect.
The absence of a provider or requester role by itself is not an error under
this rule.

For example, the component diagram below binds both `unit_1` and `unit_2` to
`InternalInterface`:

```text
' component diagram
package example <<SEooC>> {
  component service <<component>> {
    component unit_1 <<unit>>
    component unit_2 <<unit>>
    interface InternalInterface
    unit_1 -( InternalInterface
    unit_2 )- InternalInterface
  }
}
```

The Internal API diagram defines the interface method:

```text
' internal API diagram
namespace example {
  namespace service {
    interface InternalInterface <<interface>> {
      {abstract} GetData(): Data*
    }
  }
}
```

Only `unit_1` requires the interface, so only its detail class diagram must
declare the corresponding interface and `GetData()`. For example, this
declaration in `unit_1`'s design satisfies the interface consistency check.
The same diagram must also contain at least one refining entity below the
`unit_1` namespace to satisfy Unit-Class Refinement Consistency:

```text
' unit_1 detail class diagram
namespace example {
  namespace service {
    interface InternalInterface <<interface>> {
      {abstract} GetData(): Data*
    }
    namespace unit_1 {
      class Unit1Service
    }
  }
}
```

`unit_2` provides the interface, so this rule does not require its detail
class diagram to redeclare the shared contract. If `unit_1`'s design omits
`InternalInterface` or declares it without `GetData()`, validation fails for
`unit_1`. A provider implementation check is outside the scope of this rule.

*(Requirement: {requirement:downstream-ref}`Tools.ArchitecturalUnitInternalApiClassDesignConsistency`)*

### Not Validated Here

This validator does not validate:

- the correspondence between architectural units and Bazel `unit` targets;
  that is covered by the Bazel Component validator.
- component-diagram interface declarations against public or internal API
  diagrams; that is covered by the Component Public API and Component Internal
  API validators.
- method signatures, modifiers, or class-design entities against C++
  implementation code; that is covered by the Class Design Implementation
  validator. This specification checks only the presence of the bound
  interface and Internal API method names in the unit's detail class design.
- whether external class references are allowed architectural dependencies.
- whether the Provider exists or implements the interface; this rule validates
  only the Required-side unit-design contract.

## Failure Cases

| Failure case | Validation rule |
|---|---|
| The bound static unit-design class diagrams declare no eligible entity below the full architectural unit ID | Unit-Class Refinement Consistency |
| The bound static unit-design class diagrams only declare eligible entities below another architectural unit ID | Unit-Class Refinement Consistency |
| The bound static unit-design class diagrams only reference external entities | Unit-Class Refinement Consistency |
| A unit with a Required binding to a component interface has no corresponding interface entity in its own detail class design | Bound Interface and Method Consistency |
| A Required unit's corresponding detail-design interface omits one or more Internal API method names | Bound Interface and Method Consistency |
| A Required unit's matching interface exists only in another unit's class design or as a collaborator stub | Bound Interface and Method Consistency |
| Multiple detail interfaces in one Required unit ambiguously match the component interface | Bound Interface and Method Consistency |

## Debug Output

The validator shall emit debug output containing:

- the architectural unit identifier;
- the Bazel `unit` target and its bound `unit_design` target;
- the eligible entity identifiers read from the bound static class diagrams;
- the eligible entity identifiers selected as refinements of each architectural unit.
