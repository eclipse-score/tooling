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
"""
Test suite for unit, component, and dependable_element rules.

Tests the new hierarchical structure for S-CORE process compliance:
- Unit: smallest testable element
- Component: collection of units
- Dependable Element: complete SEooC with full documentation
"""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("//bazel/rules/rules_score:providers.bzl", "ComponentInfo", "SphinxSourcesInfo", "UnitInfo")

# ============================================================================
# Unit Tests
# ============================================================================

def _unit_provider_test_impl(ctx):
    """Test that unit rule provides UnitInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check UnitInfo provider exists
    asserts.true(
        env,
        UnitInfo in target_under_test,
        "Unit should provide UnitInfo",
    )

    unit_info = target_under_test[UnitInfo]

    # Verify fields are populated
    asserts.true(
        env,
        unit_info.name != None,
        "UnitInfo should have name field",
    )

    asserts.true(
        env,
        unit_info.unit_design != None,
        "UnitInfo should have unit_design field",
    )

    asserts.true(
        env,
        unit_info.implementation != None,
        "UnitInfo should have implementation field",
    )

    asserts.true(
        env,
        unit_info.tests != None,
        "UnitInfo should have tests field",
    )

    return analysistest.end(env)

unit_provider_test = analysistest.make(_unit_provider_test_impl)

def _unit_sphinx_sources_test_impl(ctx):
    """Test that unit rule provides SphinxSourcesInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check SphinxSourcesInfo provider exists
    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "Unit should provide SphinxSourcesInfo",
    )

    return analysistest.end(env)

unit_sphinx_sources_test = analysistest.make(_unit_sphinx_sources_test_impl)

# ============================================================================
# Component Tests
# ============================================================================

def _component_provider_test_impl(ctx):
    """Test that component rule provides ComponentInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check ComponentInfo provider exists
    asserts.true(
        env,
        ComponentInfo in target_under_test,
        "Component should provide ComponentInfo",
    )

    comp_info = target_under_test[ComponentInfo]

    # Verify fields are populated
    asserts.true(
        env,
        comp_info.name != None,
        "ComponentInfo should have name field",
    )

    asserts.true(
        env,
        comp_info.requirements != None,
        "ComponentInfo should have component_requirements field",
    )

    asserts.true(
        env,
        comp_info.components != None,
        "ComponentInfo should have components field",
    )

    asserts.true(
        env,
        comp_info.tests != None,
        "ComponentInfo should have tests field",
    )

    return analysistest.end(env)

component_provider_test = analysistest.make(_component_provider_test_impl)

def _component_sphinx_sources_test_impl(ctx):
    """Test that component rule provides SphinxSourcesInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check SphinxSourcesInfo provider exists
    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "Component should provide SphinxSourcesInfo",
    )

    return analysistest.end(env)

component_sphinx_sources_test = analysistest.make(_component_sphinx_sources_test_impl)

# ============================================================================
# Dependable Element Tests
# ============================================================================
# Note: Provider tests removed as dependable_element no longer creates a
# separate provider target. The main target is now a sphinx_module.

# ============================================================================
# Test Suite Definition
# ============================================================================

def unit_component_test_suite(name):
    """Create test suite for unit, component, and dependable_element rules.

    Args:
        name: Name of the test suite
    """
    native.test_suite(
        name = name,
        tests = [
            ":unit_provider_test",
            ":unit_sphinx_sources_test",
            ":component_provider_test",
            ":component_sphinx_sources_test",
        ],
    )
