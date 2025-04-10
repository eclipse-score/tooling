# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@aspect_rules_py//py:defs.bzl", "py_binary")
load("//:defs.bzl", "score_python")
load("@aspect_rules_lint//format:defs.bzl", "format_multirun", "format_test")
package(default_visibility = ["//visibility:public"])


score_python()

format_multirun(
    name = "format.fix",
    python = "//:ruff",
    visibility = [
        "//visibility:public",
    ],
)

format_multirun(
    name = "format_aspect_rules.fix",
    python = "@aspect_rules_lint//format:ruff",
    visibility = [
        "//visibility:public",
    ],
)
format_test(
    name = "format_aspect_rules.check",
    no_sandbox = True,
    python = "@aspect_rules_lint//format:ruff",
    visibility = [
        "//visibility:public",
    ],
    workspace = "//:MODULE.bazel",
)
#
format_test(
    name = "format.check",
    no_sandbox = True,
    python = "//:ruff",
    visibility = [
        "//visibility:public",
    ],
    workspace = "//:MODULE.bazel",
)


filegroup(
    name="conf",
    srcs=["pyproject.toml"],
)

py_binary(
    name="t",
    srcs=["t.py"],
)

# Exporting req. lock file
exports_files([
    "requirements.txt",
    "pyproject.toml"
])


# In order to update the requirements, change the `requirements.txt` file and run:
# `bazel run //:requirements.update`.
# This will update the `requirements_lock.txt` file.
# To upgrade all dependencies to their latest versions, run:
# `bazel run //:requirements.update -- --upgrade`.
compile_pip_requirements(
    name = "requirements",
    src = "requirements.in",
    requirements_txt = "requirements.txt",
    tags = [
        "manual",
    ],
)
