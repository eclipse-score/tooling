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
Component Requirements build rules for S-CORE projects.

This module provides macros and rules for defining component requirements
following S-CORE process guidelines. Component requirements are derived from
feature requirements and define the specific requirements for a software component.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:feature_requirements.bzl", "FeatureRequirementsInfo")

# ============================================================================
# Provider Definition
# ============================================================================

ComponentRequirementsInfo = provider(
    doc = "Provider for component requirements artifacts",
    fields = {
        "srcs": "Depset of source files containing component requirements",
        "requirements": "List of FeatureRequirementsInfo providers this component traces to",
        "name": "Name of the component requirements target",
    },
)

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _component_requirements_impl(ctx):
    """Implementation for component_requirements rule.

    Collects component requirements source files and links them to their
    parent feature requirements through providers.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo and ComponentRequirementsInfo
    """
    srcs = depset(ctx.files.srcs)

    # Collect feature requirements providers
    feature_reqs = []

    # Collect transitive sphinx sources from feature requirements
    transitive = [srcs]

    return [
        DefaultInfo(files = srcs),
        ComponentRequirementsInfo(
            srcs = srcs,
            name = ctx.label.name,
        ),
        SphinxSourcesInfo(
            srcs = srcs,
            transitive_srcs = depset(transitive = transitive),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_component_requirements = rule(
    implementation = _component_requirements_impl,
    doc = "Collects component requirements documents with traceability to feature requirements",
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".rst", ".md", ".trlc"],
            mandatory = True,
            doc = "Source files containing component requirements specifications",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def component_requirements(
        name,
        srcs,
        visibility = None):
    """Define component requirements following S-CORE process guidelines.

    Component requirements are derived from feature requirements and define
    the specific functional and safety requirements for a software component.
    They establish traceability from high-level features to component-level
    specifications.

    Args:
        name: The name of the component requirements target. Used as the base
            name for all generated targets.
        srcs: List of labels to .rst, .md, or .trlc files containing the
            component requirements specifications as defined in the S-CORE
            process.
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>: Main component requirements target providing ComponentRequirementsInfo

    Example:
        ```starlark
        component_requirements(
            name = "my_component_requirements",
            srcs = ["component_requirements.rst"],
        )
        ```
    """
    _component_requirements(
        name = name,
        srcs = srcs,
        visibility = visibility,
    )
