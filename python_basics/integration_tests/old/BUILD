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
load("@bazel_skylib//rules:build_test.bzl", "build_test")
load("@pip_score_venv_test//:requirements.bzl", "all_requirements")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@score_python_basics//:defs.bzl", "score_virtualenv")
load("@aspect_rules_lint//format:defs.bzl", "format_multirun", "format_test")
#package(default_visibility = ["//visibility:public"])

# define ":ruff" target
#score_python()

# Generate the virtualenv
score_virtualenv()

format_multirun(
    name = "format.fix",
    python = "@aspect_rules_lint//format:ruff",
    visibility = [
        "//visibility:private",
    ],
)

format_test(
    name = "format.check",
    no_sandbox= True,
    python = "@aspect_rules_lint//format:ruff",
    workspace = "@score_python_basics//:MODULE.bazel",
    visibility = [
        "//visibility:private",
    ],
)