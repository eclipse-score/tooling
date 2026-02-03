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
load(
    "//bazel/rules/rules_score:rules_score.bzl",
    "architectural_design",
    "assumptions_of_use",
    "component",
    "component_requirements",
    "dependability_analysis",
    "dependable_element",
    "feature_requirements",
    "safety_analysis",
    "sphinx_module",
    "unit",
)
load(
    ":html_generation_test.bzl",
    "html_merging_test",
    "module_dependencies_test",
    "needs_transitive_test",
    "sphinx_module_test_suite",
)
load(
    ":seooc_test.bzl",
    "seooc_artifacts_copied_test",
    "seooc_needs_provider_test",
    "seooc_sphinx_module_generated_test",
)
load(
    ":unit_component_test.bzl",
    "component_provider_test",
    "component_sphinx_sources_test",
    "dependable_element_provider_test",
    "dependable_element_sphinx_sources_test",
    "unit_component_test_suite",
    "unit_provider_test",
    "unit_sphinx_sources_test",
)

package(default_visibility = ["//visibility:public"])

# ============================================================================
# Test Fixtures - Module Definitions
# ============================================================================

# Test 1: Multi-Module Aggregation
# Dependency graph: module_a_lib -> module_b_lib -> module_c_lib
#                   module_a_lib -> module_c_lib (also direct)
sphinx_module(
    name = "module_c_lib",
    srcs = glob(["fixtures/module_c/*.rst"]),
    index = "fixtures/module_c/index.rst",
    sphinx = "//bazel/rules/rules_score:score_build",
)

sphinx_module(
    name = "module_b_lib",
    srcs = glob(["fixtures/module_b/*.rst"]),
    index = "fixtures/module_b/index.rst",
    sphinx = "//bazel/rules/rules_score:score_build",
    deps = [":module_c_lib"],
)

sphinx_module(
    name = "module_a_lib",
    srcs = glob(["fixtures/module_a/*.rst"]),
    index = "fixtures/module_a/index.rst",
    sphinx = "//bazel/rules/rules_score:score_build",
    deps = [
        ":module_b_lib",
        ":module_c_lib",
    ],
)

# Test 2: SEooC (Safety Element out of Context) Module
# Tests the score_component macro with S-CORE process artifacts

# - Feature Requirements:     wp__requirements_feat
# TODO: Feature requirements are a stand-alone artifact for now
# We have to link them manually to component requirements
feature_requirements(
    name = "feat_req",
    srcs = ["fixtures/seooc_test/feature_requirements.rst"],
)

# - Component Requirements:   wp__requirements_comp
component_requirements(
    name = "comp_req",
    srcs = ["fixtures/seooc_test/component_requirements.rst"],
    feature_requirement = [":feat_req"],
)

# - Assumptions of Use:       wp__requirements_comp_aou
assumptions_of_use(
    name = "aous",
    srcs = ["fixtures/seooc_test/assumptions_of_use.rst"],
    feature_requirement = [":feat_req"],
)

# - Architecture Design:      wp__component_arch
architectural_design(
    name = "arch_design",
    dynamic = ["fixtures/seooc_test/dynamic_architecture.rst"],
    static = ["fixtures/seooc_test/static_architecture.rst"],
)

# - Safety Analysis (DFA):    wp__sw_component_dfa
# - Safety Analysis (FMEA):   wp__sw_component_fmea
dependability_analysis(
    name = "dependability_analysis_target",
    arch_design = ":arch_design",
    dfa = ["fixtures/seooc_test/dfa.rst"],
    safety_analysis = [":samplelibrary_safety_analysis"],
)

safety_analysis(
    name = "samplelibrary_safety_analysis",
    # TODO
    # controlmeasures = [], # can be AoUs or requirements
    # failuremodes = [],
    # fta = [],
    arch_design = ":arch_design",
)

