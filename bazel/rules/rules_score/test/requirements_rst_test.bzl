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
Tests for RST-based requirement macros:
  - assumed_system_requirements(srcs=[".rst"])
  - feature_requirements(srcs=[".rst"])
  - component_requirements(srcs=[".rst"])
  - assumptions_of_use(srcs=[".rst"])

Each test verifies that the macro correctly exposes its provider when the
input is an RST file rather than a pre-built trlc_requirements label.
"""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load(
    "@score_tooling//bazel/rules/rules_score:providers.bzl",
    "AssumedSystemRequirementsInfo",
    "AssumptionsOfUseInfo",
    "ComponentRequirementsInfo",
    "FeatureRequirementsInfo",
    "SphinxSourcesInfo",
)

# ============================================================================
# assumed_system_requirements – RST input
# ============================================================================

def _asr_rst_output_test_impl(ctx):
    """assumed_system_requirements from RST produces rendered .rst output files."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "assumed_system_requirements should provide SphinxSourcesInfo",
    )
    sphinx_files = target_under_test[SphinxSourcesInfo].srcs.to_list()
    rst_files = [f for f in sphinx_files if f.basename.endswith(".rst")]
    asserts.true(
        env,
        len(rst_files) > 0,
        "assumed_system_requirements should produce a rendered .rst file in SphinxSourcesInfo.srcs",
    )

    return analysistest.end(env)

asr_rst_output_test = analysistest.make(_asr_rst_output_test_impl)

def _asr_rst_provider_test_impl(ctx):
    """assumed_system_requirements from RST exposes AssumedSystemRequirementsInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        AssumedSystemRequirementsInfo in target_under_test,
        "assumed_system_requirements from RST should provide AssumedSystemRequirementsInfo",
    )
    info = target_under_test[AssumedSystemRequirementsInfo]
    asserts.true(
        env,
        info.name != None,
        "AssumedSystemRequirementsInfo should have a name field",
    )
    asserts.true(
        env,
        info.srcs != None,
        "AssumedSystemRequirementsInfo should have a srcs field",
    )

    return analysistest.end(env)

asr_rst_provider_test = analysistest.make(_asr_rst_provider_test_impl)

def _asr_rst_sphinx_test_impl(ctx):
    """assumed_system_requirements from RST exposes SphinxSourcesInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "assumed_system_requirements from RST should provide SphinxSourcesInfo",
    )

    return analysistest.end(env)

asr_rst_sphinx_test = analysistest.make(_asr_rst_sphinx_test_impl)

# ============================================================================
# feature_requirements – RST input
# ============================================================================

def _feat_req_rst_provider_test_impl(ctx):
    """feature_requirements from RST exposes FeatureRequirementsInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        FeatureRequirementsInfo in target_under_test,
        "feature_requirements from RST should provide FeatureRequirementsInfo",
    )
    info = target_under_test[FeatureRequirementsInfo]
    asserts.true(
        env,
        info.name != None,
        "FeatureRequirementsInfo should have a name field",
    )
    asserts.true(
        env,
        info.srcs != None,
        "FeatureRequirementsInfo should have a srcs field",
    )

    return analysistest.end(env)

feat_req_rst_provider_test = analysistest.make(_feat_req_rst_provider_test_impl)

def _feat_req_rst_sphinx_test_impl(ctx):
    """feature_requirements from RST exposes SphinxSourcesInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "feature_requirements from RST should provide SphinxSourcesInfo",
    )

    return analysistest.end(env)

feat_req_rst_sphinx_test = analysistest.make(_feat_req_rst_sphinx_test_impl)

# ============================================================================
# component_requirements – RST input
# ============================================================================

def _comp_req_rst_provider_test_impl(ctx):
    """component_requirements from RST exposes ComponentRequirementsInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        ComponentRequirementsInfo in target_under_test,
        "component_requirements from RST should provide ComponentRequirementsInfo",
    )
    info = target_under_test[ComponentRequirementsInfo]
    asserts.true(
        env,
        info.name != None,
        "ComponentRequirementsInfo should have a name field",
    )
    asserts.true(
        env,
        info.srcs != None,
        "ComponentRequirementsInfo should have a srcs field",
    )

    return analysistest.end(env)

comp_req_rst_provider_test = analysistest.make(_comp_req_rst_provider_test_impl)

def _comp_req_rst_sphinx_test_impl(ctx):
    """component_requirements from RST exposes SphinxSourcesInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "component_requirements from RST should provide SphinxSourcesInfo",
    )

    return analysistest.end(env)

comp_req_rst_sphinx_test = analysistest.make(_comp_req_rst_sphinx_test_impl)

# ============================================================================
# assumptions_of_use – RST input
# ============================================================================

def _aous_rst_provider_test_impl(ctx):
    """assumptions_of_use from RST exposes AssumptionsOfUseInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        AssumptionsOfUseInfo in target_under_test,
        "assumptions_of_use from RST should provide AssumptionsOfUseInfo",
    )
    info = target_under_test[AssumptionsOfUseInfo]
    asserts.true(
        env,
        info.name != None,
        "AssumptionsOfUseInfo should have a name field",
    )
    asserts.true(
        env,
        info.srcs != None,
        "AssumptionsOfUseInfo should have a srcs field",
    )

    return analysistest.end(env)

aous_rst_provider_test = analysistest.make(_aous_rst_provider_test_impl)

def _aous_rst_sphinx_test_impl(ctx):
    """assumptions_of_use from RST exposes SphinxSourcesInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "assumptions_of_use from RST should provide SphinxSourcesInfo",
    )

    return analysistest.end(env)

aous_rst_sphinx_test = analysistest.make(_aous_rst_sphinx_test_impl)

# ============================================================================
# Test Suite
# ============================================================================

def requirements_rst_test_suite(name):
    """Register all RST-based requirement tests.

    Args:
        name: Name for the test_suite target.
    """
    native.test_suite(
        name = name,
        tests = [
            ":asr_rst_output_test",
            ":asr_rst_provider_test",
            ":asr_rst_sphinx_test",
            ":feat_req_rst_provider_test",
            ":feat_req_rst_sphinx_test",
            ":comp_req_rst_provider_test",
            ":comp_req_rst_sphinx_test",
            ":aous_rst_provider_test",
            ":aous_rst_sphinx_test",
        ],
    )
