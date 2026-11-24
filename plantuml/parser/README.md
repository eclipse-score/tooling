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

### Metadata example

Each C++ example is independent. Template declarations introduce parameters;
uses supply arguments, which may themselves be template instantiations:

```cpp
template <typename T> class Inner {};
template <typename Value, typename Policy> class Outer {};

using Example = const Outer<Inner<int>*, const double&>*;
```

Here, `<int>` belongs to `Inner`; the outer `<...>` supplies two arguments to
`Outer`: `Inner<int>*` and `const double&`.

The parser retains the original type string and records:

| Field | Example value | Meaning |
|-------|---------------|---------|
| `name` | `Outer` | Outermost template name. |
| `arguments` | `Inner<int>*`, `const double&` | Complete explicit arguments; only boundary whitespace is trimmed. |
| `outer_decorators` | `[const, *]` | Outer cv/pointer/reference layers, inner-to-outer; cv order is canonicalized per layer. |

Type aliases, attributes and typed parameters use `template_instantiation`;
method and free-function returns use `return_template_instantiation`.
Extraction is syntactic, not C++ name lookup: a named argument is not proven to
be a type.

**Return qualifiers versus method qualifiers:**

```cpp
template <typename T> class Box {};

class Service {
public:
  Box<int>* const call();
};
```

PUML `call(): Box<int>* const` means a const pointer return;
`call() const : Box<int>*` means a const method.

### Unsupported metadata cases

Each row is independent; declarations appear before their uses.

| Case | C++ template definition | Use example | Limitation |
|------|-------------------------|-------------|------------|
| Non-type or mixed arguments | `template <typename T, int Size> class Mixed {};` | `Mixed<int, 3>` | `3` is a value. Clang omits non-type argument metadata; PUML rejects numeric, boolean, null and quoted literals. |
| Function arguments | `template <typename T> class Box {};` | `Box<void (*)(int)>` | Clang rejects function-like nodes; PUML supports only limited function syntax. |
| Array arguments | `template <typename T> class Box {};` | `Box<int[3]>` | Array nodes are unsupported metadata arguments. |
| Member-pointer arguments | `struct C {};`<br>`template <typename T> class Box {};` | `Box<int C::*>` | PUML excludes member pointers, expressions, brace-wrapped types and internal pack expansions; some also fail parsing. |
| Dependent types | `template <typename T> class Box {};` | `template <typename T> struct Holder { using DependentBox = Box<typename T::type>; };` | `T::type` depends on `T`. PUML rejects explicit `typename`/`template` and `auto`; Clang rejects `Unknown` and `Dependent` nodes. |

**Fallback:** one unsupported nested node removes metadata for the whole
instantiation; this does not imply invalid C++. If PUML parsing succeeds, the
original type string remains available for complete-type comparison.

### Matching limits

#### Template name and empty arguments

| C++ template definition | Use example | Matching behavior |
|-------------------------|-------------|-------------------|
| `template <typename T = int> class Box {};` | `Box` | No template metadata; not an argument-list wildcard. |
| `template <typename T = int> class Box {};` | `Box<>` | Empty explicit list; matches only an empty list. The C++ default `int` is not expanded. |

#### Nested argument omission

```cpp
struct DefaultPolicy {};
template <typename Value, typename Extra = double> class Inner {};
template <typename Value, typename Policy = DefaultPolicy> class Outer {};

```

Only the **outermost** list allows omission. With the definitions above:

| Design type | Implementation type | Matching behavior |
|-------------|---------------------|-------------------|
| `Outer<Inner<int>>` | `Outer<Inner<int, double>, DefaultPolicy>` | No match: nested argument strings differ despite equivalent C++ defaults. |
| `Outer<Inner<int, double>>` | `Outer<Inner<int, double>, DefaultPolicy>` | Prefix match: the nested argument is identical. |

#### Conditions for prefix matching

- The prefix must be nonempty; omitted tails need not come from defaults.
- Both inputs need supported template and outer-decorator metadata; Clang must
  confirm supported type arguments throughout the implementation's argument tree.
- Outer decorators remain constrained. Missing metadata means complete-type
  comparison, without prefix relaxation.

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
| `--include-file <FILE>...` | File that `!include` directives may name besides the diagrams (repeatable; `.puml` or `.iuml`). It is neither parsed as a diagram nor written as an output. Not combinable with `--fta-output-dir`. | none |
| `--include-source-name <PATH>...` | Workspace-relative path embedded in the source locations of each `--include-file`; give none or one per `--include-file`, in the same order. | the file's absolute path |

At least one of `--file` or `--folders` is required. Inputs must have the `.puml`
extension; the same file given twice is an error.

A path in an include directive (`!include`, `!include_once`, `!include_many`, `!includesub`)
is relative to the directory of the file holding it, as in PlantUML, and must name an
`--include-file`: a file that is not declared is never read, and a diagram of the run cannot
be included. A directive that does not is an error naming its file and line, with a hint if a
file of the same path suffix is available. Source locations in the outputs and parse errors
name the file and line an included line was written at; in a diagram that defines
`!procedure`s they carry the line of the fully expanded text instead, and a warning says so.

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
