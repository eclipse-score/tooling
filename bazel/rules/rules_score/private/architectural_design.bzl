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
Architectural Design build rules for S-CORE projects.

This module provides macros and rules for defining architectural design
documentation following S-CORE process guidelines. Architectural design
documents describe the software architecture including static and dynamic views.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo")

# ============================================================================
# Provider Definition
# ============================================================================

ArchitecturalDesignInfo = provider(
    doc = "Provider for architectural design artifacts",
    fields = {
        "static": "Depset of static architecture diagram files (e.g., class diagrams, component diagrams)",
        "dynamic": "Depset of dynamic architecture diagram files (e.g., sequence diagrams, activity diagrams)",
        "name": "Name of the architectural design target",
    },
)

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _architectural_design_impl(ctx):
    """Implementation for architectural_design rule.

    Collects architectural design artifacts including static and dynamic
    diagrams and provides them through the ArchitecturalDesignInfo provider.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo and ArchitecturalDesignInfo
    """
    static_files = depset(ctx.files.static)
    dynamic_files = depset(ctx.files.dynamic)

    # Combine all files for DefaultInfo
    all_files = depset(
        transitive = [static_files, dynamic_files],
    )

    return [
        DefaultInfo(files = all_files),
        ArchitecturalDesignInfo(
            static = static_files,
            dynamic = dynamic_files,
            name = ctx.label.name,
        ),
        SphinxSourcesInfo(
            srcs = all_files,
            transitive_srcs = all_files,
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_architectural_design = rule(
    implementation = _architectural_design_impl,
    doc = "Collects architectural design documents and diagrams for S-CORE process compliance",
    attrs = {
        "static": attr.label_list(
            allow_files = [".puml", ".plantuml", ".png", ".svg", ".rst", ".md"],
            mandatory = False,
            doc = "Static architecture diagrams (class diagrams, component diagrams, etc.)",
        ),
        "dynamic": attr.label_list(
            allow_files = [".puml", ".plantuml", ".png", ".svg", ".rst", ".md"],
            mandatory = False,
            doc = "Dynamic architecture diagrams (sequence diagrams, activity diagrams, etc.)",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def architectural_design(
        name,
        static = [],
        dynamic = [],
        visibility = None):
    """Define architectural design following S-CORE process guidelines.

    Architectural design documents describe the software architecture of a
    component, including both static and dynamic views. Static views show
    the structural organization (classes, components, modules), while dynamic
    views show the behavioral aspects (sequences, activities, states).

    Args:
        name: The name of the architectural design target. Used as the base
            name for all generated targets.
        static: Optional list of labels to diagram files (.puml, .plantuml,
            .png, .svg) or documentation files (.rst, .md) containing static
            architecture views such as class diagrams, component diagrams,
            or package diagrams as defined in the S-CORE process.
        dynamic: Optional list of labels to diagram files (.puml, .plantuml,
            .png, .svg) or documentation files (.rst, .md) containing dynamic
            architecture views such as sequence diagrams, activity diagrams,
            or state diagrams as defined in the S-CORE process.
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>: Main architectural design target providing ArchitecturalDesignInfo

    Example:
        ```starlark
        architectural_design(
            name = "my_architectural_design",
            static = [
                "class_diagram.puml",
                "component_diagram.puml",
            ],
            dynamic = [
                "sequence_diagram.puml",
                "activity_diagram.puml",
            ],
        )
        ```
    """
    _architectural_design(
        name = name,
        static = static,
        dynamic = dynamic,
        visibility = visibility,
    )
