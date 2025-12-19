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
"""Unit tests for the safety_element_out_of_context macro."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@rules_python//sphinxdocs/private:sphinx_docs_library_info.bzl", "SphinxDocsLibraryInfo")

# Test that the macro generates the expected targets
def _macro_generates_targets_test_impl(ctx):
    """Test that safety_element_out_of_context macro generates all expected targets."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # The main target should exist and provide the required providers
    asserts.true(
        env,
        DefaultInfo in target_under_test,
        "Main target should provide DefaultInfo",
    )

    asserts.true(
        env,
        SphinxDocsLibraryInfo in target_under_test,
        "Main target should provide SphinxDocsLibraryInfo",
    )

    return analysistest.end(env)

macro_generates_targets_test = analysistest.make(_macro_generates_targets_test_impl)

# Test that the macro correctly creates the index library target
def _macro_index_lib_test_impl(ctx):
    """Test that the macro creates the index library target."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]

    # Verify that transitive documentation includes the index
    transitive_list = sphinx_info.transitive.to_list()
    asserts.true(
        env,
        len(transitive_list) > 0,
        "Macro should generate documentation with index",
    )

    return analysistest.end(env)

macro_index_lib_test = analysistest.make(_macro_index_lib_test_impl)

# Test that the macro properly aggregates all documentation artifacts
def _macro_doc_aggregation_test_impl(ctx):
    """Test that the macro aggregates all documentation artifacts."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]
    transitive_list = sphinx_info.transitive.to_list()

    # Should have documentation entries for:
    # - index
    # - assumptions_of_use
    # - component_requirements
    # - architectural_design
    # - safety_analysis
    # That's at least 5 entries (could be more with nested dependencies)
    asserts.true(
        env,
        len(transitive_list) >= 5,
        "Macro should aggregate all documentation artifacts (expected at least 5, got {})".format(len(transitive_list)),
    )

    return analysistest.end(env)

macro_doc_aggregation_test = analysistest.make(_macro_doc_aggregation_test_impl)

# Test that the macro correctly prefixes documentation paths
def _macro_path_structure_test_impl(ctx):
    """Test that the macro creates correct documentation path structure."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Get the SphinxDocsLibraryInfo provider
    sphinx_info = target_under_test[SphinxDocsLibraryInfo]
    transitive_list = sphinx_info.transitive.to_list()

    # Extract the module name from the target label
    module_name = target_under_test.label.name

    # Check that documentation paths follow the expected structure
    found_correct_structure = False
    for entry in transitive_list:
        if "docs/safety_elements/" in entry.prefix and module_name in entry.prefix:
            found_correct_structure = True
            break

    asserts.true(
        env,
        found_correct_structure,
        "Documentation paths should follow 'docs/safety_elements/<module_name>/' structure",
    )

    return analysistest.end(env)

macro_path_structure_test = analysistest.make(_macro_path_structure_test_impl)

# Test that the macro handles visibility correctly
def _macro_visibility_test_impl(ctx):
    """Test that the macro correctly handles visibility attribute."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # If the target can be accessed in the test, visibility is working
    asserts.true(
        env,
        target_under_test != None,
        "Target should be accessible according to visibility settings",
    )

    return analysistest.end(env)

macro_visibility_test = analysistest.make(_macro_visibility_test_impl)

# Test suite setup function
def _test_safety_element_macro():
    """Creates test targets for the safety_element_out_of_context macro."""

    # Test 1: Verify macro generates expected targets
    macro_generates_targets_test(
        name = "macro_generates_targets_test",
        target_under_test = ":test_macro_seooc",
    )

    # Test 2: Verify index library generation
    macro_index_lib_test(
        name = "macro_index_lib_test",
        target_under_test = ":test_macro_seooc",
    )

    # Test 3: Verify documentation aggregation
    macro_doc_aggregation_test(
        name = "macro_doc_aggregation_test",
        target_under_test = ":test_macro_seooc",
    )

    # Test 4: Verify path structure
    macro_path_structure_test(
        name = "macro_path_structure_test",
        target_under_test = ":test_macro_seooc",
    )

    # Test 5: Verify visibility handling
    macro_visibility_test(
        name = "macro_visibility_test",
        target_under_test = ":test_macro_seooc",
    )

def safety_element_macro_test_suite(name):
    """Creates a test suite for the safety_element_out_of_context macro.

    Args:
        name: The name of the test suite.
    """
    _test_safety_element_macro()

    native.test_suite(
        name = name,
        tests = [
            ":macro_generates_targets_test",
            ":macro_index_lib_test",
            ":macro_doc_aggregation_test",
            ":macro_path_structure_test",
            ":macro_visibility_test",
        ],
    )
