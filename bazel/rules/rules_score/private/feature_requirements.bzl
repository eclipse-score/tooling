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
Feature Requirements build rules for S-CORE projects.

This module provides macros and rules for defining feature requirements
following S-CORE process guidelines. Feature requirements describe the
high-level features that a software component must implement.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo")

# ============================================================================
# Provider Definition
# ============================================================================

FeatureRequirementsInfo = provider(
    doc = "Provider for feature requirements artifacts",
    fields = {
        "srcs": "Depset of source files containing feature requirements",
        "name": "Name of the feature requirements target",
    },
)

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _feature_requirements_impl(ctx):
    """Implementation for feature_requirements rule.

    Collects feature requirements source files and provides them through
    the FeatureRequirementsInfo provider.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo and FeatureRequirementsInfo
    """
    srcs = depset(ctx.files.srcs)

    return [
        DefaultInfo(files = srcs),
        FeatureRequirementsInfo(
            srcs = srcs,
            name = ctx.label.name,
        ),
        SphinxSourcesInfo(
            srcs = srcs,
            transitive_srcs = srcs,
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_feature_requirements = rule(
    implementation = _feature_requirements_impl,
    doc = "Collects feature requirements documents for S-CORE process compliance",
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".rst", ".md", ".trlc"],
            mandatory = True,
            doc = "Source files containing feature requirements specifications",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def feature_requirements(
        name,
        srcs,
        visibility = None):
    """Define feature requirements following S-CORE process guidelines.

    Feature requirements describe the high-level features and capabilities
    that a software component must implement. They serve as the top-level
    requirements that drive component-level requirements.

    Args:
        name: The name of the feature requirements target. Used as the base
            name for all generated targets.
        srcs: List of labels to .rst, .md, or .trlc files containing the
            feature requirements specifications as defined in the S-CORE
            process.
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>: Main feature requirements target providing FeatureRequirementsInfo

    Example:
        ```starlark
        feature_requirements(
            name = "my_feature_requirements",
            srcs = ["feature_requirements.rst"],
        )
        ```
    """
    _feature_requirements(
        name = name,
        srcs = srcs,
        visibility = visibility,
    )
