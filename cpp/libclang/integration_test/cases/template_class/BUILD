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
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("//cpp/libclang:cpp_parser.bzl", "cpp_parser")

filegroup(
    name = "expected_output",
    srcs = ["expected.json"],
    visibility = ["//cpp/libclang/integration_test:__subpackages__"],
)

cc_library(
    name = "template_class",
    srcs = glob(["*.cpp"]),
    visibility = ["//cpp/libclang:__subpackages__"],
)

cpp_parser(
    name = "parser",
    extra_args = [
    ],
    target = ":template_class",
    tool = "//cpp/libclang:clang_rs_parser",
    visibility = ["//cpp/libclang/integration_test:__pkg__"],
)

rust_test(
    name = "test_libclang_parser",
    srcs = [
        "run_test.rs",
    ],
    data = [
        ":expected_output",
        ":parser",
    ],
    env = {
        "TEST_OUTPUT_PATH": "$(rootpath :parser)",
        "EXPECTED_OUTPUT_PATH": "$(rootpath :expected_output)",
    },
    visibility = ["//cpp/libclang:__pkg__"],
    deps = [
        "//cpp/libclang/integration_test:libclang_test_framework",
    ],
)
