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
"""Macro declaring an L0 tool test that is run by pytest."""

load("@pip_tooling//:requirements.bzl", "requirement")
load("@rules_python//python:defs.bzl", "py_test")

_MAIN = Label("//bazel/rules/rules_score/src/tests:pytest_main.py")

def pytest_test(name, src, deps = [], size = "small", **kwargs):
    """Declares a `py_test` that runs pytest on one test file.

    Args:
        name: Test target name.
        src: The test file.
        deps: Dependencies of the test; pytest is added.
        size: Test size.
        **kwargs: Forwarded to `py_test` (`data`, `env`, ...).
    """
    py_test(
        name = name,
        size = size,
        srcs = [src, _MAIN],
        main = _MAIN,
        args = ["$(location {})".format(src)],
        deps = [requirement("pytest")] + deps,
        **kwargs
    )
