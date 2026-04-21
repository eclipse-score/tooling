<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Architecture Verifier (archver)

Validates that PlantUML component diagrams match the Bazel build graph structure.

## Overview

The archver tool ensures architectural consistency by comparing:
- **Bazel build graph**: Dependable element, component and unit hierarchy from the build system
- **PlantUML diagrams**: Static architecture documentation with stereotypes

## Usage

### Standalone CLI

```bash
archver \
  --architecture-json path/to/architecture.json \
  --static-fbs path/to/diagram1.fbs.bin path/to/diagram2.fbs.bin

# Write a debug log in addition to stderr output
archver \
  --architecture-json path/to/architecture.json \
  --static-fbs path/to/diagram.fbs.bin \
  --output path/to/archver.log
```

Exits with code `0` on success, `1` on any validation error or I/O failure.

## What is Validated

### Dependable Element
The dependable element is represented in the Bazel JSON as a top-level component
entry. In PlantUML, it corresponds to a `package` with stereotype `<<SEooC>>`.
The alias of the PlantUML package must match the Bazel dependable element name.

### Component Validation (Check 1)
For each unique `(component_alias, parent_alias)` combination that Bazel expects:
- PlantUML must have the exact same number of components with that combination
- Uses target name only (package path is stripped)
- Components nested under the dependable element have the dependable element as parent
- Entities must have stereotype `<<component>>`

### Unit Validation (Check 2)
For each unique `(unit_alias, parent_component_alias)` combination that Bazel expects:
- PlantUML must have the exact same number of units with that combination
- Uses target name only (e.g., `unit_1`, `unit_2`)
- Entities must have stereotype `<<unit>>`

### Duplicate Detection
If two Bazel targets resolve to the same `(alias, parent)` key (e.g., same target
name in different packages), or two PlantUML entities have the same key, an error
is emitted.

### Example
If Bazel build graph has:
- Dependable element: `safety_software_seooc_example` → key: `safety_software_seooc_example`
- Component: `@//bazel/rules/rules_score/examples/seooc:component_example` → key: `component_example` (parent: `safety_software_seooc_example`)
- Unit: `@//bazel/rules/rules_score/examples/seooc/unit_1:unit_1` → key: `unit_1` (parent: `component_example`)
- Unit: `@//bazel/rules/rules_score/examples/seooc/unit_2:unit_2` → key: `unit_2` (parent: `component_example`)

PlantUML must have:
```plantuml
package "Safety Software SEooC Example" as safety_software_seooc_example <<SEooC>> {
    component "ComponentExample" as component_example <<component>> {
        component "Unit 1" as unit_1 <<unit>>
        component "Unit 2" as unit_2 <<unit>>
    }
}
```
