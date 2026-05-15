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

Define the consistency rules between the indexed Bazel architecture and the
indexed PlantUML component diagram.

This validator checks whether the same SEooC packages, components, and units
exist on both sides after model normalization.

## What is Validated

A dependable element must be defined both in Bazel and in PlantUML. The validator checks:

- top-level dependable element: PlantUML `package <<SEooC>>` vs. Bazel dependable element
- names: PlantUML `alias` when present, otherwise `id` vs. Bazel target name
- components: PlantUML `<<component>>` vs. Bazel component under the expected parent
- units: PlantUML `<<unit>>` vs. Bazel unit under the expected component
- parent context must match
- matching is case-insensitive

In the common case, components nested directly under the dependable element use
the dependable element alias as parent. More deeply nested components use their
immediate enclosing component alias as parent.

## Failure Cases

### Missing in PlantUML

Validation fails when Bazel defines a dependable element, component, or unit
that is not present in the PlantUML diagram.

This includes cases where the name exists in PlantUML but under the wrong
parent, or with the wrong stereotype.

### Extra in PlantUML

Validation fails when PlantUML contains a dependable element, component, or
unit that does not have a corresponding definition in Bazel.

This ensures the diagram does not introduce additional structure beyond what is
declared in the Bazel architecture.

## Debug Output

The validator appends a debug log containing:

- all diagram entities
- filtered entity counts
- all normalized PlantUML keys
- all normalized Bazel keys

## Example

In Bazel BUILD files, the same structure can be declared like this:

```starlark
component(
  name = "component_example",
  components = [
    "//bazel/rules/rules_score/examples/seooc/unit_1:unit_1",
    "//bazel/rules/rules_score/examples/seooc/unit_2:unit_2",
  ],
)

dependable_element(
  name = "safety_software_seooc_example",
  components = [":component_example"],
)

unit(
  name = "unit_1",
)

unit(
  name = "unit_2",
)
```

When exported into the indexed Bazel architecture, these targets produce keys such as:

If the Bazel architecture JSON contains:

- dependable element: `safety_software_seooc_example` -> key: `("safety_software_seooc_example", None)`
- component: `@//bazel/rules/rules_score/examples/seooc:component_example` -> key: `("component_example", Some("safety_software_seooc_example"))`
- unit: `@//bazel/rules/rules_score/examples/seooc/unit_1:unit_1` -> key: `("unit_1", Some("component_example"))`
- unit: `@//bazel/rules/rules_score/examples/seooc/unit_2:unit_2` -> key: `("unit_2", Some("component_example"))`

then PlantUML must contain entities with the same normalized keys:

```plantuml
package "Sample Seooc" as safety_software_seooc_example <<SEooC>> {
    component "Component Example" as component_example <<component>> {
        component "Unit 1" as unit_1 <<unit>>
        component "Unit 2" as unit_2 <<unit>>
    }
}
```
