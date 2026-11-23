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
    "component",
    "dependable_element",
    "unit",
)
load(":shared_test_target_test.bzl", "shared_test_target_test")

# One test target attached to several units. It must be reported once, within
# a component, across sibling components and across parent and child component.
unit(
    name = "unit_a",
    testonly = True,
    maturity = "development",
    tests = ["//:test_unit_tests"],
    unit_design = ["//:test_unit_design"],
    implementation = ["//:mock_lib1"],
)

unit(
    name = "unit_b",
    testonly = True,
    maturity = "development",
    tests = ["//:test_unit_tests"],
    unit_design = ["//:test_unit_design"],
    implementation = ["//:mock_lib2"],
)

component(
    name = "same_component",
    testonly = True,
    components = [
        ":unit_a",
        ":unit_b",
    ],
    requirements = [
        "//:comp_req",
        "//:feat_req",
    ],
    tests = [],
)

component(
    name = "sibling_a",
    testonly = True,
    components = [":unit_a"],
    requirements = [
        "//:comp_req",
        "//:feat_req",
    ],
    tests = [],
)

component(
    name = "sibling_b",
    testonly = True,
    components = [":unit_b"],
    requirements = [
        "//:comp_req",
        "//:feat_req",
    ],
    tests = [],
)

component(
    name = "child",
    testonly = True,
    components = [":unit_b"],
    requirements = [
        "//:comp_req",
        "//:feat_req",
    ],
    tests = [],
)

component(
    name = "parent",
    testonly = True,
    components = [
        ":unit_a",
        ":child",
    ],
    requirements = [
        "//:comp_req",
        "//:feat_req",
    ],
    tests = [],
)

dependable_element(
    name = "sibling_de",
    testonly = True,
    architectural_design = ["//:arch_design_elem"],
    components = [
        ":sibling_a",
        ":sibling_b",
    ],
    dependability_analysis = ["//:dependability_analysis_target"],
    integrity_level = "B",
    maturity = "development",
    requirements = ["//:feat_req"],
    tests = [],
)

dependable_element(
    name = "nested_de",
    testonly = True,
    architectural_design = ["//:arch_design_elem"],
    components = [":parent"],
    dependability_analysis = ["//:dependability_analysis_target"],
    integrity_level = "B",
    maturity = "development",
    requirements = ["//:feat_req"],
    tests = [],
)

shared_test_target_test(
    name = "component_test",
    target_under_test = ":same_component",
)

shared_test_target_test(
    name = "sibling_de_test",
    target_under_test = ":sibling_de_index",
)

shared_test_target_test(
    name = "nested_de_test",
    target_under_test = ":nested_de_index",
)
