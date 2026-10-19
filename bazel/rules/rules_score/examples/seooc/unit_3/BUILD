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
load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "unit",
)

unit(
    name = "unit_3",
    scope = ["//unit_3:__subpackages__"],
    tests = [
        ":unit_3_test",
    ],
    unit_design = [
        "//unit_3/docs:unit_design",
    ],
    visibility = ["//visibility:public"],
    implementation = [
        ":unit_3_lib",
    ],
)

cc_library(
    name = "unit_3_lib",
    srcs = ["formatter.cpp"],
    hdrs = ["formatter.h"],
    visibility = ["//visibility:public"],
    deps = [
        "//unit_1:unit_1_lib",
    ],
)

cc_test(
    name = "unit_3_test",
    srcs = ["formatter_test.cpp"],
    visibility = ["//visibility:public"],
    deps = [
        ":unit_3_lib",
        "@googletest//:gtest_main",
    ],
)
