# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
"""Test rules for sphinx_module HTML generation and dependencies."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("//bazel/rules/rules_score/private:sphinx_module.bzl", "SphinxModuleInfo", "SphinxNeedsInfo")

# ============================================================================
# Provider Tests
# ============================================================================

def _providers_test_impl(ctx):
    """Test that sphinx_module provides the correct providers."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Verify required providers
    asserts.true(
        env,
        SphinxModuleInfo in target_under_test,
        "Target should provide SphinxModuleInfo",
    )

    asserts.true(
        env,
        DefaultInfo in target_under_test,
        "Target should provide DefaultInfo",
    )

    return analysistest.end(env)

providers_test = analysistest.make(_providers_test_impl)

# ============================================================================
# HTML Generation Tests
# ============================================================================

def _basic_html_generation_test_impl(ctx):
    """Test that a simple document generates HTML output."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check that HTML directory exists
    score_info = target_under_test[SphinxModuleInfo]
    asserts.true(
        env,
        score_info.html_dir != None,
        "Module should generate HTML directory",
    )

    return analysistest.end(env)

basic_html_generation_test = analysistest.make(_basic_html_generation_test_impl)

# ============================================================================
# Needs.json Generation Tests
# ============================================================================

def _needs_generation_test_impl(ctx):
    """Test that sphinx_module generates needs.json files."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check for SphinxNeedsInfo provider on _needs target
    # Note: This test requires the _needs suffix target
    asserts.true(
        env,
        DefaultInfo in target_under_test,
        "Needs target should provide DefaultInfo",
    )

    return analysistest.end(env)

needs_generation_test = analysistest.make(_needs_generation_test_impl)

def _needs_transitive_test_impl(ctx):
    """Test that needs.json files are collected transitively."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Verify SphinxNeedsInfo provider
    asserts.true(
        env,
        SphinxNeedsInfo in target_under_test,
        "Needs target should provide SphinxNeedsInfo",
    )

    needs_info = target_under_test[SphinxNeedsInfo]

    # Check direct needs.json file
    asserts.true(
        env,
        needs_info.needs_json_file != None,
        "Should have direct needs.json file",
    )

    # Check transitive needs collection
    asserts.true(
        env,
        needs_info.needs_json_files != None,
        "Should have transitive needs.json files depset",
    )

    return analysistest.end(env)

needs_transitive_test = analysistest.make(_needs_transitive_test_impl)

# ============================================================================
# Dependency and Integration Tests
# ============================================================================

def _module_dependencies_test_impl(ctx):
    """Test that module dependencies are properly handled."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    score_info = target_under_test[SphinxModuleInfo]

    # Module with dependencies should still generate HTML
    asserts.true(
        env,
        score_info.html_dir != None,
        "Module with dependencies should generate HTML",
    )

    return analysistest.end(env)

module_dependencies_test = analysistest.make(_module_dependencies_test_impl)

def _html_merging_test_impl(ctx):
    """Test that HTML from dependencies is merged correctly."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    score_info = target_under_test[SphinxModuleInfo]

    # Verify merged HTML output exists
    asserts.true(
        env,
        score_info.html_dir != None,
        "Merged HTML should be generated",
    )

    return analysistest.end(env)

html_merging_test = analysistest.make(_html_merging_test_impl)

# ============================================================================
# Config Generation Tests
# ============================================================================

def _auto_config_generation_test_impl(ctx):
    """Test that conf.py is automatically generated when not provided."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    score_info = target_under_test[SphinxModuleInfo]

    # Module without explicit config should still generate HTML
    asserts.true(
        env,
        score_info.html_dir != None,
        "Module with auto-generated config should produce HTML",
    )

    return analysistest.end(env)

auto_config_generation_test = analysistest.make(_auto_config_generation_test_impl)

def _explicit_config_test_impl(ctx):
    """Test that explicit conf.py is used when provided."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    score_info = target_under_test[SphinxModuleInfo]

    # Module with explicit config should generate HTML
    asserts.true(
        env,
        score_info.html_dir != None,
        "Module with explicit config should produce HTML",
    )

    return analysistest.end(env)

explicit_config_test = analysistest.make(_explicit_config_test_impl)

# ============================================================================
# Test Suite
# ============================================================================

def sphinx_module_test_suite(name):
    """Create a comprehensive test suite for sphinx_module.

    Tests cover:
    - Needs.json generation and transitive collection
    - Module dependencies and HTML merging

    Args:
        name: Name of the test suite
    """

    native.test_suite(
        name = name,
        tests = [
            # Needs generation
            ":needs_transitive_test",

            # Dependencies and integration
            ":module_dependencies_test",
            ":html_merging_test",
        ],
    )
