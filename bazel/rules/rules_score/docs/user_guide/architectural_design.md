<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

<!--
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
-->

# Architectural Design

## Overview and Hierarchy

Software in `rules_score` is structured in three levels:

```
dependable_element   (SEooC — complete Safety Element out of Context)
└── component        (groups units; owns component-level integration tests and requirements)
    ├── unit         (smallest independently verifiable architectural element: implementation + unit tests)
    └── component    (components can be nested for deeper hierarchies)
        └── unit
```

Two rules apply:

- `unit` targets must always be wrapped in a `component` — they cannot be placed directly under `dependable_element`.
- `component` targets can be nested: a component may contain other components as well as units, allowing arbitrary depth.

This hierarchy exists for two complementary reasons:

- **Interrelations between units and components** — A component defines a clear boundary within which its units collaborate. Grouping units into components keeps inter-unit coupling explicit and local, while the component's public interface controls what the rest of the system can depend on. The static architecture diagrams document exactly which components expose which interfaces, preventing accidental cross-boundary dependencies.
- **Interface-driven safety analysis** — The `public_api` diagrams at the SEooC level define the interfaces that external consumers may call. Failure modes in the FMEA reference individual interface items by name, establishing a direct traceability link from the safety analysis back to the architecture. Without a well-defined interface boundary this traceability would be impossible.

Each level of the hierarchy has a corresponding design artifact:

| Level | Design rule | Content |
|---|---|---|
| SEooC / component | `architectural_design` | Static structure, dynamic behaviour, public API |
| Unit | `unit_design` | Implementation Details, unit-level sequences |

A consistency check at build time verifies that every component and unit in the Bazel implementation tree also appears in the PlantUML target architecture diagrams. If anything is out of sync the build fails with a descriptive error.


## Static Architecture

The static view describes the **structural organisation** of your software: what components and units exist, how they relate to each other, and which dependencies they carry. It is the primary input for the architecture consistency check.

### PlantUML

Write a PlantUML class or component diagram that names every `component` and `unit` from your Bazel BUILD file.

```{uml} ../_assets/MySeooc_StaticDesign.puml
:align: center
:alt: MySeooc static architecture
```

```text
@startuml MySeooc_StaticDesign

package "MySeooc" as MySeooc <<SEooC>> {
    component "KvsComponent" as KvsComponent <<component>> {
        component "KeyValueStore" as KeyValueStore <<unit>>
        component "StorageBackend" as StorageBackend <<unit>>
    }
}

@enduml
```

### Bazel

The PlantUML diagrams capture *intended* structure; the Bazel rules model the *actual* implementation. Using the same example as the diagram above — SEooC `MySeooc` containing component `KvsComponent` with units `KeyValueStore` and `StorageBackend` — the three rules work together like this:

#### architectural_design

declares which diagram files belong to which view category:

```starlark
load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "architectural_design")

architectural_design(
    name   = "my_arch",
    static = ["static_design.puml"],  # the MySeooc_StaticDesign diagram above
)
```

#### unit

one target per leaf unit (`<<unit>>` stereotype) in the diagram. The unit name must match the name used in the PlantUML. It ties together implementation targets, test targets, and an optional `unit_design` target (see {doc}`unit_design`):

```starlark
load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "unit")

# Unit for KeyValueStore
cc_library(name = "kvs_lib",       srcs = ["kvs.cpp"],      hdrs = ["kvs.h"])
cc_test   (name = "kvs_unit_test", srcs = ["kvs_test.cpp"], deps = [":kvs_lib"])

unit(
    name           = "KeyValueStore",
    unit_design    = [":kvs_unit_design"],
    implementation = [":kvs_lib"],
    tests          = [":kvs_unit_test"],
)

# Unit for StorageBackend
cc_library(name = "storage_lib",       srcs = ["storage_backend.cpp"], hdrs = ["storage_backend.h"])
cc_test   (name = "storage_unit_test", srcs = ["storage_test.cpp"],   deps = [":storage_lib"])

unit(
    name           = "StorageBackend",
    unit_design    = [":storage_unit_design"],
    implementation = [":storage_lib"],
    tests          = [":storage_unit_test"],
)
```


#### component

groups the units that belong to `KvsComponent` in the diagram. It aggregates one or more `unit` (or nested `component`) targets and links them to component-level requirements. Integration tests that verify the units working together are declared here:

```starlark
load("@score_tooling//bazel/rules/rules_score:rules_score.bzl",
     "component", "component_requirements")

component_requirements(
    name = "kvs_comp_req",
    srcs = ["component_requirements.trlc"],
    deps = [":feature_req"],
)

# The component maps to KvsComponent in the PlantUML diagram
component(
    name         = "KvsComponent",
    requirements = [":kvs_comp_req"],
    components   = [":KeyValueStore", ":StorageBackend"],
    tests        = [],
)
```

## Dynamic Architecture

The dynamic view describes **behavioural aspects** — sequences of interactions, state transitions, and activity flows. Dynamic diagrams document how your software behaves at runtime. They are not validated against the Bazel structure at build time.

### PlantUML

```{uml} ../_assets/MySeooc_WriteSequence.puml
:align: center
:alt: MySeooc write sequence
```

```text
@startuml MySeooc_WriteSequence

actor Caller
participant KeyValueStore
participant StorageBackend

Caller -> KeyValueStore : write(key, value)
KeyValueStore -> StorageBackend : flush()
StorageBackend --> KeyValueStore : OK
KeyValueStore --> Caller : Result::Ok

@enduml
```

### Bazel

```starlark
architectural_design(
    name    = "my_arch",
    static  = ["static_design.puml"],
    dynamic = ["sequence.puml"],
)
```

## Public API

The public API view describes the **interface your SEooC exposes to its environment**. These diagrams are linked to safety analysis: `FailureMode` records reference interface items by name (via the `interface` field), enabling traceability from each failure mode back to the architecture.

### PlantUML

```{uml} ../_assets/MySeooc_PublicApi.puml
:align: center
:alt: MySeooc public API
```

```text
@startuml MySeooc_PublicApi

interface "KeyValueStore" as KVS {
    + write(key: string, value: bytes): Result
    + read(key: string): Optional<bytes>
}

@enduml
```

### Bazel

```starlark
architectural_design(
    name       = "my_arch",
    public_api = ["public_api.puml"],
)
```

The `public_api` attribute also generates traceability items that can be referenced by `fmea` targets (see {doc}`dependability_analysis`) via the `arch_design` attribute.

(rst-and-markdown-wrappers)=
## RST and Markdown Wrappers

When you want to combine a diagram with text, create an RST or Markdown file that embeds the diagram using the `.. uml::` directive (RST) or the MyST equivalent.

**RST wrapper example:**

```rst
Static Architecture
-------------------

The following diagram shows the component structure of MySeooc.

.. uml:: MySeooc_StaticDesign.puml
```

Include both the wrapper file *and* the referenced `.puml` file in the same Bazel list — the build needs both:

```starlark
architectural_design(
    name   = "my_arch",
    static = [
        "static_design.rst",          # wrapper with prose
        "MySeooc_StaticDesign.puml",  # diagram referenced by the wrapper
    ],
)
```

## Rule Reference: `architectural_design`

For the complete `architectural_design` attribute reference, see {ref}`architectural_design <rule-architectural-design>` in the rule index.
