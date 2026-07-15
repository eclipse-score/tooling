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
load("@pip_tooling//:requirements.bzl", "all_requirements", "requirement")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")
load("@score_tooling//python_basics:defs.bzl", "score_virtualenv")

# define ":ruff" target
#score_python()

# Generate the virtualenv
score_virtualenv()

# In order to update the requirements, change the `requirements.txt` file and run:
# `bazel run //:requirements_3_XX.update  --@@rules_python+//python/config_settings:python_version=3.XX`.
# This will update the `requirements_lock.txt` file.
# To upgrade all dependencies to their latest versions, run:
# `bazel run //:requirements_3_XX.update  --@@rules_python+//python/config_settings:python_version=3.XX -- --upgrade`.
[
    compile_pip_requirements(
        name = "requirements_3_{}".format(version),
        src = "//python_basics:requirements.in",
        python_version = "3.{}".format(version),
        requirements_txt = "//python_basics:requirements_3_{}.txt".format(version),
        tags = [
            "manual",
        ],
    )
    for version in [
        "11",
        "12",
    ]
]

build_test(
    name = "ensure_all_private_targets_work",
    targets = [
        ":requirements.update",
    ],
)
