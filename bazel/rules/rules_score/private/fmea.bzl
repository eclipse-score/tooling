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
FMEA (Failure Mode and Effects Analysis) build rules for S-CORE projects.

This module provides macros and rules for defining FMEA documentation
following S-CORE process guidelines. FMEA documents failure modes, control
measures, and root-cause fault tree analysis diagrams for a component.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:architectural_design.bzl", "ArchitecturalDesignInfo")

# ============================================================================
# Provider Definition
# ============================================================================

AnalysisInfo = provider(
    doc = "Provider for FMEA and safety analysis artifacts",
    fields = {
        "controlmeasures": "Depset of control measures documentation or requirements",
        "failuremodes": "Depset of failure modes documentation or requirements",
        "fta": "Depset of Fault Tree Analysis diagrams",
        "arch_design": "ArchitecturalDesignInfo provider for linked architectural design",
        "name": "Name of the analysis target",
    },
)

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _fmea_impl(ctx):
    """Implementation for fmea rule.

    Collects FMEA artifacts including failure modes, control measures, and
    fault tree diagrams, linking them to architectural design.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo, AnalysisInfo, SphinxSourcesInfo
    """
    controlmeasures = depset(ctx.files.controlmeasures)
    failuremodes = depset(ctx.files.failuremodes)
    fta = depset(ctx.files.root_causes)

    # TODO: render requirement sources (failuremodes, controlmeasures) into
    # documentation and extract traceability artifacts.

    # TODO: preprocess fault tree diagrams (root_causes) and extract
    # traceability artifacts.

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
        AnalysisInfo(
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

_fmea = rule(
    implementation = _fmea_impl,
    doc = "Collects FMEA documents and diagrams for S-CORE process compliance",
    attrs = {
        "failuremodes": attr.label_list(
            allow_files = [".rst", ".md", ".trlc"],
            mandatory = False,
            doc = "Failure modes documentation or requirements targets",
        ),
        "controlmeasures": attr.label_list(
            allow_files = [".rst", ".md", ".trlc"],
            mandatory = False,
            doc = "Control measures documentation or requirements targets (can be AoUs or requirements)",
        ),
        "root_causes": attr.label_list(
            allow_files = [".puml", ".plantuml", ".png", ".svg"],
            mandatory = False,
            doc = "Root-cause Fault Tree Analysis (FTA) diagrams",
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

def fmea(
        name,
        failuremodes = [],
        controlmeasures = [],
        root_causes = [],
        arch_design = None,
        visibility = None):
    """Define FMEA documentation following S-CORE process guidelines.

    FMEA (Failure Mode and Effects Analysis) documents the failure modes of a
    component, the control measures that mitigate them, and optional fault tree
    analysis diagrams that trace root causes.

    Args:
        name: The name of the FMEA target.
        failuremodes: Optional list of labels to documentation files or
            requirements targets containing identified failure modes.
        controlmeasures: Optional list of labels to documentation files or
            requirements targets containing control measures that mitigate
            identified failure modes. Can reference Assumptions of Use or
            requirements as defined in the S-CORE process.
        root_causes: Optional list of labels to Fault Tree Analysis diagram
            files (.puml, .plantuml, .png, .svg) tracing root causes.
        arch_design: Optional label to an architectural_design target for
            establishing traceability between the FMEA and the architecture.
        visibility: Bazel visibility specification. Default is None.
    """
    _fmea(
        name = name,
        failuremodes = failuremodes,
        controlmeasures = controlmeasures,
        root_causes = root_causes,
        arch_design = arch_design,
        visibility = visibility,
    )
