<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

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
| **format** | Code formatting validation | [README](third_party/format/README.md) |
| **lint** | Python lint aspects (ruff, pylint) | [README](third_party/lint/README.md) |
| **python_basics** | Python development utilities and testing | [README](python_basics/README.md) |
| **starpls** | Starlark language server support | [README](starpls/README.md) |
| **tools** | Formatters & Linters | [README](tools/README.md) |
| **coverage** | Unified LLVM source-based coverage (C++ + Rust) | [README](coverage/README.md) |

## Coverage

The `coverage/` module provides the reusable LLVM source-based coverage pipeline
used across S-CORE repositories: one report covering C++ and Rust (line + branch),
exact 0% entries for untested in-scope files, a justification system
(`COV_JUSTIFIED` markers + YAML), and effective-coverage gating. See the
[adoption guide](coverage/README.md) and the
[mechanism deep-dive](coverage/COVERAGE_GUIDE.md).

> **Breaking change:** the former Ferrocene `symbol-report`/`blanket` workflow
> (`rust_coverage_report`, `//coverage:ferrocene_report`) was removed. The LLVM
> pipeline replaces it with unified C++ + Rust reports; see the
> [adoption guide](coverage/README.md) for migration.

Generate a combined Rust + Python HTML coverage report for this repository's own
tools (`plantuml`, `validation`, `manual_analysis`):

```bash
bazel run //coverage:combined_report
```

## Usage Examples

Load tools in your `BUILD` files:

```starlark
load("@score_tooling//:defs.bzl", "score_py_pytest")
load("@score_tooling//:defs.bzl", "cli_tool")
load("@score_tooling//coverage:defs.bzl", "score_coverage_reporter", "score_coverage_scope")
```

Declare the coverage scope and reporter for your repository (see the
[coverage adoption guide](coverage/README.md) for the full setup, including the
required `.bazelrc` configuration and toolchains):

```starlark
score_coverage_scope(
    name = "coverage_scope",
    testonly = True,
    deps = ["//src/mylib"],
)

score_coverage_reporter(
    name = "reporter_wrapper",
    testonly = True,
    coverage_scope = ":coverage_scope",
    llvm_cov = "@llvm_toolchain//:llvm-cov",
    llvm_profdata = "@llvm_toolchain//:llvm-profdata",
    llvm_cxxfilt = "@llvm_toolchain_llvm//:bin/llvm-cxxfilt",
)
```

## Upgrading from separate MODULES

If you are still using separate module imports and want to upgrade to the new version.
Here are two examples to showcase how to do this.

```
load("@score_python_basics//:defs.bzl", "score_py_pytest") => load("@score_tooling//:defs.bzl", "score_py_pytest")
load("@score_cr_checker//:cr_checker.bzl", "copyright_checker") => load("@score_tooling//cr_checker:cr_checker.bzl", "copyright_checker")
```

All things inside of 'tooling' can now be imported from `@score_tooling//:defs.bzl`.
The available import targets are:

- score_virtualenv
- score_py_pytest
- dash_license_checker
- cli_helper
- setup_starpls
- score_coverage_scope
- score_coverage_reporter

Formatting, linting, and cr_checker are no longer re-exported from `defs.bzl`; use
`@score_tooling//third_party/format:macros.bzl`, `@score_tooling//third_party/lint:macros.bzl`,
`@score_tooling//cr_checker:cr_checker.bzl`, or the `@score_tooling//third_party/format:rustfmt_with_policies`
label directly.

## Format the tooling repository

```bash
bazel run //:format.fix
```

## Optional: private Ubuntu snapshot mirror

`snapshot.ubuntu.com` (used by the apt manifests in `third_party/`) is sometimes unreliable.
With access to a snapshot mirror such as [StableBuild](https://stablebuild.com), Bazel can try it
first and keep the public URL as fallback.

1. Create `user.downloader_config` in the repository root (gitignored; replace `<private-mirror-tag>` with your StableBuild tag):

   ```text
   rewrite snapshot.ubuntu.com/ubuntu/[^/]*/(.*) <private-mirror-tag>.debmirror.stablebuild.com/2026-04-01T10:40:01Z/archive.ubuntu.com/ubuntu/$1
   rewrite snapshot.ubuntu.com/ubuntu/[^/]*/(.*) <private-mirror-tag>.debmirror.stablebuild.com/2026-04-01T10:40:01Z/ports.ubuntu.com/$1
   rewrite snapshot.ubuntu.com/ubuntu/(.*) snapshot.ubuntu.com/ubuntu/$1
   ```

2. Add to `user.bazelrc` (gitignored) in the root and in each nested module that fetches apt packages
   (e.g. `bazel/rules/rules_score/examples/*`), using the absolute path:

   ```text
   common --downloader_config=/abs/path/to/repo/user.downloader_config
   ```

3. Run `bazel shutdown`.

The date in the rewrite must match the snapshot timestamp in the manifests.
In CI the same config is generated by `.github/actions/configure_ubuntu_snapshot_mirror` from the
`UBUNTU_SNAPSHOT_MIRROR_URL` secret.
