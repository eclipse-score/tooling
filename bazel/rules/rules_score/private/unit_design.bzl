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
Unit Design build rules for S-CORE projects.

This module provides macros and rules for defining unit design documentation
following S-CORE process guidelines. Unit design documents describe the internal
design of a software unit, including static and dynamic views.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo", "UnitDesignInfo")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _unit_design_impl(ctx):
    """Implementation for unit_design rule.

    Collects unit design artifacts (RST documents and diagram files) and
    provides them through the UnitDesignInfo and SphinxSourcesInfo providers.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo, UnitDesignInfo, SphinxSourcesInfo
    """
    all_source_files = depset(
        transitive = [depset(ctx.files.static), depset(ctx.files.dynamic)],
    )

    # TODO: invoke diagram parser here to produce structured design artifacts
    # for documentation and traceability (e.g. generate parsed binary
    # representations of the diagrams for downstream analysis tools).
    static_fbs = depset()
    dynamic_fbs = depset()

    return [
        DefaultInfo(files = all_source_files),
        UnitDesignInfo(
            static = static_fbs,
            dynamic = dynamic_fbs,
            name = ctx.label.name,
        ),
        SphinxSourcesInfo(
            srcs = all_source_files,
            transitive_srcs = all_source_files,
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_unit_design = rule(
    implementation = _unit_design_impl,
    doc = "Collects unit design documents and diagrams for S-CORE process compliance.",
    attrs = {
        "static": attr.label_list(
            allow_files = [".puml", ".plantuml", ".svg", ".rst", ".md"],
            doc = "Static design views (e.g., class diagrams, state machine diagrams).",
        ),
        "dynamic": attr.label_list(
            allow_files = [".puml", ".plantuml", ".svg", ".rst", ".md"],
            doc = "Dynamic design views (e.g., sequence diagrams, activity diagrams).",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def unit_design(
        name,
        static = [],
        dynamic = [],
        visibility = None):
    """Define unit design documentation following S-CORE process guidelines.

    A unit design describes the internal design of a software unit. It consists
    of static views (e.g., class diagrams) and dynamic views (e.g., sequence
    diagrams).

    Args:
        name: The name of the unit design target.
        static: List of labels to static design view files (.puml, .plantuml,
            .svg, .rst, .md). Default is empty list.
        dynamic: List of labels to dynamic design view files (.puml, .plantuml,
            .svg, .rst, .md). Default is empty list.
        visibility: Bazel visibility specification. Default is None.
    """
    _unit_design(
        name = name,
        static = static,
        dynamic = dynamic,
        visibility = visibility,
    )
