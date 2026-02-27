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
Assumptions of Use build rules for S-CORE projects.

This module provides macros and rules for defining Assumptions of Use (AoU)
following S-CORE process guidelines. Assumptions of Use define the safety-relevant
operating conditions and constraints for a Safety Element out of Context (SEooC).
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:component_requirements.bzl", "ComponentRequirementsInfo")
load("//bazel/rules/rules_score/private:feature_requirements.bzl", "FeatureRequirementsInfo")

# ============================================================================
# Provider Definition
# ============================================================================

AssumptionsOfUseInfo = provider(
    doc = "Provider for assumptions of use artifacts",
    fields = {
        "srcs": "Depset of source files containing assumptions of use",
        "feature_requirements": "List of FeatureRequirementsInfo providers this AoU traces to",
        "name": "Name of the assumptions of use target",
    },
)

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _assumptions_of_use_impl(ctx):
    """Implementation for assumptions_of_use rule.

    Collects assumptions of use source files and links them to their
    parent feature requirements through providers.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo and AssumptionsOfUseInfo
    """
    srcs = depset(ctx.files.srcs)

    # Collect feature requirements providers
    feature_reqs = []
    for feat_req in ctx.attr.feature_requirements:
        if FeatureRequirementsInfo in feat_req:
            feature_reqs.append(feat_req[FeatureRequirementsInfo])

    # Collect transitive sphinx sources from feature requirements
    transitive = [srcs]
    for feat_req in ctx.attr.feature_requirements:
        if SphinxSourcesInfo in feat_req:
            transitive.append(feat_req[SphinxSourcesInfo].transitive_srcs)

    return [
        DefaultInfo(files = srcs),
        AssumptionsOfUseInfo(
            srcs = srcs,
            feature_requirements = feature_reqs,
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

_assumptions_of_use = rule(
    implementation = _assumptions_of_use_impl,
    doc = "Collects Assumptions of Use documents with traceability to feature requirements",
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".rst", ".md", ".trlc"],
            mandatory = True,
            doc = "Source files containing Assumptions of Use specifications",
        ),
        "feature_requirements": attr.label_list(
            providers = [FeatureRequirementsInfo],
            mandatory = False,
            doc = "List of feature_requirements targets that these Assumptions of Use trace to",
        ),
        "component_requirements": attr.label_list(
            providers = [ComponentRequirementsInfo],
            mandatory = False,
            doc = "List of feature_requirements targets that these Assumptions of Use trace to",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def assumptions_of_use(
        name,
        srcs,
        feature_requirement = [],
        component_requirements = [],
        visibility = None):
    """Define Assumptions of Use following S-CORE process guidelines.

    Assumptions of Use (AoU) define the safety-relevant operating conditions
    and constraints for a Safety Element out of Context (SEooC). They specify
    the conditions under which the component is expected to operate safely
    and the responsibilities of the integrator.

    Args:
        name: The name of the assumptions of use target. Used as the base
            name for all generated targets.
        srcs: List of labels to .rst, .md, or .trlc files containing the
            Assumptions of Use specifications as defined in the S-CORE
            process.
        feature_requirement: Optional list of labels to feature_requirements
            targets that these Assumptions of Use relate to. Establishes
            traceability as defined in the S-CORE process.
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>: Main assumptions of use target providing AssumptionsOfUseInfo

    Example:
        ```starlark
        assumptions_of_use(
            name = "my_assumptions_of_use",
            srcs = ["assumptions_of_use.rst"],
            feature_requirement = [":my_feature_requirements"],
        )
        ```
    """
    _assumptions_of_use(
        name = name,
        srcs = srcs,
        feature_requirements = feature_requirement,
        component_requirements = component_requirements,
        visibility = visibility,
    )
