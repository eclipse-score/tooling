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
"""Tests for the ``package`` attr and the rst_to_trlc output-path fix."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

# ============================================================================
# assumed_system_requirements(package = ...) forwards to rst_to_trlc
# ============================================================================

def _asr_package_forwarded_test_impl(ctx):
    env = analysistest.begin(ctx)
    actions = analysistest.target_actions(env)
    rst_actions = [a for a in actions if a.mnemonic == "RstToTrlc"]

    asserts.true(env, len(rst_actions) == 1, "expected exactly one RstToTrlc action")
    if rst_actions:
        argv = rst_actions[0].argv
        asserts.true(env, "--package" in argv, "expected --package in RstToTrlc argv")
        asserts.equals(env, "CustomAsrPackage", argv[argv.index("--package") + 1])

    return analysistest.end(env)

asr_package_forwarded_test = analysistest.make(_asr_package_forwarded_test_impl)

# ============================================================================
# rst_to_trlc: same-basename srcs in one target must not collide
# ============================================================================

def _rst_to_trlc_no_collision_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)
    outs = target_under_test[DefaultInfo].files.to_list()

    asserts.equals(env, 2, len(outs))
    asserts.true(env, outs[0].path != outs[1].path, "same-basename srcs must produce distinct output paths")

    return analysistest.end(env)

rst_to_trlc_no_collision_test = analysistest.make(_rst_to_trlc_no_collision_test_impl)

# ============================================================================
# Test Suite
# ============================================================================

def requirements_package_test_suite(name):
    """Register the package-forwarding and output-collision tests."""
    native.test_suite(
        name = name,
        tests = [
            ":asr_package_forwarded_test",
            ":rst_to_trlc_no_collision_test",
        ],
    )
