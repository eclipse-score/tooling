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

compile_pip_requirements(
    name = "requirements",
    src = "//lobster_bazel:requirements.in",
    requirements_txt = "//lobster_bazel:requirements.txt",
    tags = [
        "manual",
    ],
)

build_test(
    name = "ensure_all_private_targets_work",
    targets = [
        ":requirements.update",
    ],
)
