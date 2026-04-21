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

load("@lobster//:lobster.bzl", "lobster_test", "lobster_trlc")
load("@trlc//:trlc.bzl", "trlc_requirements", "trlc_specification")
load("//manual_analysis:context_from_cc_library.bzl", "manual_analysis_context_from_cc_library")
load("//manual_analysis:context_from_filegroup.bzl", "manual_analysis_context_from_filegroup")
load("//manual_analysis:manual_analysis.bzl", "manual_analysis")

filegroup(
    name = "manual_analysis_context_filegroup",
    srcs = [
        "context_a.txt",
        "context_b.txt",
    ],
)

manual_analysis_context_from_filegroup(
    name = "manual_analysis_context_from_filegroup",
    filegroup = ":manual_analysis_context_filegroup",
    visibility = ["//manual_analysis:__pkg__"],
)

cc_library(
    name = "ma_cc_dep",
    srcs = ["ma_cc_dep.cc"],
    hdrs = ["ma_cc_dep.h"],
)

cc_library(
    name = "ma_cc_root",
    srcs = ["ma_cc_root.cc"],
    hdrs = ["ma_cc_root.h"],
    deps = [":ma_cc_dep"],
)

manual_analysis_context_from_cc_library(
    name = "manual_analysis_context_from_cc_library",
    library = ":ma_cc_root",
    visibility = ["//manual_analysis:__pkg__"],
)

manual_analysis(
    name = "manual_analysis",
    analysis = "analysis.yaml",
    contexts = [
        ":manual_analysis_context_from_filegroup",
        ":manual_analysis_context_from_cc_library",
    ],
    lock_file = "manual.lock",
    results_file = "results.json",
    visibility = ["//manual_analysis:__pkg__"],
)

trlc_specification(
    name = "requirements_spec",
    srcs = ["requirements.rsl"],
)

trlc_requirements(
    name = "requirements",
    srcs = ["requirements.trlc"],
    spec = [":requirements_spec"],
)

lobster_trlc(
    name = "manual_analysis_requirements",
    config = "lobster-trlc.yaml",
    requirements = [":requirements"],
)

lobster_test(
    name = "manual_analysis_report",
    config = "lobster-report.conf",
    inputs = [
        ":manual_analysis",
        ":manual_analysis_requirements",
    ],
)
