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

# Requirements exported by Sphinx-Needs (needs.json), read by the *_spn macros.

load("@bazel_skylib//rules:build_test.bzl", "build_test")
load("@rules_python//python:defs.bzl", "py_test")
load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "assumed_system_requirements_spn",
    "assumptions_of_use_spn",
    "component",
    "component_requirements_spn",
    "dependable_element",
    "feature_requirements_spn",
)
load("//:requirements_spn_test.bzl", "requirement_lobster_files", "spn_provider_test")

assumed_system_requirements_spn(
    name = "asr_spn",
    needs = ["needs.json"],
)

feature_requirements_spn(
    name = "feat_req_spn",
    feat_id = "feat__bits",
    needs = ["needs.json"],
)

component_requirements_spn(
    name = "comp_req_spn",
    comp_id = "comp__bits",
    needs = ["needs.json"],
)

assumptions_of_use_spn(
    name = "aous_spn",
    needs = ["needs.json"],
)

[
    spn_provider_test(
        name = target + "_provider_test",
        target_under_test = ":" + target,
    )
    for target in [
        "asr_spn",
        "feat_req_spn",
        "comp_req_spn",
        "aous_spn",
    ]
]

requirement_lobster_files(
    name = "spn_lobster_files",
    requirements = [
        ":asr_spn",
        ":feat_req_spn",
        ":comp_req_spn",
        ":aous_spn",
    ],
)

py_test(
    name = "test_requirements_needs_output",
    size = "small",
    srcs = ["test_requirements_needs_output.py"],
    data = [":spn_lobster_files"],
    env = {"LOBSTER_FILES": "$(rootpaths :spn_lobster_files)"},
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

# The *_spn targets plug into component and dependable_element unchanged.
component(
    name = "bits_component",
    requirements = [
        ":comp_req_spn",
        ":feat_req_spn",
    ],
    # No units or tests, so the component's LOBSTER CI check fails by design.
    tags = ["manual"],
)

dependable_element(
    name = "bits_lib",
    architectural_design = [],
    components = [":bits_component"],
    dependability_analysis = [],
    integrity_level = "B",
    maturity = "development",
    requirements = [":feat_req_spn"],
    tests = [],
)

build_test(
    name = "bits_lib_build_test",
    targets = [":bits_lib_index"],
)
