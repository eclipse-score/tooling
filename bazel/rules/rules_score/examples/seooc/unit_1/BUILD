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
    "//bazel/rules/rules_score:rules_score.bzl",
    "unit",
)

unit(
    name = "unit_1",
    #    scope = ["//bazel/rules/rules_score/examples/seooc/unit_1:__pkg__"],
    #    scope = ["//bazel/rules/rules_score/examples/seooc:__subpackages__"],
    scope = ["//bazel/rules/rules_score/examples/seooc/unit_1:unit_1_lib"],
    tests = [
        ":unit_1_test",
    ],
    unit_design = [
        "//bazel/rules/rules_score/examples/seooc/unit_1/docs:unit_design",
    ],
    visibility = ["//visibility:public"],
    implementation = [
        ":unit_1_lib",
    ],
)

cc_library(
    name = "unit_1_lib",
    srcs = ["foo.cpp"],
    hdrs = ["foo.h"],
    visibility = ["//visibility:public"],
    deps = ["//bazel/rules/rules_score/examples/some_other_library"],
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
