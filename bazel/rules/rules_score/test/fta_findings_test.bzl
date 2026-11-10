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
"""Fixture macro: a dependable_element over the transfer_fta diagrams whose CI report must fail with a finding."""

load("@bazel_skylib//rules:diff_test.bzl", "diff_test")
load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "dependability_analysis",
    "dependable_element",
    "failure_modes",
    "fault_trees",
    "safety_analysis",
)
load(":lobster_ci_report_test.bzl", "lobster_ci_report_test")

def fta_findings_fixture(
        name,
        contains,
        lacks = [],
        failure_modes_src = None,
        golden_events = None,
        golden_index = None):
    """Builds the fault trees of fixtures/transfer_fta and checks that lobster-ci-report fails.

    Creates `<name>_fault_trees` ... `<name>_example_lib` and `<name>_ci_report_test`.

    Args:
        name: Prefix of every generated target.
        contains: Strings the failing CI report must contain.
        lacks: Strings the CI report must not contain.
        failure_modes_src: TRLC file defining the failure modes; none when unset.
        golden_events: Expected fta_events.trlc; adds `<name>_events_golden_test` when set.
        golden_index: Expected fta_index.json; adds `<name>_index_golden_test` when set.
    """
    arch = ":arch_design_public_api_example"
    fault_tree_kwargs = {}
    safety_kwargs = {}
    if failure_modes_src:
        failure_modes(
            name = name + "_failure_modes",
            srcs = [failure_modes_src],
            deps = [arch],
        )
        fault_tree_kwargs["deps"] = [":" + name + "_failure_modes"]
        safety_kwargs["failure_modes"] = [":" + name + "_failure_modes"]

    fault_trees(
        name = name + "_fault_trees",
        srcs = [
            "fixtures/transfer_fta/shared_fta.puml",
            "fixtures/transfer_fta/top_fta.puml",
        ],
        **fault_tree_kwargs
    )
    safety_analysis(
        name = name + "_safety_analysis",
        fault_trees = ":" + name + "_fault_trees",
        **safety_kwargs
    )
    dependability_analysis(
        name = name + "_dependability_analysis",
        arch_design = arch,
        safety_analysis = [":" + name + "_safety_analysis"],
        tags = ["manual"],
    )
    dependable_element(
        name = name + "_example_lib",
        testonly = True,
        architectural_design = [arch],
        components = [":test_component"],
        dependability_analysis = [":" + name + "_dependability_analysis"],
        integrity_level = "B",
        maturity = "release",
        requirements = [":feat_req"],
        tags = ["manual"],
        tests = [],
    )
    lobster_ci_report_test(
        name = name + "_ci_report_test",
        contains = contains,
        expect_ci_failure = True,
        index = ":" + name + "_example_lib_index",
        lacks = lacks,
    )

    for kind, output_group, golden in [
        ("events", "fta_events", golden_events),
        ("index", "fta_index", golden_index),
    ]:
        if golden:
            native.filegroup(
                name = "{}_{}".format(name, kind),
                srcs = [":" + name + "_fault_trees"],
                output_group = output_group,
            )
            diff_test(
                name = "{}_{}_golden_test".format(name, kind),
                file1 = ":{}_{}".format(name, kind),
                file2 = golden,
            )
