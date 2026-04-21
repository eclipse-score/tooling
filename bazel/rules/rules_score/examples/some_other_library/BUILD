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
    "//bazel/rules/rules_score:rules_score.bzl",
    "architectural_design",
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

unit(
    name = "abc",
    scope = ["//bazel/rules/rules_score/examples/some_other_library:__pkg__"],
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
    requirements = ["//bazel/rules/rules_score/examples/seooc/docs/requirements:component_requirements"],
    tags = ["manual"],
    tests = [],
)

dependable_element(
    name = "other_seooc",
    architectural_design = [":sample_seooc_design"],
    assumptions_of_use = [],
    components = [":component_example"],
    dependability_analysis = [],
    description = "XYZ",
    integrity_level = "D",
    requirements = [],
    tests = [],
    visibility = ["//:__subpackages__"],
)

cc_library(
    name = "some_other_library",
    srcs = [],
    visibility = ["//:__subpackages__"],
    deps = [],
)
