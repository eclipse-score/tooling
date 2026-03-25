<!--
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
-->

# PlantUML Parser

The PlantUML Parser is a multi-file parser that processes PlantUML diagram files and converts them into structured data for traceability and architectural analysis. It handles preprocessing of include directives, supports multiple diagram types (Class, Sequence, Component), and generates FlatBuffers binary output for further downstream processing.

---

## What It Does

The parser takes `.puml` source files as input and produces:

- **FlatBuffers binary** (`.fbs.bin`) — primary structured output for downstream consumers
- **LOBSTER traceability JSON** (`.lobster`) — for component diagrams, enables traceability linking via the LOBSTER toolchain
- **Debug JSON** — raw and resolved ASTs when `--log-level debug` or higher is set


Rust-based parser that produces an AST for the following PlantUML diagram types:

- Class Diagram
- Sequence Diagram
- Component Diagram

Outputs are `.fbs.bin` FlatBuffers binaries (diagram AST) and optionally `.lobster`
traceability files (for component diagrams).

## Usage

### Build:
```
bazel build //tools/plantuml/parser
```

### Run:
```
bazel run //tools/plantuml/parser -- [OPTIONS]
```

Options:

| Option | Description | Default |
|--------|-------------|---------|
| `--file <FILE>...` | One or more PUML files to parse (repeatable) | — |
| `--folders <DIR>` | Folder containing PUML files | — |
| `--log-level <error\|warn\|info\|debug\|trace>` | Logging verbosity | `warn` |
| `--diagram-type <component\|class\|sequence\|none>` | Diagram type hint | `none` |
| `--fbs-output-dir <DIR>` | Output directory for `.fbs.bin` FlatBuffers files | none (no output) |
| `--lobster-output-dir <DIR>` | Output directory for `.lobster` traceability files | none (no output) |

At least one of `--file` or `--folders` is required.

Example:
```
bazel run //tools/plantuml/parser -- \
    --file $PWD/tools/plantuml/parser/integration_test/component_diagram/simple_component.puml \
    --log-level trace \
    --diagram-type component \
    --fbs-output-dir $PWD/tools/plantuml/parser
```
## Architecture

The parser is organized into separate crate modules per diagram type:

```
puml_parser/
├── src/
│   ├── class_diagram/       # Class diagram parser (pest grammar → AST)
│   ├── component_diagram/   # Component diagram parser
│   ├── sequence_diagram/    # Sequence diagram parser (two-stage: syntax → logic)
│   └── ...                  # Shared utilities (parser_core, puml_utils)
puml_cli/                    # CLI entry point (clap-based)
```

Each diagram parser uses [pest](https://pest.rs/) PEG grammars to tokenize PlantUML input,
then builds a typed AST. The CLI (`puml_cli`) dispatches to the appropriate parser based on
`--diagram-type` or auto-detection.

For the detailed design and users Guide, see [README](docs/README.md).
