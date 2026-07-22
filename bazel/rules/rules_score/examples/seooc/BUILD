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
    "component",
    "dependability_analysis",
    "dependable_element",
)

cc_library(
    name = "sample_library",
    visibility = ["//visibility:public"],
    deps = [
        "//unit_1:unit_1_lib",
        "//unit_2:unit_2_lib",
    ],
)

component(
    name = "component_example",
    components = [
        "//unit_1:unit_1",
        "//unit_2:unit_2",
        ":sub_component_example",
    ],
    # component_requirements_sub is also listed here (in addition to
    # sub_component_example's own requirements below) so its CompReq item is
    # visible in the dependable_element-level traceability report -- nested
    # components' own `requirements` are not rolled up automatically into
    # their parent's Component Requirements tier (see test/BUILD's
    # test_nested_component/test_parent_component fixture for the same
    # pattern).
    requirements = [
        "//docs/requirements:component_requirements",
        "//docs/requirements:component_requirements_extra",
        "//docs/requirements:component_requirements_sub",
    ],
    tags = ["manual"],
    tests = [],
)

# Nested component: components can themselves contain further components,
# not just units (see docs/user_guide/architectural_design.rst).
component(
    name = "sub_component_example",
    requirements = [
        "//docs/requirements:component_requirements_sub",
    ],
    tags = ["manual"],
    tests = [],
)

dependability_analysis(
    name = "sample_dependability_analysis",
    arch_design = "//design:sample_seooc_design",
    fmea = ["//safety_analysis:sample_fmea"],
)

dependable_element(
    name = "safety_software_seooc_example",
    # Chain-forward received AoUs that this element cannot handle itself.
    # TimingConstraint from other_seooc is forwarded to dependees (integrator).
    aou_forwarding = "aou_forwarding.yaml",
    architectural_design = ["//design:sample_seooc_design"],
    assumptions_of_use = ["//docs:sample_aous"],
    components = [":component_example"],
    dependability_analysis = [
        ":sample_dependability_analysis",
    ],
    glossary = ["//docs:glossary"],
    integrity_level = "B",
    requirements = [
        "//docs/requirements:feature_requirements",
    ],
    tests = [],
    # Public: the integrator example depends on this element to demonstrate
    # AoU forwarding across module boundaries (see the comment above).
    # Cross-repo visibility specs can't reference a dependent module's
    # apparent name without introducing a circular bazel_dep, so this is
    # public rather than scoped to "@integrator".
    visibility = ["//visibility:public"],
    deps = ["@some_other_library//:other_seooc"],
)
