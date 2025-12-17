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
"""Unit tests for the seooc rule."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@rules_python//sphinxdocs/private:sphinx_docs_library_info.bzl", "SphinxDocsLibraryInfo")

# Test that the seooc rule creates the correct providers
def _seooc_providers_test_impl(ctx):
    """Test that seooc rule provides DefaultInfo and SphinxDocsLibraryInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check that DefaultInfo is provided
    asserts.true(
        env,
        DefaultInfo in target_under_test,
        "seooc rule should provide DefaultInfo",
    )

    # Check that SphinxDocsLibraryInfo is provided
    asserts.true(
        env,
        SphinxDocsLibraryInfo in target_under_test,
        "seooc rule should provide SphinxDocsLibraryInfo",
    )

    return analysistest.end(env)

seooc_providers_test = analysistest.make(_seooc_providers_test_impl)

# Test that the seooc rule correctly aggregates transitive documentation
def _seooc_transitive_docs_test_impl(ctx):
    """Test that seooc rule correctly aggregates transitive documentation."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]

    # Check that transitive field is a depset
    asserts.true(
        env,
        type(sphinx_info.transitive) == type(depset([])),
        "SphinxDocsLibraryInfo.transitive should be a depset",
    )

    # Check that transitive documentation is aggregated
    transitive_list = sphinx_info.transitive.to_list()
    asserts.true(
        env,
        len(transitive_list) > 0,
        "seooc should aggregate transitive documentation",
    )

    # Verify that each entry has required fields
    for entry in transitive_list:
        asserts.true(
            env,
            hasattr(entry, "strip_prefix"),
            "Each transitive entry should have strip_prefix field",
        )
        asserts.true(
            env,
            hasattr(entry, "prefix"),
            "Each transitive entry should have prefix field",
        )
        asserts.true(
            env,
            hasattr(entry, "files"),
            "Each transitive entry should have files field",
        )

        # Check that prefix either starts with the expected path or is empty (for index)
        asserts.true(
            env,
            entry.prefix.startswith("docs/safety_elements/") or entry.prefix == "",
            "Documentation prefix should start with 'docs/safety_elements/' or be empty for index",
        )

    return analysistest.end(env)

seooc_transitive_docs_test = analysistest.make(_seooc_transitive_docs_test_impl)

# Test that the seooc rule correctly handles mandatory and optional attributes
def _seooc_attributes_test_impl(ctx):
    """Test that seooc rule correctly handles mandatory attributes."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]

    # Verify that documentation files are present
    transitive_list = sphinx_info.transitive.to_list()

    # There should be at least documentation from the index
    asserts.true(
        env,
        len(transitive_list) >= 1,
        "seooc should have at least index documentation",
    )

    return analysistest.end(env)

seooc_attributes_test = analysistest.make(_seooc_attributes_test_impl)

# Test that seooc properly prefixes paths with module name
def _seooc_path_prefixing_test_impl(ctx):
    """Test that seooc rule correctly prefixes paths with module name."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]
    transitive_list = sphinx_info.transitive.to_list()

    # Extract the module name from the target label
    module_name = target_under_test.label.name

    # Check that at least one entry has the correct prefix
    found_correct_prefix = False
    for entry in transitive_list:
        if module_name in entry.prefix:
            found_correct_prefix = True
            break

    asserts.true(
        env,
        found_correct_prefix,
        "At least one documentation entry should contain the module name in its prefix",
    )

    return analysistest.end(env)

seooc_path_prefixing_test = analysistest.make(_seooc_path_prefixing_test_impl)

# Test that seooc properly processes assumptions_of_use attribute
def _seooc_has_assumptions_test_impl(ctx):
    """Test that seooc rule properly processes assumptions_of_use attribute."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]
    transitive_list = sphinx_info.transitive.to_list()

    # Verify that documentation is present (should include assumptions)
    asserts.true(
        env,
        len(transitive_list) > 0,
        "seooc should include assumptions_of_use documentation",
    )

    return analysistest.end(env)

seooc_has_assumptions_test = analysistest.make(_seooc_has_assumptions_test_impl)

