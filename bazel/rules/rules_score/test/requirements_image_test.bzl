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
Analysis tests for image_srcs propagation through the requirements rules.

Verifies that when image files are provided via image_srcs the rendered .rst
and the staged image file both appear in SphinxSourcesInfo.srcs.
"""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load(
    "@score_tooling//bazel/rules/rules_score:providers.bzl",
    "SphinxSourcesInfo",
)

# ============================================================================
# image_srcs propagation test
# ============================================================================

def _image_srcs_sphinx_sources_test_impl(ctx):
    """feature_requirements with image_srcs stages the image in SphinxSourcesInfo.srcs."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "feature_requirements with image_srcs should provide SphinxSourcesInfo",
    )

    sphinx_files = target_under_test[SphinxSourcesInfo].srcs.to_list()

    rst_files = [f for f in sphinx_files if f.extension == "rst"]
    asserts.true(
        env,
        len(rst_files) == 1,
        "SphinxSourcesInfo.srcs should contain exactly one rendered .rst file, got: " +
        str([f.basename for f in rst_files]),
    )

    image_files = [f for f in sphinx_files if f.extension in ("svg", "png")]
    asserts.true(
        env,
        len(image_files) == 2,
        "SphinxSourcesInfo.srcs should contain exactly two staged image files, got: " +
        str([f.basename for f in image_files]),
    )

    basenames = sorted([f.basename for f in image_files])
    asserts.equals(env, ["arch.png", "arch.svg"], basenames)

    # Verify images are staged at package-relative paths under diagrams/.
    image_short_paths = sorted([f.short_path for f in image_files])
    asserts.true(
        env,
        image_short_paths[0].endswith("diagrams/arch.png") and image_short_paths[1].endswith("diagrams/arch.svg"),
        "Staged images should preserve package-relative paths under diagrams/, got: " +
        str(image_short_paths),
    )

    return analysistest.end(env)

image_srcs_sphinx_sources_test = analysistest.make(_image_srcs_sphinx_sources_test_impl)

# ============================================================================
# Test Suite
# ============================================================================

def requirements_image_test_suite(name):
    """Register all image_srcs propagation tests.

    Args:
        name: Name for the test_suite target.
    """
    native.test_suite(
        name = name,
        tests = [
            ":image_srcs_sphinx_sources_test",
        ],
    )
