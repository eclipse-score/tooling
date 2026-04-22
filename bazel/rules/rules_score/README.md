<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Rules Score

Starlark rules implementing the **S-CORE** functional-safety development process
for safety related automotive software.

## Rules Overview

![Rules integration overview](docs/rules_score_overview.svg)

| Rule | Providers emitted |
|------|-------------------|
| `feature_requirements` | `FeatureRequirementsInfo` |
| `component_requirements` | `ComponentRequirementsInfo` |
| `assumptions_of_use` | `AssumptionsOfUseInfo` |
| `architectural_design` | `ArchitecturalDesignInfo` |
| `unit` | `UnitInfo`, `CertifiedScope` |
| `component` | `ComponentInfo` |
| `fmea` | `AnalysisInfo` |
| `dependability_analysis` | `DependabilityAnalysisInfo` |
| `dependable_element` | HTML documentation zip (Sphinx) |

All rules also emit `SphinxSourcesInfo` for the documentation assembly pipeline.

---

## `feature_requirements` / `component_requirements`

```starlark
load("@trlc//:trlc.bzl", "trlc_requirements")
load("//bazel/rules/rules_score:rules_score.bzl", "feature_requirements")

trlc_requirements(
    name = "my_trlc_reqs",
    srcs = ["requirements.trlc"],
    spec = ["@score_tooling//bazel/rules/rules_score/trlc/config:score_requirements_model"],
)

feature_requirements(
    name = "my_feat_reqs",
    srcs = [":my_trlc_reqs"],
)
```

**`bazel build`** — collects `TrlcProviderInfo` from the underlying
`trlc_requirements` targets and produces `.lobster` files for LOBSTER.
Also generates a `_test` target that validates metamodel compliance.

---

## `assumptions_of_use`

```starlark
assumptions_of_use(
    name = "my_aou",
    requirements = [":my_feat_reqs"],
    srcs = ["assumptions.rst"],
)
```

**`bazel build`** — collects LOBSTER traceability files from the linked
requirements and exposes them via `AssumptionsOfUseInfo`.

---

## `architectural_design`

```starlark
architectural_design(
    name = "my_design",
    static = ["class.puml", "component.puml"],
    dynamic = ["sequence.puml"],
    public_api = ["api.puml"],
)
```

**`bazel build`** — runs `puml_parser` on every `.puml` file, producing:
- a `.fbs.bin` FlatBuffers binary (diagram AST) — consumed by archver validation
- a `.lobster` traceability file (Interface elements only) — consumed by LOBSTER
- a `plantuml_links.json` — consumed by the `clickable_plantuml` Sphinx extension

Diagrams in `public_api` are classified separately so their lobster items flow
through `public_api_lobster_files` for failure-mode traceability.

---

## `unit`

```starlark
unit(
    name = "my_unit",
    unit_design = [":my_design"],
    implementation = [":my_lib"],
    tests = [":my_tests"],
)
```

**`bazel build`** — wraps implementation targets and collects design + test
references into `UnitInfo`. Also runs unit tests via `gtest_report` and
produces `.lobster` traceability items for LOBSTER.

**`bazel test`** — executes the wrapped test targets.

---

## `component`

```starlark
component(
    name = "my_component",
    requirements = [":my_comp_reqs"],
    components = [":unit_a", ":unit_b"],
    tests = [],
)
```

**`bazel build`** — aggregates `UnitInfo` / nested `ComponentInfo` providers
and collects requirement + architecture + test lobster sources.

**`bazel test`** — runs component-level integration tests passed via `tests`.

---

## `fmea`

```starlark
fmea(
    name = "my_fmea",
    failuremodes = [":failure_modes"],
    controlmeasures = [":control_measures"],
    root_causes = ["fta.puml"],
    arch_design = ":my_design",
)
```

**`bazel build`** — generates `fmea.rst` (merged FM / CM / FTA sections),
runs `lobster-trlc` on TRLC inputs, and extracts FTA events from `.puml`
diagrams into `fta.lobster`. Build-only; traceability validation is done
by the wrapping `dependability_analysis` test.

---

## `dependability_analysis`

```starlark
dependability_analysis(
    name = "my_da",
    fmea = [":my_fmea"],
    arch_design = ":my_design",
)
```

**`bazel build`** — collects `.lobster` files from all sub-analyses and
architectural design, expands the `lobster_sa.conf` template, and runs
`lobster-ci-report` to produce a traceability report JSON + HTML.

**`bazel test`** — asserts that `lobster-ci-report` exits with code 0
(all traceability links are satisfied). This is the primary
**safety-analysis traceability gate**.

```bash
bazel test //examples/seooc:sample_dependability_analysis
```

---

## `dependable_element`

```starlark
dependable_element(
    name = "my_seooc",
    description = "My safety element",
    integrity_level = "B",
    requirements = [":feat_reqs"],
    architectural_design = [":my_design"],
    dependability_analysis = [":my_da"],
    components = [":my_component"],
    assumptions_of_use = [],
    tests = [],
)
```

**`bazel build`** — generates a complete HTML documentation zip via Sphinx.
Internally:
1. `_dependable_element_index` generates an `index.rst` aggregating all
   artifacts, runs archver architecture validation as a subrule, and
   produces a DE-level LOBSTER report (`lobster_de.conf` template covering
   Feature Req → Component Req → Architecture → Public API → Failure Modes).
2. `sphinx_module` compiles all RST sources + diagrams into an HTML zip.

**`bazel test`** — runs the LOBSTER CI report embedded in the index rule
and all component / unit tests transitively.

```bash
bazel build //examples/seooc:safety_software_seooc_example   # HTML zip
bazel test  //examples/seooc:safety_software_seooc_example   # all tests
```