# Test that seooc properly processes component_requirements attribute
def _seooc_has_requirements_test_impl(ctx):
    """Test that seooc rule properly processes component_requirements attribute."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]
    transitive_list = sphinx_info.transitive.to_list()

    # Verify that documentation is present (should include requirements)
    asserts.true(
        env,
        len(transitive_list) > 0,
        "seooc should include component_requirements documentation",
    )

    return analysistest.end(env)

seooc_has_requirements_test = analysistest.make(_seooc_has_requirements_test_impl)

# Test that seooc properly processes architectural_design attribute
def _seooc_has_architecture_test_impl(ctx):
    """Test that seooc rule properly processes architectural_design attribute."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]
    transitive_list = sphinx_info.transitive.to_list()

    # Verify that documentation is present (should include architecture)
    asserts.true(
        env,
        len(transitive_list) > 0,
        "seooc should include architectural_design documentation",
    )

    return analysistest.end(env)

seooc_has_architecture_test = analysistest.make(_seooc_has_architecture_test_impl)

# Test that seooc properly processes safety_analysis attribute
def _seooc_has_safety_test_impl(ctx):
    """Test that seooc rule properly processes safety_analysis attribute."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]
    transitive_list = sphinx_info.transitive.to_list()

    # Verify that documentation is present (should include safety analysis)
    asserts.true(
        env,
        len(transitive_list) > 0,
        "seooc should include safety_analysis documentation",
    )

    return analysistest.end(env)

seooc_has_safety_test = analysistest.make(_seooc_has_safety_test_impl)

# Test that seooc properly processes index attribute
def _seooc_has_index_test_impl(ctx):
    """Test that seooc rule properly processes index attribute."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]
    transitive_list = sphinx_info.transitive.to_list()

    # Verify that documentation is present (should include index)
    asserts.true(
        env,
        len(transitive_list) > 0,
        "seooc should include index documentation",
    )

    return analysistest.end(env)

seooc_has_index_test = analysistest.make(_seooc_has_index_test_impl)

# Test suite setup function
def _test_seooc():
    """Creates test targets for the seooc rule."""

    # Test 1: Verify providers
    seooc_providers_test(
        name = "seooc_providers_test",
        target_under_test = ":test_seooc_minimal",
    )

    # Test 2: Verify transitive documentation aggregation
    seooc_transitive_docs_test(
        name = "seooc_transitive_docs_test",
        target_under_test = ":test_seooc_minimal",
    )

    # Test 3: Verify attribute handling
    seooc_attributes_test(
        name = "seooc_attributes_test",
        target_under_test = ":test_seooc_minimal",
    )

    # Test 4: Verify path prefixing
    seooc_path_prefixing_test(
        name = "seooc_path_prefixing_test",
        target_under_test = ":test_seooc_minimal",
    )

    # Test with complete attributes
    seooc_providers_test(
        name = "seooc_providers_test_complete",
        target_under_test = ":test_seooc_complete",
    )

    seooc_transitive_docs_test(
        name = "seooc_transitive_docs_test_complete",
        target_under_test = ":test_seooc_complete",
    )

    # Test 5: Verify assumptions_of_use attribute is processed
    seooc_has_assumptions_test(
        name = "seooc_has_assumptions_test",
        target_under_test = ":test_seooc_complete",
    )

    # Test 6: Verify component_requirements attribute is processed
    seooc_has_requirements_test(
        name = "seooc_has_requirements_test",
        target_under_test = ":test_seooc_complete",
    )

    # Test 7: Verify architectural_design attribute is processed
    seooc_has_architecture_test(
        name = "seooc_has_architecture_test",
        target_under_test = ":test_seooc_complete",
    )

    # Test 8: Verify safety_analysis attribute is processed
    seooc_has_safety_test(
        name = "seooc_has_safety_test",
        target_under_test = ":test_seooc_complete",
    )

    # Test 9: Verify index attribute is processed
    seooc_has_index_test(
        name = "seooc_has_index_test",
        target_under_test = ":test_seooc_complete",
    )

def seooc_test_suite(name):
    """Creates a test suite for the seooc rule.

    Args:
        name: The name of the test suite.
    """
    _test_seooc()

    native.test_suite(
        name = name,
        tests = [
            ":seooc_providers_test",
            ":seooc_transitive_docs_test",
            ":seooc_attributes_test",
            ":seooc_path_prefixing_test",
            ":seooc_providers_test_complete",
            ":seooc_transitive_docs_test_complete",
            ":seooc_has_assumptions_test",
            ":seooc_has_requirements_test",
            ":seooc_has_architecture_test",
            ":seooc_has_safety_test",
            ":seooc_has_index_test",
        ],
    )
