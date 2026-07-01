<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Class Design Implementation Specification

## Purpose

This validator enforces consistency between the class model in the unit design
and the class model extracted from the implementation.

It shall make sure that every class, member, method, enum literal, type alias,
and relationship modeled in the unit design exists in the implementation and is
compatible with it.

Implementation-only entities and members are allowed. The unit design is the
validated contract.

## What is Validated

The validator compares unit design class diagrams with implementation class
diagrams produced by the C++ parser. It receives two indexed class-diagram
inputs:

| Input | Source | Meaning |
|---|---|---|
| `design_classes` | `unit_design` class diagram FlatBuffers | Required class model from the unit design |
| `implementation_classes` | C++ parser class diagram FlatBuffers from `unit` implementation targets | Observed class model from implementation code |

### Entity Presence and Type Consistency

Every entity in the unit design class diagram must have a corresponding entity
in the implementation class diagram. Matching entities must have the same entity
type.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationEntityConsistency`)*

This item covers entity matching and entity type only. The consistency of data
contained by a matched entity is covered by the following type alias, variable,
method, enum literal, relationship, and template parameter items.

```text
' unit design class diagram
package vehicle {
  class "Transport"
}

' implementation C++ code
namespace vehicle {
  class Transport {}
}
```

### Template Parameter Consistency

Template parameters modeled on unit design entities and methods must match the
template parameters on the corresponding implementation entities and methods.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationTemplateParameterConsistency`)*

```text
' unit design class diagram
class Repository<T> {
  + FindById<Key>(id: Key) : T
}

' implementation C++ code
template <typename T>
class Repository {
  template <typename Key>
  T FindById(Key id);
}
```

### Type Alias Consistency

Every type alias modeled in the unit design entity must exist in the matching
implementation entity. The alias name and normalized original type must match.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationTypeAliasConsistency`)*

```text
' unit design class diagram
class "Transport" {
  using Payload = std::uint8_t
}

' implementation C++ code
class Transport {
public:
  using Payload = std::uint8_t;
};
```

### Variable Consistency

Every variable modeled in the unit design entity must exist in the matching
implementation entity. The variable name, normalized data type, visibility, and
static flag must match.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationVariableConsistency`)*

```text
' unit design class diagram
class "Transport" {
  - buffer: std::uint8_t*
  {static} - instance_count: uint32_t
}

' implementation C++ code
class Transport {
private:
  std::uint8_t* buffer;
  static std::uint32_t instance_count;
};
```

### Method Consistency

Every method modeled in the unit design entity must exist in the matching
implementation entity. The method name, visibility, parameters, and method
modifiers must match. For non-constructor/non-destructor methods, normalized
return type must also match. Method modifiers include `static`, `virtual`,
`abstract`, `override`, constructor, destructor, and `noexcept`.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationMethodConsistency`)*

Method lookup first tries the full normalized signature. If no exact signature
match exists and exactly one implementation method has the same name, that
method is compared to produce field-level diagnostics. If multiple same-named
implementation overloads exist, the design method is reported as missing because
the validator cannot safely choose a candidate.

```text
' unit design class diagram
class "Transport" {
  + Dispatch(mode: std::uint8_t, payload: vehicle::Payload): bool
}

' implementation C++ code
bool Transport::Dispatch(std::uint8_t mode, vehicle::Payload payload)
```

### Enum Literal Consistency

Every enum literal modeled in the unit design enum must exist in the matching
implementation enum. The full literal data must match.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationEnumLiteralConsistency`)*

```text
' unit design class diagram
enum "Mode" {
  Startup
  Shutdown
}

' implementation C++ code
enum Mode {
  Startup,
  Shutdown,
};
```

### Relationship Consistency

Every relationship modeled on a unit design entity must exist on the matching
implementation entity. Source, target, and relationship type must match.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationRelationshipConsistency`)*

```text
' unit design class diagram
struct Payload {}
class Vehicle {
  + Load(payload : Payload)
}
Vehicle ..> Payload

' implementation C++ code
struct Payload {};
class Vehicle {
public:
  void Load(Payload payload);
};
```

### Type Normalization

Before comparing types, the validator applies limited normalization.
*(Requirement: {requirement:downstream-ref}`Tools.ClassDesignImplementationTypeNormalization`)*

| Input form | Normalized form |
|---|---|
| `std::uint8_t` | `uint8_t` |
| `uint8_t *` | `uint8_t*` |
| `Payload &` | `Payload&` |

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Missing implementation entity | Entity Presence and Type Consistency |
| Entity type mismatch | Entity Presence and Type Consistency |
| Entity template parameter mismatch | Template Parameter Consistency |
| Missing type alias | Type Alias Consistency |
| Type alias original type mismatch | Type Alias Consistency |
| Missing variable | Variable Consistency |
| Variable type, visibility, or static flag mismatch | Variable Consistency |
| Missing method | Method Consistency |
| Method return type, visibility, parameter, or modifier mismatch | Method Consistency |
| Method template parameter mismatch | Template Parameter Consistency |
| Missing enum literal | Enum Literal Consistency |
| Enum literal data mismatch | Enum Literal Consistency |
| Missing relationship | Relationship Consistency |
| Relationship source, target, or type mismatch | Relationship Consistency |

## Debug Output

The validator emits debug output containing:

- the design entity ID being compared
- the implementation entity ID being compared
- detailed design entity snapshots at trace level
- detailed implementation entity snapshots at trace level
- type aliases, variables, methods, template parameters, enum literals, and
  relationships for each matched entity

Failure messages contain source file and source line information when available.
Missing or empty source metadata is rendered as `<unknown>`.
