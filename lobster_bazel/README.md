<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->


| Extension            | Language  | Comment sign |
|----------------------|-----------|--------------|
| `.c`, `.cc`, `.cpp`, `.cxx`, `.h`, `.hh`, `.hpp`, `.hxx` | C/C++  | `//` |
| `.rs`                | Rust      | `//` |
| `.py`                | Python    | `#` |
| `.bzl`               | Starlark  | `#` |
| `.trlc`, `.rsl`      | TRLC      | `#` |

Files with unsupported extensions are silently skipped; the run continues and
reports the number of items extracted from the remaining files.

## Adding tracing tags to source files

Add a single-line comment **at the start of the line** with the tracing tag
attribute and the requirement ID:

```python
# Python example
def process():
    # req-traceability: COMP_REQ_001
    pass
```

```cpp
// C++ example
void process() {
    // req-traceability: COMP_REQ_001
}
```

```rust
// Rust example
fn process() {
    // req-traceability: COMP_REQ_001
}
```

> **Note:** The tag pattern must appear at the start of the (stripped) line.
> Inline comments at the end of a code statement are intentionally **not**
> matched to avoid false positives.

The tag attribute (e.g. `req-traceability`) is configurable via `--tag`.
The default tag used by the `lobster_linker` Bazel rule is `lobster-trace`.

## Creating lobster files

The tool reads **file-list files** as positional arguments. Each file-list file
contains one source file path per line. This design integrates naturally with
Bazel's `$(locations ...)` expansion or any tool that can dump a list of files.

```sh
$ lobster-bazel --output impl.lobster \
    --tag req-traceability \
    source_files.txt
```

Where `source_files.txt` contains:

```
src/module_a.py
src/module_b.rs
include/module_c.hpp
```

Multiple file-list files and multiple `--tag` values are supported:

```sh
$ lobster-bazel --output impl.lobster \
    --tag req-traceability \
    --tag req-Id \
    sources_a.txt sources_b.txt
```

An optional `--namespace` argument controls the namespace prefix for the
generated tags (default: `source`):

```sh
$ lobster-bazel --output impl.lobster \
    --tag req-traceability \
    --namespace impl \
    source_files.txt
```

### Error behaviour

- **Unsupported file extension**: the file is skipped with a warning; the run
  continues.
- **Unreadable or missing source file**: an error is logged and the file is
  skipped; the run continues and exits with code 0.
- **Unreadable file-list file**: a fatal error is logged and the tool exits
  with code 1.

## Bazel integration

When using Bazel, use the `lobster_linker` rule from `lobster.bzl`:

```starlark
load("@lobster//:lobster.bzl", "lobster_linker", "lobster_test")

lobster_linker(
    name = "impl_trace",
    srcs = [
        "//src:my_library",
        "//src:my_binary",
    ],
    tracing_tags = ["req-traceability"],
)
```

The `lobster_linker` rule automatically collects all source files from the
listed targets and passes them to `lobster-bazel`. The output is exposed as a
`LobsterProvider` so it can be consumed by `lobster_test`.

The `subrule_lobster_linker` subrule is also exported for composing the linker
into other custom Bazel rules:

```starlark
load("@lobster//:lobster.bzl", "subrule_lobster_linker")
```
