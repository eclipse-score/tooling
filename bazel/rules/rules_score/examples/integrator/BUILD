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
#
# Example: System integrator SEooC that depends on safety_software_seooc_example.
#
# This illustrates the full AoU forwarding chain, and the two ways a
# dependable element can cover an AoU it receives:
#
#   other_seooc               defines AoU: OtherLibrary.TimingConstraint
#       ↑ (deps)
#   safety_software_seooc_example
#       - defines own AoU: SampleType.SampleAoU  (auto-forwarded here)
#       - chain-forwards OtherLibrary.TimingConstraint via aou_forwarding.yaml
#       ↑ (deps)
#   integrator_seooc (this target)
#       - receives SampleType.SampleAoU          (auto-forwarded from seooc)
#       - receives OtherLibrary.TimingConstraint  (chain-forwarded through seooc)
#       - reviews both received AoUs directly with manual analysis
#

load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "architectural_design",
    "component",
    "dependable_element",
    "unit",
)
load("@score_tooling//manual_analysis:context_from_cc_library.bzl", "manual_analysis_context_from_cc_library")
load("@score_tooling//manual_analysis:context_from_filegroup.bzl", "manual_analysis_context_from_filegroup")

cc_library(
    name = "integrator_lib",
    srcs = [],
    visibility = ["//visibility:public"],
    deps = [
        "@seooc//:sample_library",
    ],
)

manual_analysis_context_from_cc_library(
    name = "implementation_review_context",
    library = ":integrator_lib",
    visibility = ["//manual_analysis:__subpackages__"],
)

filegroup(
    name = "integration_review_inputs",
    srcs = [
        "BUILD",
        "static_design.puml",
        "//docs/requirements:component_requirements",
        "@seooc//docs:sample_aous",
        "@some_other_library//:other_library_aous",
    ],
)

manual_analysis_context_from_filegroup(
    name = "integration_review_context",
    filegroup = ":integration_review_inputs",
    visibility = ["//manual_analysis:__subpackages__"],
)

architectural_design(
    name = "integrator_design",
    static = ["static_design.puml"],
)

unit(
    name = "integrator_unit",
    scope = ["@seooc//:sample_library"],
    tests = [],
    unit_design = [],
    implementation = [":integrator_lib"],
)

component(
    name = "integrator_component",
    components = [":integrator_unit"],
    requirements = [
        "//docs/requirements:component_requirements",
    ],
    tags = ["manual"],
    tests = [],
)

# The integrator depends on safety_software_seooc_example and therefore
# receives all forwarded AoUs:
#   - SampleType.SampleAoU            (auto-forwarded, own AoU of seooc)
#   - OtherLibrary.TimingConstraint   (chain-forwarded from other_seooc through seooc)

dependable_element(
    name = "integrator_seooc",
    architectural_design = [":integrator_design"],
    components = [":integrator_component"],
    dependability_analysis = [],
    integrity_level = "B",
    manual_analysis = [
        "//manual_analysis/received_shared_memory:review",
        "//manual_analysis/received_timing:review",
    ],
    maturity = "development",
    requirements = [
        "//docs/requirements:feature_requirements",
    ],
    tests = [],
    deps = [
        "@seooc//:safety_software_seooc_example",
    ],
)
