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

load("@trlc//:trlc.bzl", "trlc_requirements", "trlc_requirements_test")
load(
    "//bazel/rules/rules_score:rules_score.bzl",
    "fmea",
)

# FMEA
trlc_requirements(
    name = "sample_fmea_failure_modes",
    srcs = [
        "sample_fmea_failure_modes.trlc",
    ],
    spec = [
        "@score_tooling//bazel/rules/rules_score/trlc/config:score_requirements_model",
    ],
    visibility = ["//visibility:public"],
)

trlc_requirements(
    name = "sample_fmea_control_measures",
    srcs = [
        "sample_fmea_control_measures.trlc",
    ],
    spec = [
        "@score_tooling//bazel/rules/rules_score/trlc/config:score_requirements_model",
    ],
    visibility = ["//visibility:public"],
)

trlc_requirements_test(
    name = "sample_fmea_test",
    reqs = [
        ":sample_fmea_failure_modes",
        ":sample_fmea_control_measures",
    ],
    visibility = ["//visibility:public"],
)

#  FTA

filegroup(
    name = "sample_fta",
    srcs = [
        "sample_fta.puml",
    ],
    visibility = ["//visibility:public"],
)

fmea(
    name = "sample_fmea",
    arch_design = "//bazel/rules/rules_score/examples/seooc/design:sample_seooc_design",
    controlmeasures = [
        ":sample_fmea_control_measures",
    ],
    failuremodes = [":sample_fmea_failure_modes"],
    root_causes = [":sample_fta"],
    visibility = ["//visibility:public"],
)
