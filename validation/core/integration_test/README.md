<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Validation Integration Tests

End-to-end tests for the `validation_cli` binary. Each test case feeds real
PlantUML diagrams through the full pipeline — PlantUML parser → FlatBuffers
binary → validation CLI — and asserts the outcome against a JSON/YAML fixture.

## Directory structure

```
integration_test/
├── BUILD                        # shared Rust test framework library
├── puml_fixture.bzl             # Starlark rule: provider → category dirs
├── bazel_component/             # BazelComponent suite, cases, and test binary
├── component_class/             # ComponentClass suite, cases, and test binary
├── component_sequence/          # ComponentSequence suite, cases, and test binary
├── component_internal_api/      # ComponentInternalApi cases
├── sequence_internal_api/       # SequenceInternalApi cases
├── class_design_implementation/ # ClassDesignImplementation suite, cases, and test binary
├── src/                         # Rust crate sources for shared test_framework
│   ├── lib.rs                   # re-exports from test_framework
│   └── test_framework.rs        # shared helpers (CLI runner, assertions)
```

## How it works

The framework has three layers.

### Layer 1 — PlantUML parsing (Bazel rules)

Each test case BUILD calls the real production rules, such as
`architectural_design`, `unit_design`, or `unit`, on its source files. Bazel
runs the parser actions and stores the result as FlatBuffers binaries
(`.fbs.bin`). The output is exposed through providers:

| Rule | Provider | Fields used |
|------|----------|-------------|
| `architectural_design` | `ArchitecturalDesignInfo` | `static` (component), `dynamic` (sequence), `internal_api` (internal API) |
| `unit_design` | `UnitDesignInfo` | `static` (unit design class), `dynamic` (unit design sequence) |
| `unit` | `UnitInfo` | `implementation_class_fbs`, `implementation_sequence_fbs` |

### Layer 2 — Fixture preparation (`puml_fixture.bzl`)

The `provider_fbs_fixture_bundle` rule reads the provider fields from its `deps`
and creates a predictable directory layout that the Rust test binary can
navigate at runtime:

| Category | Source provider field |
|----------|-----------------------|
| `component/` | `ArchitecturalDesignInfo.static` |
| `internal_api/` | `ArchitecturalDesignInfo.internal_api`, when present |
| `sequence/` | `ArchitecturalDesignInfo.dynamic` |
| `unit_design_class/` | `UnitDesignInfo.static` |
| `unit_design_sequence/` | `UnitDesignInfo.dynamic` |
| `unit_implementation_class/` | `UnitInfo.implementation_class_fbs` |
| `unit_implementation_sequence/` | `UnitInfo.implementation_sequence_fbs` |

Each file in these directories is a **symlink** to the canonical `.fbs.bin`
produced in layer 1. No copying or re-parsing occurs; the underlying Bazel
action cache entry is reused.

A `filegroup` named `case_data` then bundles the `fbs` target together with the
static fixture files (`architecture.json`, `expected.json` or `expected.yaml`),
making the whole case available as a single Bazel dependency.

For `ComponentInternalApi` and `SequenceInternalApi` suites, cases include
`internal_api/*.fbs.bin`, and the suite forwards those files to the CLI as
the `internal_api_diagrams` input bundle field.

`ClassDesignImplementation` cases use both `unit_design` and `unit`. The
`unit_design` rule produces design class diagrams under `unit_design_class/`,
while the `unit` rule runs the C++ parser for implementation targets and exposes
their class diagrams under `unit_implementation_class/`.

### Layer 3 — CLI invocation (Rust test binary)

There is one `rust_test` binary per suite, defined in that suite's `BUILD` file.
Each binary lists the relevant suite-level `*_test_data` filegroup and the
`validation_cli` binary in its `data` attribute so that Bazel places them under
`TEST_SRCDIR` at test time.

The shared `test_framework` library provides the following helpers:

| Helper | Description |
|--------|-------------|
| `collect_case_fbs_files(suite, case, category)` | Returns sorted absolute paths to every `.fbs.bin` in a category subdirectory |
| `load_expected_fixture(suite, case)` | Deserializes `expected.json` into `ExpectedFixture` |
| `load_expected_yaml_fixture(suite, case)` | Deserializes `expected.yaml` into `ExpectedFixture` |
| `run_validation_profile(case_name, profile, input_bundle)` | Writes a profile-owned input bundle, spawns the CLI binary, and returns `CliRunResult` |
| `assert_cli_result(case, expected, result)` | Asserts exit code and checks each string in `error_contains` against the log |

Each `#[test]` function calls an `assert_case(case_dir)` helper that wires these
steps together.

### Artifact flow for one test case

