
# Source Format Targets for Bazel Projects

This module provides reusable Bazel macros for formatting **Starlark**, **Python** and **YAML** files 
using [`aspect-build/rules_lint`](https://github.com/aspect-build/rules_lint) and `buildifier`. It enables 
consistent formatting enforcement across Bazel-based projects.

---

## Features

- ✅ Supports Python (`ruff`), YAML (`yamlfmt`) and Starlark (`buildifier`)
- ✅ Provides `format.fix` and `format.check` targets
- ✅ Simple macro-based usage for local formatting scope
- ✅ Centralized logic without Bazel module extensions

---

## Directory Structure

```bash
├── BUILD.bazel               
├── MODULE.bazel              # Bazel module declaration with deps
├── macros.bzl                # Contains the reusable macro
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
```

### 2️⃣ In your project’s `BUILD.bazel`:

```python
load("@score_format_checker//:macros.bzl", "use_format_targets")

use_format_targets()
```

This will register two Bazel targets:

- `bazel run //:format.fix` — fixes format issues
- `bazel test //:format.check` — fails on unformatted files

---

## Benefits

✅ Centralized formatting config with local file scope  
✅ Consistent developer experience across repositories  
✅ Easily pluggable in CI pipelines or Git pre-commit hooks

---