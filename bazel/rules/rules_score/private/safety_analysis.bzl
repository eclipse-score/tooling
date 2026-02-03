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
Safety Analysis build rules for S-CORE projects.

This module provides macros and rules for defining safety analysis documentation
following S-CORE process guidelines. Safety analysis includes failure mode analysis,
control measures, fault tree analysis, and other safety-related artifacts.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:architectural_design.bzl", "ArchitecturalDesignInfo")

# ============================================================================
# Provider Definition
# ============================================================================

SafetyAnalysisInfo = provider(
    doc = "Provider for safety analysis artifacts",
    fields = {
        "controlmeasures": "Depset of control measures documentation or requirements",
        "failuremodes": "Depset of failure modes documentation or requirements",
        "fta": "Depset of Fault Tree Analysis diagrams",
        "arch_design": "ArchitecturalDesignInfo provider for linked architectural design",
        "name": "Name of the safety analysis target",
    },
)

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _safety_analysis_impl(ctx):
    """Implementation for safety_analysis rule.

    Collects safety analysis artifacts including control measures, failure modes,
    and fault tree analysis diagrams, linking them to architectural design.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo and SafetyAnalysisInfo
    """
    controlmeasures = depset(ctx.files.controlmeasures)
    failuremodes = depset(ctx.files.failuremodes)
    fta = depset(ctx.files.fta)

    # Get architectural design provider if available
    arch_design_info = None
    if ctx.attr.arch_design and ArchitecturalDesignInfo in ctx.attr.arch_design:
        arch_design_info = ctx.attr.arch_design[ArchitecturalDesignInfo]

    # Combine all files for DefaultInfo
    all_files = depset(
        transitive = [controlmeasures, failuremodes, fta],
    )

    # Collect transitive sphinx sources from architectural design
    transitive = [all_files]
    if ctx.attr.arch_design and SphinxSourcesInfo in ctx.attr.arch_design:
        transitive.append(ctx.attr.arch_design[SphinxSourcesInfo].transitive_srcs)

    return [
        DefaultInfo(files = all_files),
        SafetyAnalysisInfo(
            controlmeasures = controlmeasures,
            failuremodes = failuremodes,
            fta = fta,
            arch_design = arch_design_info,
            name = ctx.label.name,
        ),
        SphinxSourcesInfo(
            srcs = all_files,
            transitive_srcs = depset(transitive = transitive),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_safety_analysis = rule(
    implementation = _safety_analysis_impl,
    doc = "Collects safety analysis documents for S-CORE process compliance",
    attrs = {
        "controlmeasures": attr.label_list(
            allow_files = [".rst", ".md", ".trlc"],
            mandatory = False,
            doc = "Control measures documentation or requirements targets (can be AoUs or requirements)",
        ),
        "failuremodes": attr.label_list(
            allow_files = [".rst", ".md", ".trlc"],
            mandatory = False,
            doc = "Failure modes documentation or requirements targets",
        ),
        "fta": attr.label_list(
            allow_files = [".puml", ".plantuml", ".png", ".svg"],
            mandatory = False,
            doc = "Fault Tree Analysis (FTA) diagrams",
        ),
        "arch_design": attr.label(
            providers = [ArchitecturalDesignInfo],
            mandatory = False,
            doc = "Reference to architectural_design target for traceability",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def safety_analysis(
        name,
        controlmeasures = [],
        failuremodes = [],
        fta = [],
        arch_design = None,
        visibility = None):
    """Define safety analysis following S-CORE process guidelines.

    Safety analysis documents the safety-related analysis of a component,
    including failure mode and effects analysis (FMEA/FMEDA), fault tree
    analysis (FTA), and control measures that mitigate identified risks.

    Args:
        name: The name of the safety analysis target. Used as the base
            name for all generated targets.
        controlmeasures: Optional list of labels to documentation files or
            requirements targets containing control measures that mitigate
            identified failure modes. Can reference Assumptions of Use or
            requirements as defined in the S-CORE process.
        failuremodes: Optional list of labels to documentation files or
            requirements targets containing identified failure modes as
            defined in the S-CORE process.
        fta: Optional list of labels to Fault Tree Analysis diagram files
            (.puml, .plantuml, .png, .svg) as defined in the S-CORE process.
        arch_design: Optional label to an architectural_design target for
            establishing traceability between safety analysis and architecture.
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>: Main safety analysis target providing SafetyAnalysisInfo

    Example:
        ```starlark
        safety_analysis(
            name = "my_safety_analysis",
            controlmeasures = [":my_control_measures"],
            failuremodes = [":my_failure_modes"],
            fta = ["fault_tree.puml"],
            arch_design = ":my_architectural_design",
        )
        ```
    """
    _safety_analysis(
        name = name,
        controlmeasures = controlmeasures,
        failuremodes = failuremodes,
        fta = fta,
        arch_design = arch_design,
        visibility = visibility,
    )
