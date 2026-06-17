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
    "architectural_design",
)
load("//validation/ai_checker:ai_checker.bzl", "architecture_ai_test")

architectural_design(
    name = "sample_seooc_design",
    dynamic = [
        "dynamic_design.puml",
    ],
    public_api = [
        "public_api.puml",
    ],
    static = [
        "static_design.puml",
        "arch_design.rst",
    ],
    visibility = ["//visibility:public"],
)

# Background context (read-only) for the AI architecture review.
filegroup(
    name = "design_context",
    srcs = ["design_context.md"],
)

architecture_ai_test(
    name = "sample_seooc_design_ai_test",
    context = ":design_context",
    designs = [":sample_seooc_design"],
    tags = ["manual"],
)
