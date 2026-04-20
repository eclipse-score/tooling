<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# Validation Core

`validation/core` provides the shared Rust library and CLI used to validate
consistency between Bazel architecture data and PlantUML-derived models.

The package contains two public targets:

| Target | Kind | Purpose |
|--------|------|---------|
| `//validation/core:validation` | `rust_library` | Shared readers, models, and validators |
| `//validation/core:validation_cli` | `rust_binary` | CLI entrypoint that infers which validations can run from supplied inputs |

## What It Validates

The current implementation supports two validation flows:

1. `BazelComponent`: compares the indexed Bazel build graph with the indexed
   PlantUML component-diagram structure.
2. `ComponentClass`: compares component-diagram unit aliases with enclosing
  namespace IDs observed in class diagrams.

The CLI builds a `ValidationContext` from the provided inputs, infers which of
these flows are executable, and runs all compatible validators in one pass.

## Layering

The crate is intentionally split into three layers:

- `readers/`: deserialize raw input files.
- `models/`: normalize those inputs into indexed structures used by
  validations.
- `validators/`: compare prepared model/index structures and accumulate
  `Errors`.

`src/main.rs` is the orchestration boundary. It reads CLI arguments, builds the
shared `ValidationContext`, selects runnable validators, merges their results,
and optionally writes a validation log.

This keeps validators focused on comparison logic instead of file loading or
model construction.

## Inputs

The CLI accepts the following input families:

- `--architecture-json`: Bazel architecture export consumed by `BazelReader`
- `--component-fbs`: one or more component-diagram FlatBuffers files consumed by
  `ComponentDiagramReader`
- `--class-fbs`: one or more class-diagram FlatBuffers files consumed by
  `ClassDiagramReader`

The current inference rules are:

- `--architecture-json` + `--component-fbs` enables `BazelComponent`
- `--component-fbs` + `--class-fbs` enables `ComponentClass`

If both combinations are present, both validators are executed.

## Run

Build the CLI:

```bash
bazel build //validation/core:validation_cli
```

Run it directly:

```bash
bazel run //validation/core:validation_cli -- \
    --architecture-json path/to/architecture.json \
    --component-fbs path/to/component.fbs.bin \
    --class-fbs path/to/class.fbs.bin \
    --output path/to/validation.log
```

Run unit tests:

```bash
bazel test //validation/core:validation_test
```

## Architectural Overview

PlantUML source diagrams for the current design are stored in:

- `docs/assets/validation_core_overview.puml`
- `docs/assets/validation_core_flow.puml`

The first diagram shows the static module responsibilities. The second shows
the runtime flow from CLI input parsing to validator execution and result
aggregation.
