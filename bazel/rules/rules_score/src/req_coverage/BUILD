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

load("@pip_lobster_bazel//:requirements.bzl", lobster_requirement = "requirement")
load("@pip_tooling//:requirements.bzl", tooling_requirement = "requirement")
load("@rules_python//python:defs.bzl", "py_binary", "py_library", "py_test")

# ---------------------------------------------------------------------------
# Core libraries
# ---------------------------------------------------------------------------

py_library(
    name = "read_gtest_lobster",
    srcs = ["read_gtest_lobster.py"],
    imports = [".."],
    visibility = ["//visibility:public"],
    deps = [
        ":compute_lock",
        "@lobster//lobster/common",
    ],
)

py_library(
    name = "compute_lock",
    srcs = ["compute_lock.py"],
    imports = [".."],
    visibility = ["//visibility:public"],
    deps = [lobster_requirement("pyyaml")],
)

py_library(
    name = "check_lock",
    srcs = ["check_lock.py"],
    imports = [".."],
    visibility = ["//visibility:public"],
    deps = [":compute_lock"],
)

py_library(
    name = "lobster_generator",
    srcs = ["lobster_generator.py"],
    imports = [".."],
    visibility = ["//visibility:public"],
    deps = [
        ":compute_lock",
        "@lobster//lobster/common",
    ],
)

py_library(
    name = "update_runner_lib",
    srcs = ["update_runner.py"],
    imports = [".."],
    visibility = ["//visibility:public"],
    deps = [
        ":check_lock",
        ":compute_lock",
        ":read_gtest_lobster",
    ],
)

py_binary(
    name = "update_runner",
    srcs = ["update_runner.py"],
    imports = [".."],
    main = "update_runner.py",
    visibility = ["//visibility:public"],
    deps = [":update_runner_lib"],
)

py_library(
    name = "test_runner_lib",
    srcs = ["test_runner.py"],
    imports = [".."],
    visibility = ["//visibility:public"],
    deps = [
        ":check_lock",
        ":compute_lock",
        ":lobster_generator",
        ":read_gtest_lobster",
    ],
)

py_binary(
    name = "test_runner",
    srcs = ["test_runner.py"],
    imports = [".."],
    main = "test_runner.py",
    visibility = ["//visibility:public"],
    deps = [":test_runner_lib"],
)

# ---------------------------------------------------------------------------
# Unit tests
# ---------------------------------------------------------------------------

py_test(
    name = "read_gtest_lobster_test",
    srcs = ["read_gtest_lobster_test.py"],
    imports = [".."],
    deps = [
        ":read_gtest_lobster",
        tooling_requirement("pytest"),
    ],
)

py_test(
    name = "compute_lock_test",
    srcs = ["compute_lock_test.py"],
    imports = [".."],
    deps = [
        ":compute_lock",
        ":read_gtest_lobster",
        tooling_requirement("pytest"),
    ],
)

py_test(
    name = "check_lock_test",
    srcs = ["check_lock_test.py"],
    imports = [".."],
    deps = [
        ":check_lock",
        tooling_requirement("pytest"),
    ],
)

py_test(
    name = "lobster_generator_test",
    srcs = ["lobster_generator_test.py"],
    imports = [".."],
    deps = [
        ":lobster_generator",
        tooling_requirement("pytest"),
    ],
)

py_test(
    name = "update_runner_test",
    srcs = [
        "conftest.py",
        "update_runner_test.py",
    ],
    imports = [".."],
    deps = [
        ":update_runner_lib",
        tooling_requirement("pytest"),
    ],
)

py_test(
    name = "test_runner_test",
    srcs = [
        "conftest.py",
        "test_runner_test.py",
    ],
    imports = [".."],
    deps = [
        ":test_runner_lib",
        tooling_requirement("pytest"),
    ],
)
