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

# Requirements exported by Sphinx-Needs (needs.json).

load("@rules_python//python:defs.bzl", "py_test")
load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "assumed_system_requirements",
    "assumptions_of_use",
    "component_requirements",
    "feature_requirements",
)
load(
    "//:requirements_needs_test.bzl",
    "needs_and_trlc_test",
    "needs_only_test",
    "needs_provider_test",
    "requirement_lobster_files",
)

assumed_system_requirements(
    name = "asr_needs",
    needs = ["needs.json"],
)

feature_requirements(
    name = "feat_req_needs",
    feat_id = "feat__bits",
    needs = ["needs.json"],
)

component_requirements(
    name = "comp_req_needs",
    comp_id = "comp__bits",
    needs = ["needs.json"],
)

assumptions_of_use(
    name = "aous_needs",
    needs = ["needs.json"],
)

# TRLC and Sphinx-Needs requirements in one target.
component_requirements(
    name = "comp_req_mixed_needs",
    srcs = ["//:comp_req_rst_trlc"],
    comp_id = "comp__bits",
    needs = ["needs.json"],
    deps = [
        "//:asr_rst",
        "//:feat_req_rst",
    ],
)

needs_only_test(
    name = "comp_req_needs_only_test",
    target_under_test = ":comp_req_needs",
)

needs_and_trlc_test(
    name = "comp_req_needs_and_trlc_test",
    target_under_test = ":comp_req_mixed_needs",
)

[
    needs_provider_test(
        name = target + "_provider_test",
        target_under_test = ":" + target,
    )
    for target in [
        "asr_needs",
        "feat_req_needs",
        "aous_needs",
    ]
]

requirement_lobster_files(
    name = "needs_lobster_files",
    requirements = [
        ":asr_needs",
        ":feat_req_needs",
        ":comp_req_needs",
        ":aous_needs",
    ],
)

py_test(
    name = "test_requirements_needs_output",
    size = "small",
    srcs = ["test_requirements_needs_output.py"],
    data = [":needs_lobster_files"],
    env = {"LOBSTER_FILES": "$(rootpaths :needs_lobster_files)"},
    deps = ["@lobster//lobster/common"],
)

py_test(
    name = "test_lobster_sphinx_needs",
    size = "small",
    srcs = ["test_lobster_sphinx_needs.py"],
    deps = [
        "@lobster//lobster/common",
        "@score_tooling//bazel/rules/rules_score:lobster_sphinx_needs",
    ],
)

py_test(
    name = "test_lobster_report_to_needs",
    size = "small",
    srcs = ["test_lobster_report_to_needs.py"],
    deps = ["@score_tooling//bazel/rules/rules_score:lobster_report_to_needs"],
)
