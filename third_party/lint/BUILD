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

load("@rules_python//python/entry_points:py_console_script_binary.bzl", "py_console_script_binary")

package(default_visibility = ["//visibility:public"])

exports_files(["macros.bzl"])

# Optional pylint binary for consumers; not used by this repo's own lint run (ruff PL rules cover it).
py_console_script_binary(
    name = "pylint",
    pkg = "@pip_tooling//pylint:pkg",
    deps = [
        "@pip_tooling//astroid:pkg",
        "@pip_tooling//dill:pkg",
        "@pip_tooling//isort:pkg",
        "@pip_tooling//mccabe:pkg",
        "@pip_tooling//platformdirs:pkg",
        "@pip_tooling//tomlkit:pkg",
    ],
)
