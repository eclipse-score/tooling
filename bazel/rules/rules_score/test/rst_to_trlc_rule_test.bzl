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
"""Tests for the rst_to_trlc rule and its use as srcs of requirement macros."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@trlc//:trlc.bzl", "TrlcProviderInfo")

# ============================================================================
# rst_to_trlc(package / ref_package) are forwarded to the converter
# ============================================================================

def _flag_value(argv, flag):
    return argv[argv.index(flag) + 1] if flag in argv else None

def _package_forwarded_test_impl(ctx):
    env = analysistest.begin(ctx)
    actions = [a for a in analysistest.target_actions(env) if a.mnemonic == "RstToTrlc"]

    asserts.equals(env, 1, len(actions))
    if actions:
        asserts.equals(env, "CustomAsrPackage", _flag_value(actions[0].argv, "--package"))

    return analysistest.end(env)

rst_to_trlc_package_forwarded_test = analysistest.make(_package_forwarded_test_impl)

def _ref_package_forwarded_test_impl(ctx):
    env = analysistest.begin(ctx)
    actions = [a for a in analysistest.target_actions(env) if a.mnemonic == "RstToTrlc"]

    asserts.equals(env, 1, len(actions))
    if actions:
        asserts.equals(env, "AssumedSystemRequirements", _flag_value(actions[0].argv, "--ref-package"))
        asserts.equals(env, None, _flag_value(actions[0].argv, "--package"))

    return analysistest.end(env)

rst_to_trlc_ref_package_forwarded_test = analysistest.make(_ref_package_forwarded_test_impl)

# ============================================================================
# rst_to_trlc: same-basename srcs in one target must not collide
# ============================================================================

def _no_collision_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)
    outs = target_under_test[DefaultInfo].files.to_list()

    asserts.equals(env, 2, len(outs))
    asserts.true(env, outs[0].path != outs[1].path, "same-basename srcs must produce distinct output paths")

    return analysistest.end(env)

rst_to_trlc_no_collision_test = analysistest.make(_no_collision_test_impl)

# ============================================================================
# rst_to_trlc: sources of another Bazel package
# ============================================================================

def _cross_package_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)
    outs = target_under_test[DefaultInfo].files.to_list()

    asserts.equals(env, 1, len(outs))
    if outs:
        asserts.true(env, outs[0].short_path.endswith(target_under_test.label.name + "/0/req.trlc"), "output must live under the target's own directory")
        asserts.equals(env, target_under_test.label, outs[0].owner)

    return analysistest.end(env)

rst_to_trlc_cross_package_test = analysistest.make(_cross_package_test_impl)

# ============================================================================
# Requirement macro: srcs providing TrlcProviderInfo are routed to deps
# ============================================================================

def _provider_srcs_routed_to_deps_test_impl(ctx):
    env = analysistest.begin(ctx)
    info = analysistest.target_under_test(env)[TrlcProviderInfo]

    own = [f.basename for f in info.reqs.to_list()]
    transitive = [f.basename for f in info.deps.to_list()]
    asserts.equals(env, ["feature_requirements.trlc"], own)
    asserts.true(env, "assumed_system_requirements.trlc" in transitive, "provider target in srcs must contribute to deps")
    asserts.false(env, "assumed_system_requirements.trlc" in own, "provider target in srcs must not be an own source")

    return analysistest.end(env)

provider_srcs_routed_to_deps_test = analysistest.make(_provider_srcs_routed_to_deps_test_impl)

# ============================================================================
# Test Suite
# ============================================================================

def rst_to_trlc_test_suite(name):
    """Register the rst_to_trlc rule tests."""
    native.test_suite(
        name = name,
        tests = [
            ":rst_to_trlc_package_forwarded_test",
            ":rst_to_trlc_ref_package_forwarded_test",
            ":rst_to_trlc_no_collision_test",
            ":rst_to_trlc_cross_package_test",
            ":provider_srcs_routed_to_deps_test",
        ],
    )
