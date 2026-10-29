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
load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "test_specification",
    "unit",
)

unit(
    name = "unit_1",
    #    scope = ["//bazel/rules/rules_score/examples/seooc/unit_1:__pkg__"],
    #    scope = ["//bazel/rules/rules_score/examples/seooc:__subpackages__"],
    scope = ["//unit_1:unit_1_lib"],
    test_specs = [
        ":get_number_spec",
    ],
    tests = [
        ":unit_1_test",
    ],
    unit_design = [
        "//unit_1/docs:unit_design",
    ],
    visibility = ["//visibility:public"],
    implementation = [
        ":unit_1_lib",
    ],
)

# Formal test specification, referenced by a test case via
# RecordProperty("test-spec", "//unit_1:get_number_spec").
test_specification(
    name = "get_number_spec",
    src = "get_number_spec.rst",
)

cc_library(
    name = "unit_1_lib",
    srcs = ["foo.cpp"],
    hdrs = ["foo.h"],
    visibility = ["//visibility:public"],
    deps = ["@some_other_library"],
)

cc_test(
    name = "unit_1_test",
    srcs = ["foo_test.cpp"],
    visibility = ["//visibility:public"],
    deps = [
        ":unit_1_lib",
        "@googletest//:gtest_main",
    ],
)
