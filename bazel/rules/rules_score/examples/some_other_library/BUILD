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
    "architectural_design",
    "assumptions_of_use",
    "component",
    "dependable_element",
    "unit",
)

architectural_design(
    name = "sample_seooc_design",
    static = [
        "static_design.puml",
    ],
)

assumptions_of_use(
    name = "other_library_aous",
    srcs = ["aous.trlc"],
)

unit(
    name = "abc",
    scope = ["//:__pkg__"],
    tests = [],
    unit_design = [],
    implementation = [
        ":some_other_library",
    ],
)

component(
    name = "component_example",
    components = [
        ":abc",
    ],
    requirements = [],  # TODO: define own requirements (removed circular dep on @seooc)
    tags = ["manual"],
    tests = [],
)

dependable_element(
    name = "other_seooc",
    architectural_design = [":sample_seooc_design"],
    assumptions_of_use = [":other_library_aous"],
    components = [":component_example"],
    dependability_analysis = [],
    docs_prefix = "bazel/rules/rules_score/docs/sphinx/",
    integrity_level = "D",
    requirements = [],
    tests = [],
    visibility = ["//visibility:public"],
)

cc_library(
    name = "some_other_library",
    srcs = [],
    visibility = ["//visibility:public"],
    deps = [],
)
