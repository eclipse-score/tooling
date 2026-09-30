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
    tags = ["manual"],  # intentionally fails to compile: event_control.h is never checked in
)

# allow_parse_errors = True so the CppAnalyze action tolerates the
# unresolved #include below (event_control.h is intentionally never
# checked in, to simulate a header that fails to resolve) without having to
# downgrade this unit's maturity -- validation findings must still be
# treated as errors, since this case is specifically checking that the
# recovered type is NOT flagged as a mismatch.
unit(
    name = "unit",
    testonly = False,
    allow_parse_errors = True,
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
