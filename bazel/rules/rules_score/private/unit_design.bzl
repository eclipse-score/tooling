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
design of a software unit including static and dynamic views.

PlantUML files (.puml) are passed through to Sphinx for rendering via
the sphinxcontrib-plantuml extension. The rule also invokes the PlantUML
parser pipeline to produce FlatBuffers outputs that can be consumed by
verification tooling.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo", "UnitDesignInfo")
load("//bazel/rules/rules_score/private:puml_parser.bzl", "disambiguated_stems", "run_puml_parser")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _unit_design_impl(ctx):
    """Implementation for unit_design rule.

    Collects unit design artifacts (RST documents and PlantUML diagrams) and
    provides them through the UnitDesignInfo and SphinxSourcesInfo providers.
    PlantUML files are passed through for Sphinx rendering and parsed into
    FlatBuffers binaries and idmap sidecars.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo, UnitDesignInfo, SphinxSourcesInfo
    """

    all_source_files = depset(
        transitive = [depset(ctx.files.static), depset(ctx.files.dynamic)],
    )

    # One `puml_cli` action per view; its diagrams resolve as one group and
    # the views never mix. Diagram type is autodetected.
    stems = disambiguated_stems(ctx, ctx.files.static + ctx.files.dynamic)
    static_fbs_list, _static_lobster, static_idmap_list = run_puml_parser(
        ctx,
        ctx.files.static,
        stems,
        progress_label = "{} (static)".format(ctx.label),
    )
    dynamic_fbs_list, _dynamic_lobster, dynamic_idmap_list = run_puml_parser(
        ctx,
        ctx.files.dynamic,
        stems,
        progress_label = "{} (dynamic)".format(ctx.label),
    )
    idmap_list = static_idmap_list + dynamic_idmap_list
    static_fbs = depset(static_fbs_list)
    dynamic_fbs = depset(dynamic_fbs_list)

    # Stage idmap sidecars into the sphinx sources alongside the source
    # diagrams so `clickable_plantuml` (which scans `srcdir` recursively for
    # `*.idmap.json`) can discover them - see architectural_design.bzl's
    # identical `all_idmap_files`/`sphinx_files` handling.
    sphinx_srcs = depset(
        transitive = [depset(idmap_list), all_source_files],
    )

    return [
        DefaultInfo(files = all_source_files),
        UnitDesignInfo(
            static = static_fbs,
            dynamic = dynamic_fbs,
            name = ctx.label.name,
        ),
        SphinxSourcesInfo(
            srcs = sphinx_srcs,
            deps = sphinx_srcs,
            aux_srcs = depset(),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_unit_design = rule(
    implementation = _unit_design_impl,
    doc = "Collects unit design documents and diagrams for S-CORE process compliance. " +
          "PlantUML files are passed through to Sphinx and parsed to FlatBuffers.",
    attrs = dict(
        {
            "static": attr.label_list(
                allow_files = [".puml", ".svg", ".rst", ".md"],
                mandatory = False,
                doc = "Static unit design diagrams (class diagrams, etc.). " +
                      "All diagrams of this view resolve as one group: an overview " +
                      "and its detail files merge by element id.",
            ),
            "dynamic": attr.label_list(
                allow_files = [".puml", ".svg", ".rst", ".md"],
                mandatory = False,
                doc = "Dynamic unit design diagrams (sequence diagrams, etc.). " +
                      "All diagrams of this view resolve as one group: a participant " +
                      "a scenario references but does not declare resolves to the " +
                      "declaration in a shared cast file of the view.",
            ),
            "_puml_parser": attr.label(
                default = Label("//plantuml/parser:parser"),
                executable = True,
                cfg = "exec",
                doc = "PlantUML parser tool that generates FlatBuffers from .puml files",
            ),
        },
        **VERBOSITY_ATTR
    ),
)

# ============================================================================
# Public Macro
# ============================================================================

def unit_design(
        name,
        static = [],
        dynamic = [],
        **kwargs):
    """Define unit design following S-CORE process guidelines.

    Unit design documents describe the internal design of a software unit,
    including both static and dynamic views. Static views show the structural
    organization (classes, data types), while dynamic views show the behavioral
    aspects (sequences, state transitions).

    Include the PlantUML diagram file (``.puml``) together with an RST wrapper
    that references it via ``.. uml::`` so that Sphinx renders the diagram.
    Unit design diagrams are also parsed into FlatBuffers during the build.

    Args:
        name: The name of the unit design target.
        static: Optional list of labels to diagram files (.puml,
            .svg) or documentation files (.rst, .md) containing static
            design views such as class diagrams. Include both the RST wrapper
            and the referenced .puml file(s) together.
        dynamic: Optional list of labels to diagram files (.puml,
            .svg) or documentation files (.rst, .md) containing dynamic
            design views such as sequence diagrams.
        visibility: Bazel visibility specification for the generated targets.

    RST Fragment Convention:
        Design RST files are inlined into the generated unit page via
        ``.. include::``.  Because the parent document already uses ``=``
        (section title) and ``-`` (subsection), any title inside the design
        RST **must** use a heading character that has not yet appeared — such
        as ``^`` — so it becomes a sub-subsection rather than a top-level
        section::

            My Diagram Title
            ^^^^^^^^^^^^^^^^

            .. uml:: my_diagram.puml

    Example:
        ```starlark
        unit_design(
            name = "my_unit_design",
            static = [
                "class_diagram.rst",  # RST fragment (use ^ for headings)
            ],
            dynamic = [
                "sequence_diagram.puml",
            ],
        )
        ```
    """

    _unit_design(
        name = name,
        static = static,
        dynamic = dynamic,
        **kwargs
    )
