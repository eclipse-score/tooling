<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Bazel Component Specification

## Purpose

This validator enforces consistency between the implemented Bazel SW architecture and
the targetted architecture in PlantUML component diagram.

It shall make sure that the same architectural elements exist on both sides and are related in the same way.

## What is Validated

All comparisons are case-insensitive. Names are derived from the PlantUML
`alias` when present, otherwise from the `id`. On the Bazel IDs are generated
from the target name.

### Dependable Element Consistency

Every PlantUML `package <<SEooC>>` must have a corresponding Bazel
`dependable_element` target, and vice versa.
*(Requirement: {requirement:downstream-ref}`Tools.BazelComponentDependableElementConsistency`)*

```starlark
dependable_element(
  name = "safety_software_seooc_example",
  components = [":component_example"],
)
```

```text
package "Sample Seooc" as safety_software_seooc_example <<SEooC>> {
}
```

### Component Consistency

Every PlantUML `<<component>>` must have a corresponding Bazel `component`
target under the same parent dependable element, and vice versa.
*(Requirement: {requirement:downstream-ref}`Tools.BazelComponentComponentConsistency`)*

```starlark
component(
  name = "component_example",
  components = [
    "//bazel/rules/rules_score/examples/seooc/unit_1:unit_1",
    "//bazel/rules/rules_score/examples/seooc/unit_2:unit_2",
  ],
)
```

```text
package "Sample Seooc" as safety_software_seooc_example <<SEooC>> {
    component "Component Example" as component_example <<component>> {
    }
}
```

### Unit Consistency

Every PlantUML `<<unit>>` must have a corresponding Bazel `unit` target under
the same parent component, and vice versa.
*(Requirement: {requirement:downstream-ref}`Tools.BazelComponentUnitConsistency`)*

```starlark
unit(
  name = "unit_1",
)

unit(
  name = "unit_2",
)
```

```text
component "Component Example" as component_example <<component>> {
    component "Unit 1" as unit_1 <<unit>>
    component "Unit 2" as unit_2 <<unit>>
}
```

### Parent Context

In the common case, components nested directly under the dependable element use
the dependable element alias as parent. More deeply nested components use their
immediate enclosing component alias as parent.
*(Requirements: {requirement:downstream-ref}`Tools.BazelComponentNameCaseInsensitive`, {requirement:downstream-ref}`Tools.BazelComponentParentContext`)*

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Missing dependable element in PlantUML | Dependable Element Consistency |
| Extra dependable element in PlantUML | Dependable Element Consistency |
| Missing component in PlantUML | Component Consistency |
| Extra component in PlantUML | Component Consistency |
| Missing unit in PlantUML | Unit Consistency |
| Extra unit in PlantUML | Unit Consistency |

## Debug Output

The validator emits debug output containing:

- all diagram entities
- filtered entity counts
- all normalized PlantUML keys
- all normalized Bazel keys
