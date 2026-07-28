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

"""score_tooling's own lint aspect declarations (dogfooding //third_party/lint:macros.bzl).

Run with:

    bazel build --config=lint //...
"""

load("//third_party/lint:macros.bzl", "pylint_lint_aspect", "ruff_lint_aspect", "ty_lint_aspect")

ruff = ruff_lint_aspect(config = Label("//:pyproject.toml"))

pylint = pylint_lint_aspect(
    binary = Label("//third_party/lint:pylint"),
    config = Label("//:pyproject.toml"),
)

ty = ty_lint_aspect(config = Label("//:pyproject.toml"))
