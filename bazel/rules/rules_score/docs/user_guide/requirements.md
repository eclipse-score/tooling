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

# Requirements

`rules_score` provides three rules for capturing different levels of requirements.

## Requirement Hierarchy & Traceability

```
AssumedSystemReq  →  FeatReq  →  CompReq
    (System)        (Feature)   (Component)
         \                          ↑
          \________________________/
```

```{list-table}
:header-rows: 1
:widths: 18 47 35

* - Type
  - Description
  - Traceability
* - **AssumedSystemReq**
  - Requirements from the user / assumed system towards the SEooC.

    Too high-level for a single component — can only be satisfied by
    multiple components working together.
  - Root — no parent
* - **FeatReq**
  - Refined requirements derived from `AssumedSystemReq`.

    Used when assumed system requirements are too high-level to be broken
    down directly to one component — still require multiple components.
  - **Must** reference ≥ 1 `AssumedSystemReq` via `derived_from`
* - **CompReq**
  - Requirements assigned to exactly one component.

    Can be directly implemented and tested within that component.
  - Optionally references ≥ 1 `FeatReq` via `derived_from`
    using `[Package.FeatReq@version]`
```

Traceability is enforced by the trlc type system — version pinning (e.g. `@1`) ensures that when a parent requirement changes, all downstream references must be explicitly updated.

Each rule consumes one or more `.trlc` source files and produces a target that carries both a Sphinx documentation page and traceability information for downstream rules. The TRLC Type Model (.rsl file) is already included in the rule.

## Modeling Requirements in TRLC

