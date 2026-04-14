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
load("@rules_python//python:defs.bzl", "py_library", "py_test")
load("//lobster_bazel:lobster_bazel.bzl", "lobster_linker")

# BUILD.bazel
# gazelle:exclude __init__.py

py_library(
    name = "test_runner",
    srcs = [
        "__init__.py",
        "test_runner.py",
    ],
    imports = ["."],
)

py_test(
    name = "test_basic_tracing",
    srcs = ["test_basic_tracing.py"],
    data = [
        "//lobster_bazel:lobster-bazel",
        "//lobster_bazel/test/data:bazel_data_system",
    ],
    deps = [
        ":test_runner",
    ],
)

lobster_linker(
    name = "impl_trace",
    srcs = [
        "//lobster_bazel/test/data:cpp_source_with_tags",
        "//lobster_bazel/test/data:py_source_with_tags",
    ],
    tracing_tags = ["req-traceability"],
)

py_test(
    name = "test_bazel_rule",
    srcs = ["test_bazel_rule.py"],
    data = [
        ":impl_trace",
    ],
    deps = [
        ":test_runner",
    ],
)
