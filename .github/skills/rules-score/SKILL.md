<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

---
name: rules-score
description: "Central entry point for building a Safety Element out of Context (SEooC) with the rules_score Bazel rules. USE FOR: creating or assembling a dependable_element end to end, understanding the full requirements → architecture → units → tests → safety-analysis workflow, wiring all rules_score targets together, and deciding which specialized skill to use for each work product. Delegates to score-requirements, score-architecture, score-testing, and score-safety-analysis. Use when scaffolding a new SEooC, adding a component/unit across all layers, or coordinating multi-layer changes."
argument-hint: "the SEooC / dependable element to build or extend"
---

# rules_score — SEooC Orchestration Skill

The central skill for building a **Safety Element out of Context (SEooC)** with the `rules_score`
Bazel rules. It gives the end-to-end picture and the assembly recipe for a `dependable_element`,
then **delegates the details of each work product to a specialized skill**.

> **Source of truth**: the rule macros under `bazel/rules/rules_score/private/`, the aggregator
> [`bazel/rules/rules_score/rules_score.bzl`](../../../bazel/rules/rules_score/rules_score.bzl), the
> validator specs in [`validation/core/docs/specifications/`](../../../validation/core/docs/specifications),
> and the runnable examples in [`bazel/rules/rules_score/examples/`](../../../bazel/rules/rules_score/examples)
> (`minimal/` and `seooc/`). When source and documentation disagree, the source wins.

---

## Delegation Map

Use this skill to coordinate; open the specialized skill for the actual work:

| You are working on… | Use skill |
|---------------------|-----------|
| `.trlc` requirement records, `ScoreReq` model, traceability, `assumed_system_requirements` / `feature_requirements` / `component_requirements` / `assumptions_of_use` | **score-requirements** |
| PlantUML diagrams, `architectural_design` / `unit` / `unit_design` / `component` / `dependable_element` structure, architecture/API/sequence validations | **score-architecture** |
| GoogleTest `lobster-tracing` + Given-When-Then, `test_case_coverage.lock.yaml`, attaching tests | **score-testing** |
| FMEA, `FailureMode` / `ControlMeasure` / FTA, `fmea` / `dependability_analysis` | **score-safety-analysis** |

---

## The Dependable Element

A `dependable_element` is the top-level SEooC entity. It aggregates every work product and, at
build time, verifies their mutual consistency and assembles them into Sphinx HTML documentation
with a traceability report.

| Work product | Rule(s) | Skill |
|--------------|---------|-------|
| Assumed System Requirements | `assumed_system_requirements` | score-requirements |
| Feature Requirements | `feature_requirements` | score-requirements |
| Component Requirements | `component_requirements` | score-requirements |
| Assumptions of Use | `assumptions_of_use` | score-requirements |
| Architectural Design | `architectural_design` | score-architecture |
| Units & Components | `unit`, `unit_design`, `component` | score-architecture |
| Tests & Coverage | `tests` attr, `test_case_coverage_lock` | score-testing |
| Dependability Analysis | `fmea`, `dependability_analysis` | score-safety-analysis |
| SEooC assembly | `dependable_element` | this skill |

### Hierarchy

```
dependable_element   (SEooC boundary)
└── component        (groups units; owns component requirements + integration tests)
    ├── unit         (implementation + unit design + unit tests)
    └── component    (nestable to arbitrary depth)
        └── unit
```

A `unit` must always sit inside a `component`; a `component` may contain units and/or other
components.

---

## End-to-End Recipe

All rules load from one aggregator:

```starlark
load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "architectural_design",
    "assumed_system_requirements",
    "component",
    "dependable_element",
    "feature_requirements",
    "unit",
    "unit_design",
)
```

The smallest complete SEooC, wired end-to-end in a single BUILD file (verbatim from
[`examples/minimal/BUILD`](../../../bazel/rules/rules_score/examples/minimal/BUILD)):

