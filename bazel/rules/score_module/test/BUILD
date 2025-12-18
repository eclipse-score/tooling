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
load("//bazel/rules/score_module:score_module.bzl", "safety_element_out_of_context", "score_module")
load(
    ":html_generation_test.bzl",
    "html_merging_test",
    "module_dependencies_test",
    "needs_transitive_test",
    "score_module_test_suite",
)
load(
    ":score_module_providers_test.bzl",
    "deps_html_merging_test",
    "deps_needs_collection_test",
    "score_module_providers_test_suite",
    "score_needs_with_deps_test",
)
load(
    ":seooc_test.bzl",
    "seooc_artifacts_copied_test",
    "seooc_index_generation_test",
    "seooc_needs_provider_test",
    "seooc_score_module_generated_test",
)

package(default_visibility = ["//visibility:public"])

# ============================================================================
# Test Fixtures - Module Definitions
# ============================================================================

# Test 1: Multi-Module Aggregation
# Dependency graph: module_a_lib -> module_b_lib -> module_c_lib
#                   module_a_lib -> module_c_lib (also direct)
score_module(
    name = "module_c_lib",
    srcs = glob(["fixtures/module_c/*.rst"]),
    index = "fixtures/module_c/index.rst",
    sphinx = "//bazel/rules/score_module:score_build",
)

score_module(
    name = "module_b_lib",
    srcs = glob(["fixtures/module_b/*.rst"]),
    index = "fixtures/module_b/index.rst",
    sphinx = "//bazel/rules/score_module:score_build",
    deps = [":module_c_lib"],
)

score_module(
    name = "module_a_lib",
    srcs = glob(["fixtures/module_a/*.rst"]),
    index = "fixtures/module_a/index.rst",
    sphinx = "//bazel/rules/score_module:score_build",
    deps = [
        ":module_b_lib",
        ":module_c_lib",
    ],
)

# Test 2: SEooC (Safety Element out of Context) Module
# Tests the safety_element_out_of_context macro with S-CORE process artifacts
safety_element_out_of_context(
    name = "seooc_test_lib",
    architectural_design = ["fixtures/seooc_test/architectural_design.rst"],
    assumptions_of_use = ["fixtures/seooc_test/assumptions_of_use.rst"],
    component_requirements = ["fixtures/seooc_test/component_requirements.rst"],
    description = "Test SEooC module demonstrating S-CORE process compliance structure.",
    safety_analysis = ["fixtures/seooc_test/safety_analysis.rst"],
    deps = [
        ":module_c_lib",
    ],
)

# ============================================================================
# Test Instantiations - HTML Generation Tests
# ============================================================================

# Needs Generation Tests
needs_transitive_test(
    name = "needs_transitive_test",
    target_under_test = ":module_b_lib_needs",
)

# Dependency Tests
module_dependencies_test(
    name = "module_dependencies_test",
    target_under_test = ":module_a_lib",
)

html_merging_test(
    name = "html_merging_test",
    target_under_test = ":module_a_lib",
)

# ============================================================================
# Test Instantiations - Provider Tests
# ============================================================================

# ScoreNeedsInfo Tests
score_needs_with_deps_test(
    name = "score_needs_with_deps_test",
    target_under_test = ":module_a_lib_needs",
)

# Dependency Tests
deps_html_merging_test(
    name = "deps_html_merging_test",
    target_under_test = ":module_a_lib",
)

deps_needs_collection_test(
    name = "deps_needs_collection_test",
    target_under_test = ":module_a_lib_needs",
)

# ============================================================================
# SEooC-Specific Tests
# ============================================================================

# Test that index generation works correctly
seooc_index_generation_test(
    name = "seooc_tests_index_generation",
    target_under_test = ":seooc_test_lib_seooc_index",
)

# Test that all artifacts are copied
seooc_artifacts_copied_test(
    name = "seooc_tests_artifacts_copied",
    target_under_test = ":seooc_test_lib_seooc_index",
)

# Test that score_module is generated with correct providers
seooc_score_module_generated_test(
    name = "seooc_tests_score_module_generated",
    target_under_test = ":seooc_test_lib",
)

# Test that needs provider exists for cross-referencing
seooc_needs_provider_test(
    name = "seooc_tests_needs_provider",
    target_under_test = ":seooc_test_lib_needs",
)

# ============================================================================
# Test Suites
# ============================================================================

# Main test suite combining all score_module tests
score_module_test_suite(name = "score_module_tests")

# Provider-focused test suite
score_module_providers_test_suite(name = "provider_tests")

# SEooC-focused test suite
test_suite(
    name = "seooc_tests",
    tests = [
        ":seooc_tests_artifacts_copied",
        ":seooc_tests_index_generation",
        ":seooc_tests_needs_provider",
        ":seooc_tests_score_module_generated",
    ],
)

# Combined test suite for all tests
test_suite(
    name = "all_tests",
    tests = [
        ":provider_tests",
        ":score_module_tests",
        ":seooc_tests",
    ],
)
