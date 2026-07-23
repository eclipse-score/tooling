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
    "assumed_system_requirements",
    "component",
    "dependable_element",
    "feature_requirements",
    "unit",
    "unit_design",
)

# Exposes source files to the parent workspace so that the rules_score_doc
# Sphinx build can reference them via sphinx_docs_library without copies.
filegroup(
    name = "doc_sources",
    srcs = [
        "docs/class_design.puml",
        "docs/static_design.puml",
        "requirements/asr.trlc",
        "requirements/feature_requirements.trlc",
        "src/my_unit.cpp",
        "src/my_unit.h",
        "test/my_unit_test.cpp",
    ],
    visibility = ["//bazel/rules/rules_score:__pkg__"],
)

assumed_system_requirements(
    name = "assumed_system_requirements",
    srcs = ["requirements/asr.trlc"],
    visibility = ["//visibility:public"],
)

feature_requirements(
    name = "feature_requirements",
    srcs = ["requirements/feature_requirements.trlc"],
    visibility = ["//visibility:public"],
    deps = [":assumed_system_requirements"],
)

architectural_design(
    name = "my_arch",
    static = ["docs/static_design.puml"],
)

cc_library(
    name = "my_unit_lib",
    srcs = ["src/my_unit.cpp"],
    hdrs = ["src/my_unit.h"],
)

cc_test(
    name = "my_unit_test",
    srcs = ["test/my_unit_test.cpp"],
    deps = [
        ":my_unit_lib",
        "@googletest//:gtest_main",
    ],
)

unit_design(
    name = "MyUnit_design",
    static = ["docs/class_design.puml"],
)

unit(
    name = "MyUnit",
    scope = ["//:my_unit_lib"],
    tests = [":my_unit_test"],
    unit_design = [":MyUnit_design"],
    implementation = [":my_unit_lib"],
)

component(
    name = "MyComponent",
    components = [":MyUnit"],
    requirements = [],
    tests = [],
)

dependable_element(
    name = "my_element",
    architectural_design = [":my_arch"],
    assumptions_of_use = [],
    components = [":MyComponent"],
    dependability_analysis = [],
    integrity_level = "B",
    requirements = [":feature_requirements"],
    tests = [],
)
