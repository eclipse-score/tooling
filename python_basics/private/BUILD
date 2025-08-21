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
load("@aspect_rules_lint//format:defs.bzl", "format_multirun", "format_test")
load("@bazel_skylib//rules:build_test.bzl", "build_test")
load("@pip_tooling//:requirements.bzl", "all_requirements", "requirement")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@score_tooling//python_basics:defs.bzl", "score_virtualenv")

# define ":ruff" target
#score_python()

# Generate the virtualenv
score_virtualenv()

python_tool = "@aspect_rules_lint//format:ruff"

format_multirun(
    name = "format.fix",
    print_command = True,
    python = python_tool,
    visibility = [
        "//visibility:private",
    ],
)

# Ensure all files are formatted correctly
format_test(
    name = "format.check",

    # Scan the entire workspace.
    # This mode causes the test to be non-hermetic and it cannot be cached.
    no_sandbox = True,
    python = python_tool,
    visibility = [
        "//visibility:private",
    ],
    workspace = "//:MODULE.bazel",
)

# In order to update the requirements, change the `requirements.txt` file and run:
# `bazel run //:requirements.update`.
# This will update the `requirements_lock.txt` file.
# To upgrade all dependencies to their latest versions, run:
# `bazel run //:requirements.update -- --upgrade`.
compile_pip_requirements(
    name = "requirements",
    src = "//:requirements.in",
    requirements_txt = "//:requirements.txt",
    tags = [
        "manual",
    ],
)

build_test(
    name = "ensure_all_private_targets_work",
    targets = [
        ":format.fix",  # Ensure format.fix is executable
        ":requirements.update",
    ],
)
