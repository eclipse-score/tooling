<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# PlantUML Parser

The PlantUML Parser is a multi-file parser that processes PlantUML diagram files and converts them into structured data for traceability and architectural analysis. It handles preprocessing of include directives, supports multiple diagram types (Class, Sequence, Component), and generates FlatBuffers binary output for further downstream processing.

---

## What It Does

The parser takes `.puml` source files as input and produces:

- **FlatBuffers binary** (`.fbs.bin`) — primary structured output for downstream consumers
- **Public API TRLC** (`--api-output-file`) — one `ScoreReq.ApiMethod` record per public method (all overloads) and per free function of the class diagrams, in the TRLC package given by `--api-package`
- **Debug JSON** — raw and resolved ASTs when `--log-level debug` or higher is set


Rust-based parser that produces an AST for the following PlantUML diagram types:

- Class Diagram
- Sequence Diagram
- Component Diagram

Outputs are `.fbs.bin` FlatBuffers binaries (diagram AST).

## Class template instantiation metadata

Type aliases, attributes and typed parameters may carry `template_instantiation`;
methods and free functions may carry `return_template_instantiation`. The shared
`TemplateInstantiation` record stores:

- `name`: the outer template name.
- `arguments`: complete explicit argument strings, trimmed only at their boundaries;
  nested templates and internal whitespace remain intact.
- `outer_decorators`: `const`, `volatile`, pointer and reference decorators in
  inner-to-outer order, with cv order canonicalized within each layer.

For example, `const Outer<Inner<int>*, const T&>*` records name `Outer`, arguments
`Inner<int>*` and `const T&`, and decorators `[const, *]`. The original type string
is retained. `call(): Box<T>* const` describes a const pointer return type, not a
const method; use `call() const : Box<T>*` for the method qualifier.

Extraction reuses the type grammar and is syntactic, not C++ name lookup. Named
arguments, supported multiword builtins and nested templates are accepted;
names such as `N` in `Box<N>` are not proven to be types.

### Unsupported metadata cases

| Case | Example | Limitation |
|------|---------|------------|
| Non-type or mixed arguments | `Mixed<int, 3>` | Clang does not emit metadata for non-type arguments; PUML metadata extraction also rejects numeric, boolean, null and quoted literals. |
| Function types, pointers or references | `Pair<int, void (*)(int)>` | Clang metadata extraction rejects function-like nodes; PUML supports only limited function syntax. |
| Array types | `Box<int[3]>` | Array nodes are not supported as metadata arguments. |
| Other unsupported PUML syntax | `Box<int C::*>` | Member pointers, expressions, brace-wrapped types and internal pack expansions are outside the metadata allowlist. Some forms also fail ordinary PUML parsing. |
| Dependent, unresolved or placeholder types | `Box<typename T::type>` | PUML rejects explicit `typename`/`template` syntax and `auto`; Clang metadata extraction rejects `Unknown` and `Dependent` nodes. |

These restrictions apply recursively: one unsupported node removes metadata for
the entire enclosing instantiation. Missing metadata does not mean that the C++
declaration is invalid. If the UML
declaration parses, its original type string remains available for complete-type
comparison. Metadata extraction does not make unsupported PUML syntax parseable.

### Matching limits

| Case | Example | Behavior |
|------|---------|----------|
| Bare non-template name | `Box` | No template metadata; not an argument-list wildcard. |
| Empty explicit argument list | `Box<>` | Metadata has an empty argument list, which matches only an empty list. |
| Nested argument omission | `Outer<Map<K>>` versus `Outer<Map<K, V>, Policy>` | Nested arguments remain complete strings; omission is allowed only in the outermost list. |
| Missing template or outer-decorator metadata on either side | Legacy input or an unsupported type | No template-prefix relaxation; retain complete-type comparison. |

Downstream class-design validation allows a nonempty outer argument prefix only
when both inputs provide supported template and outer-decorator metadata. Clang
must confirm supported type arguments throughout the implementation's argument
tree. Omitted tails need not originate from defaults; outer decorators remain
constrained.
See [Type Normalization](../../validation/core/docs/specifications/class_design_implementation.md#type-normalization)
for comparison rules, including parameter top-level `const` handling.

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
| `--api-output-file <FILE>` | Output file for the public API TRLC (`ApiMethod` records of class diagrams); all inputs must be class diagrams; requires `--api-package` | none (no output) |
| `--api-package <NAME>` | TRLC package of the public API file (a TRLC identifier); requires `--api-output-file` | none |
| `--output-stem <STEM>` | Override the file stem used to name every output file for this run, instead of deriving it from the input file's own basename. Requires exactly one input file. | none (uses the input file's basename) |

At least one of `--file` or `--folders` is required. Inputs must have the `.puml`
extension; the same file given twice is an error.

All inputs of one run, whether from `--file` or `--folders`, are resolved together: per
diagram kind, the files form one group in which a top-level declaration with a body is
reparented onto the nested declaration it matches in another file, and a relation
endpoint unknown to its file resolves in the other files. Run separate invocations to
keep files apart.

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

Shared model and serialization crates are split by responsibility:

- `tools/metamodel/{class,component,sequence}` contains the logical model types.
- `tools/serialization/flatbuffers/{class,component,sequence}` contains the FlatBuffers
  schemas and serializer crates that produce `.fbs.bin` outputs.

For the detailed design and users Guide, see `docs/README.md` in the source tree.

Authoring guides:

- [`docs/element-identifiers.md`](docs/element-identifiers.md) — how element identifiers are constructed and how to link component, class, and sequence diagrams
- [`docs/class-diagram.md`](docs/class-diagram.md) — class-diagram syntax and free-function conventions
- [`docs/sequence-diagram.md`](docs/sequence-diagram.md) — supported sequence diagram syntax and semantics
