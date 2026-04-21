<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

- ✅ Centralized logic without Bazel module extensions

---

## Directory Structure

```bash
├── BUILD.bazel
├── macros.bzl
└── README.md
```

---

## Key Files

### `macros.bzl`

Defines the main macro to use in projects:

```python
def use_format_targets(fix_name = "format.fix", check_name = "format.check"):
    ...
```

This sets up:

- `format.fix` — a multi-run rule that applies formatting tools
- `format.check` — a test rule that checks formatting

### `MODULE.bazel`

Declares this module and includes required dependencies:

```python
module(name = "score_format_checker", version = "0.1.1")

bazel_dep(name = "aspect_rules_lint", version = "1.0.3")
bazel_dep(name = "buildifier_prebuilt", version = "7.3.1")
bazel_dep(name = "score_rust_policies", version = "0.0.2")
```

---

## Usage

### 1️⃣ Declare the dependency in your project’s `MODULE.bazel`:

```python
bazel_dep(name = "score_format_checker", version = "0.1.1")

# If using local source:
local_path_override(
    module_name = "score_format_checker",
    path = "../tooling/format",
)

# Explicit dependencies required by the macro
bazel_dep(name = "aspect_rules_lint", version = "1.0.3")
bazel_dep(name = "buildifier_prebuilt", version = "7.3.1")
bazel_dep(name = "score_rust_policies", version = "0.0.2")
```

### 2️⃣ In your project’s `BUILD.bazel`:

```python
load("@score_format_checker//:macros.bzl", "use_format_targets")

use_format_targets()
```

This will register two Bazel targets:

- `bazel run //:format.fix` — fixes format issues
- `bazel test //:format.check` — fails on unformatted files

### 3️⃣ In VS Code settings:

⚠️ First formatting run can be slow!

Add the following entry to `.vscode/settings.json`:

```json
"rust-analyzer.rustfmt.overrideCommand": [
    "${workspaceFolder}/.vscode/rustfmt.sh"
]
```

Add `.vscode/rustfmt.sh` file with `+x` permissions:

```bash
#!/usr/bin/env bash

bazel run @score_tooling//format_checker:rustfmt_with_policies
```

---

## Rust support

- Default formatter label: `@score_tooling//format_checker:rustfmt_with_policies`, which wraps the
  upstream `rustfmt` binary with the shared `rustfmt.toml` policies from `score_rust_policies`.

---

## Benefits

✅ Centralized formatting config with local file scope
✅ Consistent developer experience across repositories
✅ Easily pluggable in CI pipelines or Git pre-commit hooks

---