```starlark
assumed_system_requirements(
    name = "assumed_system_requirements",
    srcs = ["requirements/asr.trlc"],
    visibility = ["//visibility:public"],
)

feature_requirements(
    name = "feature_requirements",
    srcs = ["requirements/feature_requirements.trlc"],
    visibility = ["//visibility:public"],
    deps = [":assumed_system_requirements"],
)

architectural_design(
    name = "my_arch",
    static = ["docs/static_design.puml"],
)

cc_library(
    name = "my_unit_lib",
    srcs = ["src/my_unit.cpp"],
    hdrs = ["src/my_unit.h"],
)

cc_test(
    name = "my_unit_test",
    srcs = ["test/my_unit_test.cpp"],
    deps = [":my_unit_lib", "@googletest//:gtest_main"],
)

unit_design(
    name = "MyUnit_design",
    static = ["docs/class_design.puml"],
)

unit(
    name = "MyUnit",
    scope = ["//:my_unit_lib"],
    tests = [":my_unit_test"],
    unit_design = [":MyUnit_design"],
    implementation = [":my_unit_lib"],
)

component(
    name = "MyComponent",
    components = [":MyUnit"],
    requirements = [],
    tests = [],
)

dependable_element(
    name = "my_element",
    architectural_design = [":my_arch"],
    assumptions_of_use = [],
    components = [":MyComponent"],
    dependability_analysis = [],
    integrity_level = "B",
    requirements = [":feature_requirements"],
    tests = [],
)
```

Note how the diagram alias must equal the target name — `docs/static_design.puml` declares
`package "my_element" ... { component "MyComponent" { component "MyUnit" } }`, matching
`dependable_element(name = "my_element")`, `component(name = "MyComponent")`, and
`unit(name = "MyUnit")`. This is enforced by the `bazel_component` validator (see
**score-architecture**).

For a fuller SEooC — nested components, public/internal API, sequence diagrams, AoUs, glossary,
FMEA, cross-module `deps`, and test-case coverage — see
[`examples/seooc/`](../../../bazel/rules/rules_score/examples/seooc).

---

## Suggested Order of Work

1. **Requirements** → write `AssumedSystemReq`, `FeatReq`, then `CompReq`; wire the three rules with
   `deps` chaining. *(score-requirements)*
2. **Architecture** → draw the `static` PlantUML tree and add `dynamic`/`public_api`/`internal_api`
   as needed; create `unit`, `unit_design`, and `component` targets whose names match the diagram.
   *(score-architecture)*
3. **Implementation & tests** → back each `unit` with a `cc_library` + `cc_test`; annotate tests
   with `lobster-tracing` + Given-When-Then; add `test_case_coverage_lock` on components.
   *(score-testing)*
4. **Safety analysis** → add `fmea` (FailureMode + ControlMeasure + FTA) and wrap it in a
   `dependability_analysis`. *(score-safety-analysis)*
5. **Assemble** → allocate `CompReq` to `component(requirements=…)` and `FeatReq` to
   `dependable_element(requirements=…)`; wire `architectural_design`, `components`,
   `assumptions_of_use`, `dependability_analysis`, `glossary`, and `integrity_level`.

---

## Build, Test, Validate

```bash
# Build the SEooC docs + run every consistency validation
bazel build //path/to:my_element

# Run all tests: unit/integration tests, trlc --verify, coverage-drift checks
bazel test //...

# Refresh a component's coverage lock after intended test changes
bazel run //path/to:MyComponent.update
```

`dependable_element(maturity = "development")` downgrades scope- and coverage-violations to
warnings while a design is in progress; switch to `"release"` before certification.

---

## Key Cross-Cutting Facts

- **ASIL vs. integrity_level**: requirement records use `ScoreReq.Asil` = `QM`/`B`/`D` only;
  `dependable_element.integrity_level` uses `A`/`B`/`C`/`D` (D > C > B > A) for the element
  hierarchy.
- **Naming**: diagram aliases must equal Bazel target names. `bazel_component` is case-insensitive;
  all other validators are case-sensitive.
- **Traceability spine**: `AssumedSystemReq → FeatReq → CompReq` (version-pinned `@`) →
  `lobster-tracing` in tests → `test_case_coverage.lock.yaml`; `FailureMode.interface` links safety
  analysis to the `public_api` diagram.

---

## References

- [`examples/minimal/`](../../../bazel/rules/rules_score/examples/minimal) — smallest end-to-end SEooC
- [`examples/seooc/`](../../../bazel/rules/rules_score/examples/seooc) — full-featured SEooC
- [`rules_score.bzl`](../../../bazel/rules/rules_score/rules_score.bzl) — the rule aggregator (all public symbols)
- [`docs/user_guide/general.rst`](../../../bazel/rules/rules_score/docs/user_guide/general.rst) — dependable-element concept + validations
- [`validation/core/docs/specifications/`](../../../validation/core/docs/specifications) — normative validator specs
