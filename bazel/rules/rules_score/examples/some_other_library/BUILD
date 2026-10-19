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
    "dependability_analysis",
    "dependable_element",
    "failure_modes",
    "fault_trees",
    "safety_analysis",
    "unit",
)

architectural_design(
    name = "sample_seooc_design",
    public_api = [
        "public_api.puml",
    ],
    static = [
        "static_design.puml",
    ],
)

assumptions_of_use(
    name = "other_library_aous",
    srcs = ["aous.trlc"],
    visibility = ["//visibility:public"],
    deps = [":other_library_fault_trees"],
)

failure_modes(
    name = "other_library_failure_modes",
    srcs = ["failure_modes.trlc"],
)

fault_trees(
    name = "other_library_fault_trees",
    srcs = ["fta.puml"],
    fta_package = "other_library_fta",
    deps = [":other_library_failure_modes"],
)

safety_analysis(
    name = "other_library_safety_analysis",
    arch_design = ":sample_seooc_design",
    failure_modes = [":other_library_failure_modes"],
    fault_trees = ":other_library_fault_trees",
    safety_measures = [":other_library_aous"],
)

dependability_analysis(
    name = "other_library_dependability_analysis",
    arch_design = ":sample_seooc_design",
    safety_analysis = [":other_library_safety_analysis"],
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
    components = [":component_example"],
    dependability_analysis = [":other_library_dependability_analysis"],
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
