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
load("@rules_python//sphinxdocs:sphinx_docs_library.bzl", "sphinx_docs_library")
load("//bazel/rules/score_module:score_module.bzl", "safety_element_out_of_context")
load("//bazel/rules/score_module/private:seooc.bzl", "seooc")
load(":score_module_test.bzl", "safety_element_macro_test_suite")
load(":seooc_test.bzl", "seooc_test_suite")

package(default_visibility = ["//visibility:public"])

# Test fixtures for seooc tests
sphinx_docs_library(
    name = "test_assumptions",
    srcs = ["fixtures/assumptions_of_use.rst"],
    tags = ["manual"],
)

sphinx_docs_library(
    name = "test_requirements",
    srcs = ["fixtures/component_requirements.rst"],
    tags = ["manual"],
)

sphinx_docs_library(
    name = "test_architecture",
    srcs = ["fixtures/architecture_design.rst"],
    tags = ["manual"],
)

sphinx_docs_library(
    name = "test_safety",
    srcs = ["fixtures/safety_analysis.rst"],
    tags = ["manual"],
)

sphinx_docs_library(
    name = "test_index_lib",
    srcs = ["fixtures/index.rst"],
    tags = ["manual"],
)

# Minimal seooc test target with all mandatory attributes
seooc(
    name = "test_seooc_minimal",
    architectural_design = ":test_architecture",
    assumptions_of_use = ":test_assumptions",
    component_requirements = ":test_requirements",
    index = ":test_index_lib",
    safety_analysis = ":test_safety",
    tags = ["manual"],
)

# Complete seooc test target with all mandatory attributes
seooc(
    name = "test_seooc_complete",
    architectural_design = ":test_architecture",
    assumptions_of_use = ":test_assumptions",
    component_requirements = ":test_requirements",
    index = ":test_index_lib",
    safety_analysis = ":test_safety",
    tags = ["manual"],
)

# Run the test suite
seooc_test_suite(name = "seooc_tests")

# Test target using the safety_element_out_of_context macro
# Uses sphinx_docs_library targets that provide SphinxDocsLibraryInfo
safety_element_out_of_context(
    name = "test_macro_seooc",
    architectural_design = ":test_architecture",
    assumptions_of_use = ":test_assumptions",
    component_requirements = ":test_requirements",
    implementations = None,
    safety_analysis = ":test_safety",
    tests = None,
    visibility = ["//visibility:public"],
)

# Run the macro test suite
safety_element_macro_test_suite(name = "macro_tests")
