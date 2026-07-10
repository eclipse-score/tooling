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

load("@bazel_skylib//rules:build_test.bzl", "build_test")
load("@rules_python//python:pip.bzl", "compile_pip_requirements")

# In order to update the requirements, change the `requirements.txt` file and run:
# `bazel run //:requirements_3_XX.update  --@@rules_python+//python/config_settings:python_version=3.XX`.
# This will update the `requirements_lock.txt` file.
# To upgrade all dependencies to their latest versions, run:
# `bazel run //:requirements_3_XX.update  --@@rules_python+//python/config_settings:python_version=3.XX -- --upgrade`.
[
    compile_pip_requirements(
        name = "requirements_3_{}".format(version),
        src = "//lobster_bazel:requirements.in",
        python_version = "3.{}".format(version),
        requirements_txt = "//lobster_bazel:requirements_3_{}.txt".format(version),
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
