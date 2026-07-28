<!-- ----------------------------------------------------------------------------
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
----------------------------------------------------------------------------- -->

# lint

Reusable [aspect_rules_lint](https://github.com/aspect-build/rules_lint) aspect
factories for Python linting (ruff, pylint, ty), shared across score repos.

## Directory Structure

```bash
├── BUILD
├── linters.bzl
├── macros.bzl
└── README.md
```

## Key Files

### `macros.bzl`

- `ruff_lint_aspect(config)` — a ruff lint aspect using aspect_rules_lint's
  built-in ruff binary (`@aspect_rules_lint//lint:ruff_bin`, an alias for the
  same `@multitool//tools/ruff` that `third_party/format` uses for formatting, so
  linting and formatting always run the same ruff version).
- `pylint_lint_aspect(binary, config)` — a pylint lint aspect. Pylint is a
  pure-python package, not a static binary, so it isn't provided by
  `rules_multitool`: the caller must add it to their own pip requirements and
  declare a `py_console_script_binary` target for it (see `//third_party/lint:pylint`
  in this repo for a working example).
- `ty_lint_aspect(config)` — a [ty](https://docs.astral.sh/ty/) type-checking
  lint aspect using aspect_rules_lint's built-in ty binary
  (`@aspect_rules_lint//lint:ty_bin`, an alias for `@multitool//tools/ty`, the
  same default multitool hub used for ruff, so no extra pip dependency is
  required).

### `linters.bzl`

This repo's own dogfooding declarations (`ruff`, `pylint`, `ty` aspects), wired
up via `.bazelrc`'s `build:lint` config below. Consumers replicate this file
in their own repo (see Usage).

Rust linting (clippy) doesn't need anything from this module: `rules_rust`
(already a `score_tooling` dependency) ships native `rust_clippy_aspect` /
`rust_clippy_test` support directly.

## Usage

Aspects must be declared at the top level of your own repo's `linters.bzl`,
so that `Label("//...")` config-file references resolve relative to your
repo, not to `score_tooling`:

```python
load("@score_tooling//third_party/lint:macros.bzl", "pylint_lint_aspect", "ruff_lint_aspect", "ty_lint_aspect")

ruff = ruff_lint_aspect(config = Label("//:pyproject.toml"))

pylint = pylint_lint_aspect(
    binary = Label("//third_party/lint:pylint"),
    config = Label("//:pyproject.toml"),
)

ty = ty_lint_aspect(config = Label("//:pyproject.toml"))
```

Then run/check them via a `.bazelrc` config (see this repo's own `.bazelrc`
for a working example):

```
build:lint --aspects=//third_party/lint:linters.bzl%ruff,//third_party/lint:linters.bzl%pylint,//third_party/lint:linters.bzl%ty
build:lint --output_groups=+rules_lint_human
```

```bash
bazel build --config=lint //...
```
