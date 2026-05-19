<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# General Information

`rules_score` provides a set of Bazel rules that help you build and document a **Safety Element out of Context (SEooC)** — a safety-critical software component developed independently and delivered with all the evidence needed for integration into a safety-relevant system.

By declaring your workproducts (requirements, architecture, units, safety analysis) as Bazel targets, `rules_score` automatically verifies traceability and consistency of all workproducts and assembles them into a Sphinx HTML documentation including the traceability report.

## The Dependable Element Concept

A *dependable element* is the top-level unit of certification work. It bundles:

| Artifact | What it contains |
|---|---|
| Assumed System Requirements | System-level requirements given as constraints from the surrounding context |
| Feature Requirements | Functional and safety requirements for this element |
| Assumptions of Use | Conditions the integrating project must satisfy |
| Architectural Design | Software Architectural Design in PlantUML |
| Software Units and Components | Implementation targets linked to their design |
| Dependability Analysis | FMEA, FTA diagrams and control measures |

When you run `bazel build //:my_element`, all these pieces are assembled into a single HTML documentation site at `bazel-bin/my_element/html/`.

## Build Flow

The diagram below shows how your input files flow through the Bazel rules to produce the final outputs.

```{uml} ../_assets/seooc_flow.puml
:align: center
:alt: SEooC build flow
:width: 90%
```

## Assembling a Dependable Element

### Step 1 — Define your artifacts

Define your requirements, architecture, units, and safety analysis using the rules described in the topic pages:

- {doc}`requirements` — `assumed_system_requirements`, `feature_requirements`, `component_requirements`, `assumptions_of_use`
- {doc}`architectural_design` — `architectural_design`, `unit`, `component`
- {doc}`unit_design` — `unit_design`
- {doc}`dependability_analysis` — `fmea`, `dependability_analysis`

### Step 2 — Wire them together

```{code-block} starlark
dependable_element(
    name = "safety_software_seooc_example",
    architectural_design = ["//bazel/rules/rules_score/examples/seooc/design:sample_seooc_design"],
    assumptions_of_use = [],
    components = [":component_example"],
    dependability_analysis = [":sample_dependability_analysis"],
    integrity_level = "B",
    requirements = ["//bazel/rules/rules_score/examples/seooc/docs/requirements:feature_requirements"],
    tests = [],
    deps = ["//bazel/rules/rules_score/examples/some_other_library:other_seooc"],
)
```

### Step 3 — Build

```bash
bazel build //my/package:my_element
```

Output:

```
bazel-bin/my/package/my_element/html/      ← HTML documentation
bazel-bin/my/package/my_element_index/     ← traceability report (JSON + HTML)
```

Run traceability checks:

```bash
bazel test //my/package:my_element
```

## Rule Reference `dependable_element`

For the complete `dependable_element` attribute reference, see {ref}`dependable_element <rule-dependable-element>` in the rule index.

## Automatic Validations

`rules_score` enforces the following constraints at **build time** — the build fails if any of them are violated:

TODO: Link here the Test Specifications for the Validations for more details

### Architecture consistency

The components and units declared in `dependable_element.components` are compared against the static PlantUML diagrams in `architectural_design`. Every component or unit that appears in the implementation tree must also appear in the architecture diagrams.

### Certified scope

Every Bazel target that is transitively referenced through `unit.implementation` must fall within the package tree declared by the `unit` and `component` targets belonging to this element. External library dependencies that are not safety-certified must not appear there.

When `maturity = "development"` is set, scope violations are printed as warnings instead of failing the build. Switch back to `"release"` before certification.

### Integrity level

A `dependable_element` with `integrity_level = "B"` must not depend (via `deps`) on another `dependable_element` with `integrity_level = "A"`. The hierarchy is D > C > B > A.
