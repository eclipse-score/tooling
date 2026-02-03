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
load("//bazel/rules/rules_score:providers.bzl", "ComponentInfo", "DependableElementInfo", "SphinxSourcesInfo", "UnitInfo")

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
        comp_info.units != None,
        "ComponentInfo should have units field",
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

def _dependable_element_provider_test_impl(ctx):
    """Test that dependable_element rule provides DependableElementInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check DependableElementInfo provider exists
    asserts.true(
        env,
        DependableElementInfo in target_under_test,
        "Dependable element should provide DependableElementInfo",
    )

    de_info = target_under_test[DependableElementInfo]

    # Verify fields are populated
    asserts.true(
        env,
        de_info.name != None,
        "DependableElementInfo should have name field",
    )

    asserts.true(
        env,
        de_info.description != None,
        "DependableElementInfo should have description field",
    )

    asserts.true(
        env,
        de_info.assumptions_of_use != None,
        "DependableElementInfo should have assumptions_of_use field",
    )

    asserts.true(
        env,
        de_info.requirements != None,
        "DependableElementInfo should have requirements field",
    )

    asserts.true(
        env,
        de_info.architectural_design != None,
        "DependableElementInfo should have architectural_design field",
    )

    asserts.true(
        env,
        de_info.dependability_analysis != None,
        "DependableElementInfo should have dependability_analysis field",
    )

    asserts.true(
        env,
        de_info.consists_of != None,
        "DependableElementInfo should have consists_of field",
    )

    asserts.true(
        env,
        de_info.tests != None,
        "DependableElementInfo should have tests field",
    )

    return analysistest.end(env)

dependable_element_provider_test = analysistest.make(_dependable_element_provider_test_impl)

def _dependable_element_sphinx_sources_test_impl(ctx):
    """Test that dependable_element rule provides SphinxSourcesInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check SphinxSourcesInfo provider exists
    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "Dependable element should provide SphinxSourcesInfo",
    )

    return analysistest.end(env)

dependable_element_sphinx_sources_test = analysistest.make(_dependable_element_sphinx_sources_test_impl)

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
            ":dependable_element_provider_test",
            ":dependable_element_sphinx_sources_test",
        ],
    )
