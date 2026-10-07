# *******************************************************************************
# Copyright (c) 2024 Contributors to the Eclipse Foundation
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
    "failure_modes",
    "fault_trees",
    "safety_analysis",
)

#  FTA

filegroup(
    name = "sample_fta",
    srcs = [
        "sample_fta.puml",
        "sample_fta2.puml",
    ],
    visibility = ["//visibility:public"],
)

failure_modes(
    name = "sample_failure_modes",
    srcs = ["sample_safety_analysis_failure_modes.trlc"],
)

# Root causes of the failure modes. Requirement targets (the AoUs in
# //docs:sample_aous) list it in `deps` to reference the generated root causes.
fault_trees(
    name = "sample_fault_trees",
    srcs = [":sample_fta"],
    fta_package = "sample_safety_analysis_fta",
    visibility = ["//visibility:public"],
    deps = [":sample_failure_modes"],
)

# The failure modes with the measures addressing their root causes.
safety_analysis(
    name = "sample_safety_analysis",
    arch_design = "//design:sample_seooc_design",
    failure_modes = [":sample_failure_modes"],
    fault_trees = ":sample_fault_trees",
    safety_measures = [
        "//docs:sample_aous",
        "sample_safety_analysis_safetymeasures.trlc",
    ],
    visibility = ["//visibility:public"],
)
