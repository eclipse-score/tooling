# Score Tooling

A unified Bazel module containing development tools and utilities for building, testing, and maintaining code quality.

## Quick Start

Add this module to your `MODULE.bazel`:

```starlark
bazel_dep(name = "score_tooling", version = "1.0.0")
```

## Available Tools

Each tool maintains its own documentation and examples in their respective subdirectories.
See the individual README files for detailed usage instructions and configuration options.

| Tool | Description | Documentation |
|------|-------------|---------------|
| **cli_helper** | Command-line interface utilities | [README](cli_helper/README.md) |
| **cr_checker** | Code review and compliance checking | [README](cr_checker/README.md) |
| **dash** | Eclipse Dash license scanning | [README](dash/README.md) |
| **format_checker** | Code formatting validation | [README](format_checker/README.md) |
| **python_basics** | Python development utilities and testing | [README](python_basics/README.md) |
| **starpls** | Starlark language server support | [README](starpls/README.md) |
| **tools** | Formatters & Linters | [README](tools/README.md) |

## Usage Examples

Load tools in your `BUILD` files:

```starlark
load("@score_tooling//:defs.bzl", "score_py_pytest")
load("@score_tooling//:defs.bzl", "cli_tool")
```

## Upgrading from separate MODULES

If you are still using separate module imports and want to upgrade to the new version. 
Here are two examples to showcase how to do this.

```
load("@score_python_basics//:defs.bzl", "score_py_pytest") => load("@score_tooling//:defs.bzl", "score_py_pytest")
load("@score_cr_checker//:cr_checker.bzl", "copyright_checker") => load("@score_tooling//:defs.bzl", "copyright_checker")
```
All things inside of 'tooling' can now be imported from `@score_tooling//:defs.bzl`. 
The available import targets are:

- score_virtualenv
- score_py_pytest
- dash_license_checker
- copyright_checker
- cli_helper
- use_format_targets
- setup_starpls
