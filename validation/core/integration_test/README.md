<!--
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
-->

# Validation Integration Tests

End-to-end tests for the `validation_cli` binary. Each test case feeds real
PlantUML diagrams through the full pipeline — PlantUML parser → FlatBuffers
binary → validation CLI — and asserts the outcome against a JSON fixture.

## Directory structure

```
integration_test/
├── BUILD                        # test binaries + aggregated filegroups
├── puml_fixture.bzl             # Starlark rule: provider → category dirs
├── src/
│   ├── lib.rs                   # re-exports from test_framework
│   ├── test_framework.rs        # shared helpers (CLI runner, assertions)
│   ├── bazel_component_suite.rs # tests for BazelComponent validator
│   ├── component_class_suite.rs # tests for ComponentClass validator
│   └── component_sequence_suite.rs # tests for ComponentSequence validator
```

## How it works

The framework has three layers.

### Layer 1 — PlantUML parsing (Bazel rules)

Each test case BUILD calls the real production rules (`architectural_design`,
`unit_design`) on its `.puml` source files. Bazel runs the `puml_parser` binary
as a cached action and stores the result as a FlatBuffers binary (`.fbs.bin`).
The output is exposed through providers:

| Rule | Provider | Fields used |
|------|----------|-------------|
| `architectural_design` | `ArchitecturalDesignInfo` | `static` (component), `dynamic` (sequence) |
| `unit_design` | `UnitDesignInfo` | `static` (class), `dynamic` (sequence) |

### Layer 2 — Fixture preparation (`puml_fixture.bzl`)

The `provider_fbs_fixture_bundle` rule reads the provider fields from its `deps`
and creates a predictable directory layout that the Rust test binary can
navigate at runtime:

```
fbs/
├── component/   ← from ArchitecturalDesignInfo.static
├── class/       ← from UnitDesignInfo.static
└── sequence/    ← from ArchitecturalDesignInfo.dynamic + UnitDesignInfo.dynamic
```

Each file in these directories is a **symlink** to the canonical `.fbs.bin`
produced in layer 1. No copying or re-parsing occurs; the underlying Bazel
action cache entry is reused.

A `filegroup` named `case_data` then bundles the `fbs` target together with the
static fixture files (`architecture.json`, `expected.json`), making the whole
case available as a single Bazel dependency.

### Layer 3 — CLI invocation (Rust test binary)

There is one `rust_test` binary per validator. Each binary lists the relevant
`case_data` filegroups and the `validation_cli` binary in its `data` attribute
so that Bazel places them under `TEST_SRCDIR` at test time.

The shared `test_framework` library provides the following helpers:

| Helper | Description |
|--------|-------------|
| `collect_case_fbs_files(suite, case, category)` | Returns sorted absolute paths to every `.fbs.bin` in a category subdirectory |
| `load_expected_fixture(suite, case)` | Deserialises `expected.json` into `ExpectedFixture` |
| `run_validation_cli(case_name, cli_args)` | Spawns the CLI binary, writes its log to `TEST_TMPDIR`, returns `CliRunResult` |
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
fbs/component/component_diagram.fbs.bin ← symlink → file above

        │  filegroup case_data
        │  → bundles fbs + architecture.json + expected.json
        ▼
bazel_component_test_data               ← aggregated across all cases

        │  rust_test data = [...]
        │  → placed under TEST_SRCDIR at test execution
        ▼
bazel_component_integration_test
        │
        │  collect_case_fbs_files()  → absolute paths to .fbs.bin files
        │  load_expected_fixture()   → ExpectedFixture
        │  run_validation_cli()      → spawns validation_cli --output <log>
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

### ComponentSequence cases

```
<case>/
├── BUILD                 # architectural_design (static + dynamic) + provider_fbs_fixture_bundle + case_data
├── component_diagram.puml
├── sequence_diagram.puml
└── expected.json
```

### `expected.json` format

```json
{
  "should_pass": true,
  "error_contains": []
}
```

| Field | Type | Description |
|-------|------|-------------|
| `should_pass` | `bool` | Whether the CLI must exit with code 0 |
| `error_contains` | `string[]` | Substrings that must appear in the CLI log on failure. Empty for positive cases. |

The framework uses **substring matching** for `error_contains`, so entries do
not need to reproduce exact formatting — just enough context to uniquely
identify the error.

## Running the tests

Run all integration tests:

```bash
bazel test //validation/core/integration_test/...
```

Run a single suite:

```bash
bazel test //validation/core/integration_test:bazel_component_integration_test
bazel test //validation/core/integration_test:component_class_integration_test
bazel test //validation/core/integration_test:component_sequence_integration_test
```

## Adding a new test case

1. Create a directory under the appropriate suite folder:
   ```
   validation/core/integration_test/<suite>/<case_name>/
   ```

2. Add the `.puml` source file(s) and — for `bazel_component` — an
   `architecture.json`.

3. Write `expected.json`. For negative cases add the error substrings you
   expect to see in the CLI log.

4. Create a `BUILD` file following the pattern of an existing case in the same
   suite.

5. Add the new `case_data` target to the matching filegroup in
   [`BUILD`](BUILD) (`bazel_component_test_data`,
   `component_class_test_data`, or `component_sequence_test_data`).

6. Add a `#[test]` function in the matching suite file under `src/`.

## Caching behaviour

- The `puml_parser` action is shared: the same `.fbs.bin` is used by both
  production `dependable_element` targets and integration test fixtures — no
  redundant parsing.
- Symlinks created by `provider_fbs_fixture_bundle` have zero action cost; the
  cache key for a test case only covers the underlying `puml_parser` action.
- Changing one test case's `.puml` only invalidates actions for that case; the
  other cases and test binaries stay cached.
- The three test binaries are independent: touching a sequence test case does
  not invalidate the component or class test binary.
