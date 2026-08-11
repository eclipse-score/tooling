<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Component Internal API Specification

## Purpose

This validator enforces consistency between two diagram types:

- **Component diagrams**
- **Internal API diagrams**

It shall make sure that every interface declared by the component design is
also declared by the internal API design.

## What is Validated

All comparisons are case-sensitive.

### Interface Declaration Consistency

Every interface declared in the component diagram must resolve to an interface
declared in the internal API diagram.
*(Requirement: {requirement:downstream-ref}`Tools.ComponentInternalApiInterfaceDeclarationConsistency`)*

Only interfaces that are nested inside a parent component or package are
considered. An interface declared at the diagram's root level (no enclosing
component/package) has no parent and is silently ignored by this validator.

```text
' PlantUML component diagram
component "Unit 1" as unit_1 <<unit>>
package "Package A" as package_a {
  interface "IData" as IData
}
unit_1 -( IData
```

```text
' PlantUML internal_api diagram
package "Package A" as package_a {
  interface "IData" as IData <<interface>> {
    {abstract} GetData(): Data*
  }
}
```

The component interface is matched against the internal API interface ID. The
ID includes the parent qualifier, so `IData` nested under `package_a` is
matched as `package_a.IData`, not just `IData`. The match is exact and
case-sensitive. This check applies even when a component interface is not
referenced by a unit relation.

When multiple interfaces are missing, they are all reported together in a
single failure message; an interface referenced by more than one unit is
reported only once. The message includes, for each missing interface, the
component diagram's source file and line:

```text
[Interface] component interface(s) "package_a.IData" from the component diagram not found in the internal API diagram
  missing interfaces                              : "package_a.IData"
  component source file for "package_a.IData"     : "component_diagram.puml"
  component source line for "package_a.IData"     : 22
  Fix                                              : add interface declaration(s) "package_a.IData" in the internal API diagram, or remove those interface declarations from the component diagram
```

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Missing internal API interface | Interface Declaration Consistency |

## Debug Output

The validator emits debug output containing:

- component interfaces checked against the internal API
- internal API interfaces available for component interfaces
