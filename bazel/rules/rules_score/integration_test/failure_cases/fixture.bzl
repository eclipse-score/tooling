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

load("@rules_cc//cc:defs.bzl", "cc_library", "cc_test")
load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "architectural_design",
    "component",
    "dependable_element",
    "unit",
    "unit_design",
)

def fixture(
        maturity = "release",
        lock = None,
        static = "//base:static_design.puml",
        design = "//base:class_design.puml",
        implementation_deps = [],
        test_src = "//base:my_unit_test.cpp"):
    architectural_design(
        name = "my_arch",
        static = [static],
    )

    unit_design(
        name = "my_unit_design",
        static = [design],
    )

    cc_library(
        name = "unit_hdr",
        hdrs = ["//base:my_unit.h"],
    )

    cc_library(
        name = "unit_lib",
        srcs = ["//base:my_unit.cpp"],
        deps = [":unit_hdr"] + implementation_deps,
    )

    cc_test(
        name = "unit_test",
        srcs = [test_src],
        deps = [
            ":unit_lib",
            "@googletest//:gtest_main",
        ],
    )

    unit(
        name = "MyUnit",
        implementation = [":unit_lib"],
        scope = [":unit_hdr"],
        tests = [":unit_test"],
        unit_design = [":my_unit_design"],
    )

    component(
        name = "SampleComponent",
        components = [":MyUnit"],
        requirements = [
            "//base:component_requirements",
            "//base:feature_requirements",
            "//base:assumed_system_requirements",
        ],
        test_case_coverage_lock = lock,
        tests = [],
    )

    dependable_element(
        name = "my_element",
        architectural_design = [":my_arch"],
        components = [":SampleComponent"],
        dependability_analysis = [],
        integrity_level = "B",
        maturity = maturity,
        requirements = ["//base:feature_requirements"],
        tests = [],
    )