```
component_diagram.puml
        │
        │  architectural_design Bazel rule
        │  → puml_parser action (cached per .puml content)
        ▼
design/component_diagram.fbs.bin        ← in ArchitecturalDesignInfo.static

        │  provider_fbs_fixture_bundle rule
        │  → symlink action (zero cost)
        ▼
component/component_diagram.fbs.bin     ← symlink → file above

        │  filegroup case_data
        │  → bundles fbs + architecture.json + expected.json
        ▼
//.../bazel_component:bazel_component_test_data ← aggregated across all cases

        │  rust_test data = [...]
        │  → placed under TEST_SRCDIR at test execution
        ▼
//.../bazel_component:integration_test
        │
        │  collect_case_fbs_files()  → absolute paths to .fbs.bin files
        │  load_expected_fixture()   → ExpectedFixture
        │  run_validation_profile()  → writes inputs and spawns validation_cli
        │  assert_cli_result()       → checks pass/fail + error substrings
        ▼
PASS / FAIL
```

## Test case anatomy

Each test case is a self-contained directory. The exact files required depend on
the validator under test.

### BazelComponent cases

```
<case>/
├── BUILD                 # architectural_design + provider_fbs_fixture_bundle + case_data
├── component_diagram.puml
├── architecture.json     # Bazel build-graph snapshot (components/units)
└── expected.json
```

### ComponentClass cases

```
<case>/
├── BUILD                 # architectural_design + unit_design + provider_fbs_fixture_bundle + case_data
├── component_diagram.puml
├── class_diagram.puml    # one or more; multi-file: class_diagram_part1.puml, ...
└── expected.json
```

### ClassDesignImplementation cases

```
<case>/
├── BUILD                 # unit_design + implementation cc_library + unit + provider_fbs_fixture_bundle + case_data
├── class_diagram.puml    # unit design class diagram
├── transport.cpp         # or other implementation source(s)
├── transport.h           # optional implementation header(s)
└── expected.yaml
```

### ComponentSequence cases

```
<case>/
├── BUILD                 # architectural_design + provider_fbs_fixture_bundle + case_data
├── component_diagram.puml
├── sequence_diagram.puml
└── expected.json
```

### ComponentInternalApi cases

```
<case>/
├── BUILD                 # architectural_design + provider_fbs_fixture_bundle + case_data
├── component_diagram.puml
├── internal_api_diagram.puml
└── expected.json
```

### SequenceInternalApi cases

```
<case>/
├── BUILD                 # architectural_design + provider_fbs_fixture_bundle + case_data
├── component_diagram.puml
├── sequence_diagram.puml
├── internal_api_diagram.puml
└── expected.json
```

### `expected.json` / `expected.yaml` format

```json
{
  "should_pass": true,
  "error_contains": []
}
```

| Field | Type | Description |
|-------|------|-------------|
| `should_pass` | `bool` | Whether the CLI must exit with code 0 |
| `error_contains` | `string` or `string[]` | Substrings that must appear in the CLI log on failure. Empty for positive cases. |

The framework uses **substring matching** for `error_contains`, so entries do
not need to reproduce exact formatting — just enough context to uniquely
identify the error. Existing suites use `expected.json`; the
`class_design_implementation` suite uses `expected.yaml` so larger multi-line
error fragments stay readable.

## Running the tests

Run all integration tests:

```bash
bazel test //validation/core/integration_test/...
```

Run a single suite:

```bash
bazel test //validation/core/integration_test/bazel_component:integration_test
bazel test //validation/core/integration_test/component_class:integration_test
bazel test //validation/core/integration_test/component_sequence:integration_test
bazel test //validation/core/integration_test/component_internal_api:component_internal_api_integration_test
bazel test //validation/core/integration_test/sequence_internal_api:sequence_internal_api_integration_test
bazel test //validation/core/integration_test/class_design_implementation:integration_test
```

## Adding a new test case

1. Create a directory under the appropriate suite folder:
   ```
   validation/core/integration_test/<suite>/<case_name>/
   ```

2. Add the `.puml` source file(s) and — for `bazel_component` — an
   `architecture.json`.

3. Write `expected.json` or `expected.yaml`. For negative cases add the error
  substrings you expect to see in the CLI log.

4. Create a `BUILD` file following the pattern of an existing case in the same
   suite.

5. Add the new `case_data` target to the matching filegroup in
  [`BUILD`](BUILD) (`bazel_component_test_data`,
  `component_class_test_data`, `component_sequence_test_data`,
  `component_internal_api_test_data`, `sequence_internal_api_test_data`,
  or `class_design_implementation_test_data`).

6. Add a `#[test]` function in the matching suite file, such as
   `bazel_component_suite.rs` or `class_design_implementation_suite.rs`.

## Caching behaviour

- The `puml_parser` action is shared: the same `.fbs.bin` is used by both
  production `dependable_element` targets and integration test fixtures — no
  redundant parsing.
- Symlinks created by `provider_fbs_fixture_bundle` have zero action cost; the
  cache key for a test case only covers the underlying `puml_parser` action.
- Changing one test case's `.puml` only invalidates actions for that case; the
  other cases and test binaries stay cached.
- The suite test binaries are independent: touching a sequence test case does
  not invalidate the Bazel component, component class, or class design
  implementation test binaries.
