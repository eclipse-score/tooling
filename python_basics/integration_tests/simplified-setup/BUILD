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

load("@score_python_basics//:defs.bzl", "score_py_pytest")
load("@pip_test//:requirements.bzl", "requirement")
load("@rules_python//python:defs.bzl", "py_binary")

# Test that Python toolchain is available (provided by score_python_basics)
py_binary(
    name = "test_python_toolchain",
    srcs = ["test_python_toolchain.py"],
)

# Test that pip dependencies work with the simplified setup
score_py_pytest(
    name = "test_simplified_setup",
    srcs = ["test_simplified_setup.py"],
    deps = [
        requirement("requests"),
    ],
)