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
"""
Tests that requirement macros accept a list of labels for ``spec``.

Verifies that assumed_system_requirements, feature_requirements, and
component_requirements all produce their expected providers when spec is
passed as a list of two labels.
"""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load(
    "@score_tooling//bazel/rules/rules_score:providers.bzl",
    "AssumedSystemRequirementsInfo",
    "ComponentRequirementsInfo",
    "FeatureRequirementsInfo",
    "SphinxSourcesInfo",
)

# ============================================================================
# assumed_system_requirements – list spec
# ============================================================================

def _asr_multi_spec_provider_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)
    asserts.true(
        env,
        AssumedSystemRequirementsInfo in target_under_test,
        "assumed_system_requirements with list spec should provide AssumedSystemRequirementsInfo",
    )
    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "assumed_system_requirements with list spec should provide SphinxSourcesInfo",
    )
    return analysistest.end(env)

asr_multi_spec_provider_test = analysistest.make(_asr_multi_spec_provider_test_impl)

# ============================================================================
# feature_requirements – list spec
# ============================================================================

def _feat_req_multi_spec_provider_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)
    asserts.true(
        env,
        FeatureRequirementsInfo in target_under_test,
        "feature_requirements with list spec should provide FeatureRequirementsInfo",
    )
    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "feature_requirements with list spec should provide SphinxSourcesInfo",
    )
    return analysistest.end(env)

feat_req_multi_spec_provider_test = analysistest.make(_feat_req_multi_spec_provider_test_impl)

# ============================================================================
# component_requirements – list spec
# ============================================================================

def _comp_req_multi_spec_provider_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)
    asserts.true(
        env,
        ComponentRequirementsInfo in target_under_test,
        "component_requirements with list spec should provide ComponentRequirementsInfo",
    )
    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "component_requirements with list spec should provide SphinxSourcesInfo",
    )
    return analysistest.end(env)

comp_req_multi_spec_provider_test = analysistest.make(_comp_req_multi_spec_provider_test_impl)

# ============================================================================
# Test Suite
# ============================================================================

def requirements_multi_spec_test_suite(name):
    """Register all multi-spec requirement tests.

    Args:
        name: Name for the test_suite target.
    """
    native.test_suite(
        name = name,
        tests = [
            ":asr_multi_spec_provider_test",
            ":feat_req_multi_spec_provider_test",
            ":comp_req_multi_spec_provider_test",
            # Integration test: trlc --verify with both RSL files actually loaded
            ":asr_multi_spec_inttest_test",
        ],
    )
