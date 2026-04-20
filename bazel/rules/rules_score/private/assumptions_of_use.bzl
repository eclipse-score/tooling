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

load("//bazel/rules/rules_score:providers.bzl", "AssumptionsOfUseInfo", "ComponentRequirementsInfo", "FeatureRequirementsInfo", "SphinxSourcesInfo")

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

    # Collect requirements providers and lobster files
    reqs = []
    lobster_files = []
    for req in ctx.attr.requirements:
        if FeatureRequirementsInfo in req:
            info = req[FeatureRequirementsInfo]
            reqs.append(info)
            lobster_files.append(info.srcs)
        elif ComponentRequirementsInfo in req:
            info = req[ComponentRequirementsInfo]
            reqs.append(info)
            lobster_files.append(info.srcs)

    # Collect transitive sphinx sources from requirements
    transitive = [srcs]
    for req in ctx.attr.requirements:
        if SphinxSourcesInfo in req:
            transitive.append(req[SphinxSourcesInfo].deps)

    return [
        DefaultInfo(files = srcs),
        AssumptionsOfUseInfo(
            srcs = depset(transitive = lobster_files),
            name = ctx.label.name,
        ),
        SphinxSourcesInfo(
            srcs = srcs,
            deps = depset(transitive = transitive),
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
        "requirements": attr.label_list(
            providers = [[FeatureRequirementsInfo], [ComponentRequirementsInfo]],
            mandatory = False,
            doc = "List of feature or component requirements targets that these Assumptions of Use trace to",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def assumptions_of_use(
        name,
        srcs,
        requirements = [],
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
        requirements: Optional list of labels to feature or component requirements
            targets that these Assumptions of Use trace to. Establishes
            traceability as defined in the S-CORE process.
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>: Main assumptions of use target providing AssumptionsOfUseInfo

    Example:
        ```starlark
        assumptions_of_use(
            name = "my_assumptions_of_use",
            srcs = ["assumptions_of_use.rst"],
            requirements = [":my_feature_requirements"],
        )
        ```
    """
    _assumptions_of_use(
        name = name,
        srcs = srcs,
        requirements = requirements,
        visibility = visibility,
    )
