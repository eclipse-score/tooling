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
    "component",
    "dependability_analysis",
    "dependable_element",
)

cc_library(
    name = "sample_library",
    visibility = ["//visibility:public"],
    deps = [
        "//bazel/rules/rules_score/examples/seooc/unit_1:unit_1_lib",
        "//bazel/rules/rules_score/examples/seooc/unit_2:unit_2_lib",
    ],
)

component(
    name = "component_example",
    components = [
        "//bazel/rules/rules_score/examples/seooc/unit_1:unit_1",
        "//bazel/rules/rules_score/examples/seooc/unit_2:unit_2",
    ],
    requirements = [
        "//bazel/rules/rules_score/examples/seooc/docs/requirements:component_requirements",
        "//bazel/rules/rules_score/examples/seooc/docs/requirements:component_requirements_extra",
    ],
    tags = ["manual"],
    tests = [],
)

dependability_analysis(
    name = "sample_dependability_analysis",
    arch_design = "//bazel/rules/rules_score/examples/seooc/design:sample_seooc_design",
    fmea = ["//bazel/rules/rules_score/examples/seooc/safety_analysis:sample_fmea"],
)

dependable_element(
    name = "safety_software_seooc_example",
    architectural_design = ["//bazel/rules/rules_score/examples/seooc/design:sample_seooc_design"],
    assumptions_of_use = [],
    components = [":component_example"],
    dependability_analysis = [
        ":sample_dependability_analysis",
    ],
    description = "Safety Element out of Context (SEooC) example",
    integrity_level = "B",
    requirements = [
        "//bazel/rules/rules_score/examples/seooc/docs/requirements:feature_requirements",
    ],
    tests = [],
    deps = ["//bazel/rules/rules_score/examples/some_other_library:other_seooc"],
)
