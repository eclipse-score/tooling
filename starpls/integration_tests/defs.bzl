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
load("@rules_python//python:defs.bzl", "py_test")

def starpls_py_integration_test(name, srcs, data, **kwargs):
    """Creates a py_test target configured for starpls integration."""
    py_test(
        name = name,
        srcs = srcs,
        deps = ["@pip_deps_test//bazel_runfiles:bazel_runfiles"],
        data = data,
        python_version = "PY3",
        size = kwargs.pop("size", "small"),
        **kwargs
    )