dependable_element(
    name = "seooc_test_lib",
    architectural_design = [":arch_design"],
    assumptions_of_use = [":aous"],
    components = [],
    dependability_analysis = [":dependability_analysis_target"],
    description = "Test SEooC module demonstrating S-CORE process compliance structure.",
    requirements = [":comp_req"],
    tests = [],
    deps = [
        ":module_c_lib",  # dependency to other seoocs/score_components
    ],
)

# ============================================================================
# Test Fixtures - Unit, Component, and Dependable Element
# ============================================================================

# Mock implementation targets with dummy functions
cc_library(
    name = "mock_lib1",
    srcs = ["fixtures/mock_lib1.cc"],
)

cc_library(
    name = "mock_lib2",
    srcs = ["fixtures/mock_lib2.cc"],
)

cc_binary(
    name = "test_component_binary",
    srcs = ["fixtures/test_component_main.cc"],
    deps = [
        ":mock_lib1",
        ":mock_lib2",
    ],
)

cc_test(
    name = "test_unit_tests",
    testonly = True,
    srcs = ["fixtures/test_unit_test.cc"],
    tags = ["manual"],
    deps = [
        ":mock_lib1",
        ":mock_lib2",
    ],
)

# Test Unit
unit(
    name = "test_unit",
    testonly = True,
    tests = [":test_unit_tests"],
    unit_design = [":arch_design"],
    implementation = [
        ":mock_lib1",
        ":mock_lib2",
    ],
)

# Test Component
component(
    name = "test_component",
    testonly = True,
    requirements = [":comp_req"],
    tests = [],  # Empty for testing
    units = [":test_unit"],
    implementation = [":test_component_binary"],
)

# Test Dependable Element
dependable_element(
    name = "test_dependable_element",
    testonly = True,
    architectural_design = [":arch_design"],
    assumptions_of_use = [":aous"],
    components = [":test_component"],
    dependability_analysis = [":dependability_analysis_target"],
    description = "Test dependable element for unit testing",
    requirements = [":comp_req"],
    tests = [],  # Empty for testing
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
# SEooC-Specific Tests
# ============================================================================

# Test that all artifacts are copied
seooc_artifacts_copied_test(
    name = "seooc_tests_artifacts_copied",
    target_under_test = ":seooc_test_lib_index",
)

# Test that sphinx_module is generated with correct providers
seooc_sphinx_module_generated_test(
    name = "seooc_tests_sphinx_module_generated",
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

# Main test suite combining all sphinx_module tests
sphinx_module_test_suite(name = "sphinx_module_tests")

# SEooC-focused test suite
test_suite(
    name = "seooc_tests",
    tests = [
        ":seooc_tests_artifacts_copied",
        ":seooc_tests_needs_provider",
        ":seooc_tests_sphinx_module_generated",
    ],
)

# ============================================================================
# Unit, Component, and Dependable Element Test Instantiations
# ============================================================================

# Unit tests
unit_provider_test(
    name = "unit_provider_test",
    target_under_test = ":test_unit",
)

unit_sphinx_sources_test(
    name = "unit_sphinx_sources_test",
    target_under_test = ":test_unit",
)

# Component tests
component_provider_test(
    name = "component_provider_test",
    target_under_test = ":test_component",
)

component_sphinx_sources_test(
    name = "component_sphinx_sources_test",
    target_under_test = ":test_component",
)

# Dependable Element tests
dependable_element_provider_test(
    name = "dependable_element_provider_test",
    target_under_test = ":test_dependable_element_provider",
)

dependable_element_sphinx_sources_test(
    name = "dependable_element_sphinx_sources_test",
    target_under_test = ":test_dependable_element_provider",
)

# Unit, Component, and Dependable Element test suite
unit_component_test_suite(name = "unit_component_tests")

# ============================================================================
# Combined Test Suite
# ============================================================================

# Combined test suite for all tests
test_suite(
    name = "all_tests",
    tests = [
        ":seooc_tests",
        ":sphinx_module_tests",
        ":unit_component_tests",
    ],
)
