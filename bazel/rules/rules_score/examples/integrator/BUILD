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
# This illustrates the full AoU forwarding chain:
#
#   other_seooc               defines AoU: OtherLibrary.TimingConstraint
#       ↑ (deps)
#   safety_software_seooc_example
#       - defines own AoU: SampleType.SampleAoU  (auto-forwarded here)
#       - chain-forwards OtherLibrary.TimingConstraint via aou_forwarding.yaml
#       ↑ (deps)
#   integrator_seooc (this target)
#       - receives SampleType.SampleAoU           (auto-forwarded from seooc)
#       - receives OtherLibrary.TimingConstraint   (chain-forwarded through seooc)
#       - must handle both in its lobster traceability report
#

load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "architectural_design",
    "component",
    "dependable_element",
    "unit",
)

cc_library(
    name = "integrator_lib",
    srcs = [],
    visibility = ["//visibility:public"],
    deps = [
        "@seooc//:sample_library",
    ],
)

architectural_design(
    name = "integrator_design",
    static = ["static_design.puml"],
)

unit(
    name = "integrator_unit",
    scope = ["//:__pkg__"],
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
    assumptions_of_use = [],
    components = [":integrator_component"],
    dependability_analysis = [],
    integrity_level = "B",
    requirements = [
        "//docs/requirements:feature_requirements",
    ],
    tests = [],
    deps = ["@seooc//:safety_software_seooc_example"],
)
