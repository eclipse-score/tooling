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

safety_analysis(
    name = "sample_safety_analysis",
    arch_design = "//design:sample_seooc_design",
    controlmeasures = ["sample_safety_analysis_control_measures.trlc"],
    failuremodes = ["sample_safety_analysis_failure_modes.trlc"],
    root_causes = [":sample_fta"],
    visibility = ["//visibility:public"],
)
