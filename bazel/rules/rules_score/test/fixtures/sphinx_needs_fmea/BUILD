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

# FMEA written as Sphinx-Needs: the comp_saf_fmea needs become generated
# FailureMode records, which the fault tree references by need ID and which
# trace to the public API through their violated real_arc_int_op need.

load("@rules_python//python:defs.bzl", "py_test")
load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "component_requirements",
    "dependability_analysis",
    "failure_modes",
    "fault_trees",
    "safety_analysis",
)

failure_modes(
    name = "needs_fmea_failure_modes",
    needs = ["needs.json"],
    package = "NeedsFmeaExample",
    deps = ["//:arch_design_public_api_example"],
)

fault_trees(
    name = "needs_fmea_fault_trees",
    srcs = ["fta.puml"],
    fta_package = "NeedsFmeaFta",
    deps = [":needs_fmea_failure_modes"],
)

component_requirements(
    name = "needs_fmea_measures",
    comp_id = "comp__example",
    needs = ["needs.json"],
)

safety_analysis(
    name = "needs_fmea_safety_analysis",
    failure_modes = [":needs_fmea_failure_modes"],
    fault_trees = ":needs_fmea_fault_trees",
    safety_measures = [":needs_fmea_measures"],
)

dependability_analysis(
    name = "needs_fmea_dependability_analysis",
    arch_design = "//:arch_design_public_api_example",
    safety_analysis = [":needs_fmea_safety_analysis"],
)

# The sufficient FMEA need assesses its root causes and traces its measure.
py_test(
    name = "test_needs_fmea_safety_analysis",
    size = "small",
    srcs = ["test_needs_fmea_safety_analysis.py"],
    data = [":needs_fmea_safety_analysis"],
    env = {"SAFETY_ANALYSIS_FILES": "$(rootpaths :needs_fmea_safety_analysis)"},
)

py_test(
    name = "test_fmea_needs_to_trlc",
    size = "small",
    srcs = ["test_fmea_needs_to_trlc.py"],
    deps = ["@score_tooling//bazel/rules/rules_score:fmea_needs_to_trlc"],
)

py_test(
    name = "test_fmea_needs_assessments",
    size = "small",
    srcs = ["test_fmea_needs_assessments.py"],
    deps = [
        "@lobster//lobster/common",
        "@score_tooling//bazel/rules/rules_score:fmea_needs_assessments",
    ],
)
