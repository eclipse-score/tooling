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

# pylint isn't provided by rules_multitool (it's a pure-python package, not a
# static binary), so it's set up "in userland": declared as a pip dependency
# in python_basics/requirements.in and exposed here as a runnable binary.
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
