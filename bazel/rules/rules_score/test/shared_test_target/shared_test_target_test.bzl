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

"""Analysis test for a test target shared by several units."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _shared_test_target_test_impl(ctx):
    """
    Given units that share a single test target (in one component, or across
    components of a dependable_element),
    When the target is analyzed,
    Then a single lobster-gtest action receives the shared test's report once.
    """
    env = analysistest.begin(ctx)

    gtest_actions = [
        action
        for action in analysistest.target_actions(env)
        if action.argv and action.argv[0].endswith("lobster-gtest")
    ]
    asserts.equals(env, 1, len(gtest_actions), "Expected exactly one lobster-gtest action")
    if len(gtest_actions) != 1:
        return analysistest.end(env)

    reports = [f for f in gtest_actions[0].inputs.to_list() if f.basename.endswith(".xml")]
    asserts.equals(
        env,
        1,
        len(reports),
        "shared test target must be reported once; inputs were: %s" % reports,
    )

    return analysistest.end(env)

shared_test_target_test = analysistest.make(_shared_test_target_test_impl)
