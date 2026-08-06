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

All comparisons are case-insensitive: both Bazel target short names and
PlantUML aliases/IDs are normalized to lowercase before matching, so a Bazel
target `Component_X` matches a PlantUML entity `as COMPONENT_X`. Names are
derived from the PlantUML `alias` when present, otherwise from the `id`. On
the Bazel side, IDs are generated from the target's short name (the part
after the last `:` in the label), also lowercased. Parent aliases are
lowercased the same way when resolving parent-child relationships.

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

### Bazel Label Format Validation

Every Bazel label must define a target name. A label such as `@//pkg:` (a
colon with nothing following it) has no target name and is rejected with a
`[Design]` error before any entity matching happens.

### Duplicate Entity Detection

The validator detects when two entities of the same kind normalize to the same
key (same lowercased name under the same parent):

- **In Bazel:** if two different Bazel targets (e.g. under different
  packages) would both map to the same dependable-element, component, or unit
  key, a `[Design]` error is reported naming both Bazel labels.
- **In the PlantUML diagram:** entity IDs are matched case-insensitively, so
  two entities such as `MyDE` and `myDE` collide. A `[Design]` error is
  reported showing both source file locations, even if their stereotypes
  differ.

### Parent Reference Validity

Every PlantUML entity's `parent_id` (if present) must resolve to another
entity defined in the same component diagram. An entity referencing an
undefined parent is rejected with a `[Design]` error naming the missing
parent ID.

## Failure Cases

| Failure case | Validation rule |
|---|---|
| Missing dependable element in PlantUML | Dependable Element Consistency |
| Extra dependable element in PlantUML | Dependable Element Consistency |
| Missing component in PlantUML | Component Consistency |
| Extra component in PlantUML | Component Consistency |
| Missing unit in PlantUML | Unit Consistency |
| Extra unit in PlantUML | Unit Consistency |
| Invalid Bazel label (no target name after `:`) | Bazel Label Format Validation |
| Duplicate Bazel entity key (same name/parent from different labels) | Duplicate Entity Detection |
| Duplicate PlantUML entity ID (case-insensitive collision) | Duplicate Entity Detection |
| PlantUML entity references an undefined parent | Parent Reference Validity |

## PlantUML Stereotype Reference

The validator identifies elements by their **stereotype**, not by the PlantUML keyword. Both `package` and `component` keywords are accepted for each role, but the stereotype must match exactly: a Bazel `component` target will not match a PlantUML entity marked `<<SEooC>>`, even though both use compatible keywords.

| Stereotype | Valid PlantUML keywords | Meaning | Bazel rule |
|---|---|---|---|
| `<<SEooC>>` | `package`, `component` | Safety Element out of Context boundary | `dependable_element` |
| `<<component>>` | `component`, `package` | Architectural component | `component` |
| `<<unit>>` | `component`, `package` | Leaf implementation unit | `unit` |

Port declarations and interface bindings (`portin`/`portout`, `-(`/`)-`) are
not validated by this validator; they are checked by the internal/public API
and sequence validators instead.

## Debug Output

The validator emits debug output containing:

- all diagram entities
- filtered entity counts
- all normalized PlantUML keys
- all normalized Bazel keys
