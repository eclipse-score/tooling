# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************

load("//bazel/rules/rules_score:rules_score.bzl", "unit", "unit_design")
load("//validation/core/integration_test:puml_fixture.bzl", "provider_fbs_fixture_bundle")

unit_design(
    name = "unit_design",
    static = ["class_diagram.puml"],
    visibility = ["//visibility:private"],
)

cc_library(
    name = "transport",
    srcs = ["transport.cpp"],
)

unit(
    name = "unit",
    testonly = False,
    maturity = "development",
    tests = [],
    unit_design = [":unit_design"],
    visibility = ["//visibility:private"],
    implementation = [":transport"],
)

provider_fbs_fixture_bundle(
    name = "fbs",
    visibility = ["//visibility:private"],
    deps = [
        ":unit",
        ":unit_design",
    ],
)

filegroup(
    name = "case_data",
    srcs = [
        "expected.yaml",
        ":fbs",
    ],
    visibility = ["//validation/core/integration_test:__subpackages__"],
)