All requirements are written in [TRLC](https://github.com/bmw-software-engineering/trlc) (Traceability Requirements Language Checker). Each record maps to a specific `ScoreReq` type defined in the [S-CORE requirements model](https://github.com/eclipse-score/tooling/blob/main/bazel/rules/rules_score/trlc/config/score_requirements_model.rsl).

For `TRLC` both a VSCode Extension and a LSP Server (e.g. for Clion) are [available](https://github.com/bmw-software-engineering/trlc-vscode-extension)

### Assumed System Requirements

System-level requirements that your SEooC receives from the wider context — for example, from a system specification.

```text
package MySeooc

import ScoreReq

ScoreReq.AssumedSystemReq SYSREQ_001 {
    description = "The system shall provide a real-time clock interface"
    safety      = ScoreReq.Asil.B
    rationale   = "Required for time-stamped log entries"
    version     = 1
}
```

### Feature Requirements

```text
package MySeooc

import ScoreReq

ScoreReq.FeatReq FEAT_001 {
    description  = "The component shall store key-value pairs persistently"
    safety       = ScoreReq.Asil.B
    derived_from = [MySeooc.SYSREQ_001@1]
    version      = 1
}
```

### Component Requirements

`derived_from` uses the versioned tuple syntax `[Package.RecordId@version]`.

```text
package MySeooc

import ScoreReq

ScoreReq.CompReq COMP_001 {
    description  = "Write operations shall complete within 5 ms"
    safety       = ScoreReq.Asil.B
    derived_from = [MySeooc.FEAT_001@1]
    version      = 1
}
```

### Assumptions of Use

Conditions that the *integrating project* must satisfy when using your SEooC. The optional `mitigates` field describes (as a free-form string) the hazard or risk that is mitigated when this assumption is fulfilled.

Traceability to requirements is established at the Bazel level via the `requirements` attribute on the `assumptions_of_use` rule — there is no TRLC `derived_from` or `satisfies` field on `AoU`.

```text
package MySeooc

import ScoreReq

ScoreReq.AoU AOU_001 {
    description = "The integrator shall ensure exclusive write access to the storage partition"
    safety      = ScoreReq.Asil.B
    mitigates   = "ConcurrentWriteCorruption"
    version     = 1
}
```

## Allocation of Requirements to Architectural Elements

Requirements are allocated to architectural elements differently depending on their level:

**Component Requirements (`CompReq`)**
`CompReq` records are associated with exactly one component. The allocation is expressed implicitly through Bazel: the `component.requirements` attribute lists the `component_requirements` targets that belong to that component. Because a `component` maps directly to an architectural element in the static PlantUML diagram, the allocation to the architecture is established automatically.

**Feature Requirements (`FeatReq`)**
`FeatReq` records operate at the integration level — they are too broad for a single component and can only be satisfied by multiple components working together. They are therefore allocated to the `dependable_element` as a whole via the Bazel `requirements` attribute:

```starlark
dependable_element(
    name = "my_element",
    requirements = [":feature_requirements"],   # FeatReq targets
    ...
)
```

The traceability from `FeatReq` down to the components that implement it runs through the `component_requirements` chain (`FeatReq → CompReq → component`).

## Modeling Requirements in Bazel Rules

For the complete attribute reference for all requirements Bazel rules, see the rule index:

- {ref}`assumed_system_requirements <rule-assumed-system-req>`
- {ref}`feature_requirements <rule-feature-requirements>`
- {ref}`component_requirements <rule-component-requirements>`
- {ref}`assumptions_of_use <rule-assumptions-of-use>`

## Validation

Every requirement target generates a `<name>_test` target that runs `trlc --verify` on your `.trlc` sources. This check runs automatically as part of `bazel test ...`.

The validation catches:

- **Syntax errors** — malformed TRLC records
- **Type errors** — wrong value types for fields (e.g. a string where an enum is expected)
- **Mandatory field violations** — missing `description`, `safety`, or `version`
- **Broken cross-references** — a `derived_from` or `satisfies` pointing to a non-existent record
- **Unknown fields** — fields not defined in the S-CORE requirements model

To run the validation for a single target:

```bash
bazel test //my/package:my_feature_req_test
```

## AI-Powered Quality Check

In addition to the structural TRLC validation described above, `rules_score` provides an optional AI-powered quality check for requirements via the `trlc_requirements_ai_test` rule. Unlike the structural check — which validates syntax, types, and cross-references — the AI check evaluates the *quality* of each requirement against requirements engineering guidelines (clarity, testability, completeness, etc.).

### `trlc_requirements_ai_test`

```starlark
load("@score_tooling//validation/ai_checker:ai_checker.bzl",
     "trlc_requirements_ai_test")

trlc_requirements_ai_test(
    name = "feature_requirements_ai_check",
    reqs = [":feature_requirements"],
    score_threshold = "6.0",
    tags = ["manual"],
)
```

The `tags = ["manual"]` attribute is strongly recommended to prevent the rule from running automatically during routine `bazel test //...` sweeps. The check requires a locally initialized copilot CLI or network access to an AI model in a cloudroom.

Run the check explicitly with:

```bash
bazel test //my/package:feature_requirements_ai_check --config=copilot
```

| Attribute | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Target name |
| `reqs` | label list | yes | Requirement targets to analyse (any target providing `TrlcProviderInfo`, e.g. `feature_requirements`, `component_requirements`) |
| `model` | string | no | AI model identifier (default: `"anthropic/claude-sonnet-4-5"`) |
| `score_threshold` | string | no | Minimum average quality score from 0 to 10 to pass the test (default: `"0.0"`) |
| `guidelines` | label | no | Filegroup of guideline Markdown files to override the built-in guidelines |

**Output files** (written to `bazel-bin/`):

| File | Content |
|---|---|
| `<name>_analysis.json` | Machine-readable scores, findings, and suggestions per requirement |
| `<name>_analysis.html` | Interactive HTML report with colour-coded score cards and guideline references |

**Prerequisites:** a GitHub Copilot licence (default) or a custom AI model configured via the `_custom_ai_model` attribute — see `https://github.com/eclipse-score/tooling/blob/main/validation/ai_checker/README.md` in the score-tooling repository for details.
