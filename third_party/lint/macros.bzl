# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************

"""Reusable lint aspect factories for score_tooling consumers.

Python linting (ruff, pylint, ty) is wired up as Bazel aspects on top of
aspect_rules_lint, following the same model used elsewhere in score (see
bmw-software-engineering-trlc's `third_party/lint/linters.bzl`):

Declare the aspects once, at the top level of your own repo's
`linters.bzl` (so that `Label("//...")` config-file references resolve
relative to your repo, not to score_tooling):

    load("@score_tooling//third_party/lint:macros.bzl", "pylint_lint_aspect", "ruff_lint_aspect", "ty_lint_aspect")

    ruff = ruff_lint_aspect(config = Label("//:pyproject.toml"))

    pylint = pylint_lint_aspect(
        binary = Label("//third_party/lint:pylint"),
        config = Label("//:pyproject.toml"),
    )

    ty = ty_lint_aspect(config = Label("//:pyproject.toml"))

Then run/check them with plain `bazel build --aspects=...`, e.g. via a
`.bazelrc` config:

    build:lint --aspects=//third_party/lint:linters.bzl%ruff,//third_party/lint:linters.bzl%pylint,//third_party/lint:linters.bzl%ty
    build:lint --output_groups=+rules_lint_human

    bazel build --config=lint //...

Rust linting (clippy) doesn't need any of this: rules_rust (already a
score_tooling dependency) ships native `rust_clippy_test`, wired up directly
in the consuming repo's own BUILD files -- no aspect wrapper needed.
"""

load("@aspect_rules_lint//lint:pylint.bzl", "lint_pylint_aspect")
load("@aspect_rules_lint//lint:ruff.bzl", "lint_ruff_aspect")
load("@aspect_rules_lint//lint:ty.bzl", "lint_ty_aspect")

def ruff_lint_aspect(config):
    """Creates a ruff lint aspect using aspect_rules_lint's built-in ruff binary.

    `@aspect_rules_lint//lint:ruff_bin` is just an alias for `@multitool//tools/ruff`
    -- the same default multitool hub that aspect_rules_lint itself populates
    (score_tooling doesn't declare or `use_repo` it) and that third_party/format's
    `@aspect_rules_lint//format:ruff` also aliases into -- so linting and
    formatting always run the exact same ruff version.

    Args:
        config: label of the caller's ruff config file (pyproject.toml, ruff.toml, or .ruff.toml).

    Returns:
        An aspect, to be assigned to a module-level variable and passed to
        `lint_test()` or referenced via `--aspects=<label>%<name>`.
    """
    return lint_ruff_aspect(
        binary = Label("@aspect_rules_lint//lint:ruff_bin"),
        configs = [config],
    )

def pylint_lint_aspect(binary, config):
    """Creates a pylint lint aspect.

    Pylint is not provided by multitool (it's a pure-python package, not a
    static binary), so the caller must set it up "in userland": add pylint to
    their pip requirements and declare a `py_console_script_binary` target for
    it (see `//third_party/lint:pylint` for a ready-to-use one, or roll your own
    following https://github.com/aspect-build/rules_lint/blob/main/lint/pylint.bzl).

    Args:
        binary: label of a py_console_script_binary target for pylint.
        config: label of the caller's pylint config file (pyproject.toml, .pylintrc, or setup.cfg).

    Returns:
        An aspect, to be assigned to a module-level variable and passed to
        `lint_test()` or referenced via `--aspects=<label>%<name>`.
    """
    return lint_pylint_aspect(binary = binary, config = config)

def ty_lint_aspect(config):
    """Creates a ty lint (type-checking) aspect using aspect_rules_lint's built-in ty binary.

    `@aspect_rules_lint//lint:ty_bin` is just an alias for `@multitool//tools/ty`
    -- the same default multitool hub used above for ruff (populated by
    aspect_rules_lint itself, not by score_tooling) -- so no extra pip
    dependency or userland binary target is required.

    Args:
        config: label of the caller's ty config file (pyproject.toml or ty.toml).

    Returns:
        An aspect, to be assigned to a module-level variable and passed to
        `lint_test()` or referenced via `--aspects=<label>%<name>`.
    """
    return lint_ty_aspect(
        binary = Label("@aspect_rules_lint//lint:ty_bin"),
        config = config,
    )
