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
    "//bazel/rules/score_module:score_module.bzl",
    "architectural_design",
    "assumptions_of_use",
    "component_requirements",
    "dependability_analysis",
    "feature_requirements",
    "safety_analysis",
    "score_component",
    "sphinx_module",
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
    sphinx = "//bazel/rules/score_module:score_build",
)

sphinx_module(
    name = "module_b_lib",
    srcs = glob(["fixtures/module_b/*.rst"]),
    index = "fixtures/module_b/index.rst",
    sphinx = "//bazel/rules/score_module:score_build",
    deps = [":module_c_lib"],
)

sphinx_module(
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

score_component(
    name = "seooc_test_lib",
    architectural_design = [":arch_design"],
    assumptions_of_use = [":aous"],
    component_requirements = [":comp_req"],
    dependability_analysis = [":dependability_analysis_target"],
    description = "Test SEooC module demonstrating S-CORE process compliance structure.",
    implementations = [],
    tests = [],
    deps = [
        ":module_c_lib",  # dependency to other seoocs/score_components
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
# SEooC-Specific Tests
# ============================================================================

# Test that all artifacts are copied
seooc_artifacts_copied_test(
    name = "seooc_tests_artifacts_copied",
    target_under_test = ":seooc_test_lib_seooc_index",
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

# Combined test suite for all tests
test_suite(
    name = "all_tests",
    tests = [
        ":seooc_tests",
        ":sphinx_module_tests",
    ],
)
