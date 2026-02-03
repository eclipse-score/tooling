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
Dependability Analysis build rules for S-CORE projects.

This module provides macros and rules for defining dependability analysis
documentation following S-CORE process guidelines. Dependability analysis
combines safety analysis with dependent failure analysis (DFA) to provide
a comprehensive view of component reliability and safety.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:architectural_design.bzl", "ArchitecturalDesignInfo")
load("//bazel/rules/rules_score/private:safety_analysis.bzl", "SafetyAnalysisInfo")

# ============================================================================
# Provider Definition
# ============================================================================

DependabilityAnalysisInfo = provider(
    doc = "Provider for dependability analysis artifacts",
    fields = {
        "safety_analysis": "List of SafetyAnalysisInfo providers",
        "dfa": "Depset of Dependent Failure Analysis documentation",
        "fmea": "Depset of Failure Mode and Effects Analysis documentation",
        "arch_design": "ArchitecturalDesignInfo provider for linked architectural design",
        "name": "Name of the dependability analysis target",
    },
)

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _dependability_analysis_impl(ctx):
    """Implementation for dependability_analysis rule.

    Collects dependability analysis artifacts including safety analysis results
    and dependent failure analysis, linking them to architectural design.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo and DependabilityAnalysisInfo
    """
    dfa_files = depset(ctx.files.dfa)
    fmea_files = depset(ctx.files.fmea)

    # Collect safety analysis providers
    safety_analysis_infos = []
    safety_analysis_files = []
    for sa in ctx.attr.safety_analysis:
        if SafetyAnalysisInfo in sa:
            safety_analysis_infos.append(sa[SafetyAnalysisInfo])
        safety_analysis_files.append(sa.files)

    # Get architectural design provider if available
    arch_design_info = None
    if ctx.attr.arch_design and ArchitecturalDesignInfo in ctx.attr.arch_design:
        arch_design_info = ctx.attr.arch_design[ArchitecturalDesignInfo]

    # Combine all files for DefaultInfo
    all_files = depset(
        transitive = [dfa_files, fmea_files] + safety_analysis_files,
    )

    # Collect transitive sphinx sources from safety analysis and architectural design
    transitive = [all_files]
    for sa in ctx.attr.safety_analysis:
        if SphinxSourcesInfo in sa:
            transitive.append(sa[SphinxSourcesInfo].transitive_srcs)
    if ctx.attr.arch_design and SphinxSourcesInfo in ctx.attr.arch_design:
        transitive.append(ctx.attr.arch_design[SphinxSourcesInfo].transitive_srcs)

    return [
        DefaultInfo(files = all_files),
        DependabilityAnalysisInfo(
            safety_analysis = safety_analysis_infos,
            dfa = dfa_files,
            fmea = fmea_files,
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

_dependability_analysis = rule(
    implementation = _dependability_analysis_impl,
    doc = "Collects dependability analysis documents for S-CORE process compliance",
    attrs = {
        "safety_analysis": attr.label_list(
            providers = [SafetyAnalysisInfo],
            mandatory = False,
            doc = "List of safety_analysis targets containing FMEA, FMEDA, FTA results",
        ),
        "dfa": attr.label_list(
            allow_files = [".rst", ".md"],
            mandatory = False,
            doc = "Dependent Failure Analysis (DFA) documentation",
        ),
        "fmea": attr.label_list(
            allow_files = [".rst", ".md"],
            mandatory = False,
            doc = "Failure Mode and Effects Analysis (FMEA) documentation",
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

def dependability_analysis(
        name,
        safety_analysis = [],
        dfa = [],
        fmea = [],
        arch_design = None,
        visibility = None):
    """Define dependability analysis following S-CORE process guidelines.

    Dependability analysis provides a comprehensive view of component
    reliability and safety by combining safety analysis results with
    dependent failure analysis (DFA). It establishes traceability to
    the architectural design for complete safety argumentation.

    Args:
        name: The name of the dependability analysis target. Used as the base
            name for all generated targets.
        safety_analysis: Optional list of labels to safety_analysis targets
            containing the results of FMEA, FMEDA, FTA, or other safety
            analysis methods as defined in the S-CORE process.
        dfa: Optional list of labels to .rst or .md files containing
            Dependent Failure Analysis (DFA) documentation. DFA identifies
            failures that could affect multiple components or functions
            as defined in the S-CORE process.
        fmea: Optional list of labels to .rst or .md files containing
            Failure Mode and Effects Analysis (FMEA) documentation. FMEA
            identifies potential failure modes and their effects on the
            system as defined in the S-CORE process.
        arch_design: Optional label to an architectural_design target for
            establishing traceability between dependability analysis and
            the software architecture.
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>: Main dependability analysis target providing DependabilityAnalysisInfo

    Example:
        ```starlark
        dependability_analysis(
            name = "my_dependability_analysis",
            safety_analysis = [":my_safety_analysis"],
            dfa = ["dependent_failure_analysis.rst"],
            fmea = ["failure_mode_effects_analysis.rst"],
            arch_design = ":my_architectural_design",
        )
        ```
    """
    _dependability_analysis(
        name = name,
        safety_analysis = safety_analysis,
        dfa = dfa,
        fmea = fmea,
        arch_design = arch_design,
        visibility = visibility,
    )
