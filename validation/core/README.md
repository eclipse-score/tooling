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
| `//validation/core:validation_cli` | `rust_binary` | CLI entrypoint that dispatches the selected validation profile |

## What It Validates

The current implementation supports these validation flows:

1. `BazelComponent`: compares the indexed Bazel build graph with the indexed
   PlantUML component-diagram structure.
2. `ComponentInternalApi`: checks that every component-diagram interface is
  declared by the Internal API diagram.
3. `ComponentSequence`: checks that component-diagram unit aliases, shared
  interface relations, and sequence-diagram function-call connections stay in
  sync.
4. `SequenceInternalApi`: checks that Internal API methods are exercised by
  sequence interactions. When component input is also available, it uses that
  component context to check sequence function names against related shared
  interfaces.

The CLI dispatches to the selected validation profile. Each profile owns its
input schema, reads the models it needs, and runs the validators that are
available for those profile inputs.

## Layering

The crate is intentionally split into three layers:

- `readers/`: deserialize raw input files.
- `models/`: normalize those inputs into indexed structures used by
  validations.
- `validators/`: compare prepared model/index structures and accumulate
  `Errors`.

`src/main.rs` is the orchestration boundary. It reads CLI arguments, dispatches
to the selected profile, merges validator results, and optionally writes a
validation log.

This keeps validators focused on comparison logic instead of file loading or
model construction.

## Inputs

The CLI accepts a validation profile and a JSON input bundle:

- `--profile`: validation scenario to run. Named profiles such as `architectural-design` select validators in the Rust validation layer.
- `--inputs`: JSON file containing the input paths for the selected profile.

Supported profiles:

| Profile | Status | Input schema | Validation scope |
|---------|--------|--------------|------------------|
| `architectural-design` | Supported | `ArchitecturalDesignInputs` | Design consistency |
| `dependable-element` | Supported | `DependableElementInputs` | Bazel architecture consistency |
| `unit` | Placeholder | not read | none; writes `SKIPPED` |

Profile validators:

`architectural-design`:
- `validate_component_sequence`

`dependable-element`:
- `validate_bazel_component`
- `validate_component_class` (pending)

`unit`:
- placeholder

Each profile owns its own input schema.

`dependable-element`:

```json
{
  "architecture": "path/to/architecture.json",
  "component_diagrams": ["path/to/component.fbs.bin"]
}
```

`architectural-design`:

```json
{
  "component_diagrams": ["path/to/component.fbs.bin"],
  "sequence_diagrams": ["path/to/sequence.fbs.bin"],
  "internal_api": ["path/to/internal_api.fbs.bin"],
  "public_api": ["path/to/public_api.fbs.bin"]
}
```

`unit`:

```json
{
  "design_classes": ["path/to/design_class.fbs.bin"],
  "design_sequences": ["path/to/design_sequence.fbs.bin"],
  "implementation_classes": ["path/to/implementation_class.fbs.bin"],
  "implementation_sequences": ["path/to/implementation_sequence.fbs.bin"]
}
```

Bazel rules declare the profile and provide the input bundle. The Rust
validation layer decides which validators belong to the selected profile and
maps the profile inputs to those validators.

## Run

Build the CLI:

```bash
bazel build //validation/core:validation_cli
```

Run it directly:

```bash
bazel run //validation/core:validation_cli -- \
    --profile dependable-element \
    --inputs path/to/validation_inputs.json \
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
