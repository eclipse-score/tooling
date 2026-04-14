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

The rule automatically invokes the PlantUML parser on .puml/.plantuml files
to produce FlatBuffers binary representations of the parsed diagrams.
"""

load("//bazel/rules/rules_score:providers.bzl", "ArchitecturalDesignInfo", "SphinxSourcesInfo")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _run_puml_parser(ctx, puml_file):
    """Run the PlantUML parser on a single .puml file to produce a FlatBuffers binary
    and a lobster traceability file.

    The diagram type is auto-detected by the parser and encoded in the
    FlatBuffers schema (each diagram type uses its own root_type).
    Lobster output is produced in-process for component diagrams.

    Args:
        ctx: Rule context
        puml_file: The .puml File object to parse

    Returns:
        Tuple of (fbs_output, lobster_output) declared output Files.
    """
    file_stem = puml_file.basename.rsplit(".", 1)[0]
    fbs_output = ctx.actions.declare_file(
        "{}/{}.fbs.bin".format(ctx.label.name, file_stem),
    )
    lobster_output = ctx.actions.declare_file(
        "{}/{}.lobster".format(ctx.label.name, file_stem),
    )

    ctx.actions.run(
        inputs = [puml_file],
        outputs = [fbs_output, lobster_output],
        executable = ctx.executable._puml_parser,
        arguments = [
            "--file",
            puml_file.path,
            "--fbs-output-dir",
            fbs_output.dirname,
            "--lobster-output-dir",
            lobster_output.dirname,
        ],
        progress_message = "Parsing PlantUML diagram: %s" % puml_file.short_path,
    )

    return fbs_output, lobster_output

def _parse_puml_diagrams(ctx, files):
    """Run the PlantUML parser on all .puml/.plantuml files in a list.

    Args:
        ctx: Rule context
        files: List of File objects

    Returns:
        Tuple of (fbs_outputs, lobster_outputs) lists of generated Files.
    """
    fbs_outputs = []
    lobster_outputs = []
    for f in files:
        if f.extension in ("puml", "plantuml"):
            fbs, lobster = _run_puml_parser(ctx, f)
            fbs_outputs.append(fbs)
            lobster_outputs.append(lobster)
    return fbs_outputs, lobster_outputs

def _architectural_design_impl(ctx):
    """Implementation for architectural_design rule.

    Collects architectural design artifacts including static and dynamic
    diagrams, runs the PlantUML parser on .puml files to generate FlatBuffers
    binaries, and provides them through the ArchitecturalDesignInfo provider.

    The diagram type (component, class, sequence) is auto-detected by the
    parser and encoded in the FlatBuffers binary via its schema root_type.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo, ArchitecturalDesignInfo, SphinxSourcesInfo
    """

    # Parse static and dynamic diagrams separately so each provider field
    # carries the flatbuffers for its own category
    static_fbs_list, static_lobster_list = _parse_puml_diagrams(ctx, ctx.files.static)
    dynamic_fbs_list, dynamic_lobster_list = _parse_puml_diagrams(ctx, ctx.files.dynamic)
    public_api_fbs_list, public_api_lobster_list = _parse_puml_diagrams(ctx, ctx.files.public_api)

    static_fbs = depset(static_fbs_list)
    dynamic_fbs = depset(dynamic_fbs_list)
    public_api_fbs = depset(public_api_fbs_list)
    all_lobster = depset(static_lobster_list + dynamic_lobster_list + public_api_lobster_list)
    public_api_lobster = depset(public_api_lobster_list)

    # Source files for SphinxSourcesInfo (sphinx documentation pipeline)
    all_source_files = depset(
        transitive = [depset(ctx.files.static), depset(ctx.files.dynamic), depset(ctx.files.public_api)],
    )

    # Run the linker on all generated .fbs.bin files to produce a
    # plantuml_links.json for the clickable_plantuml Sphinx extension.
    all_fbs_files = static_fbs.to_list() + dynamic_fbs.to_list() + public_api_fbs.to_list()
    plantuml_links_json = ctx.actions.declare_file(
        "{}/plantuml_links.json".format(ctx.label.name),
    )
    if all_fbs_files:
        ctx.actions.run(
            inputs = all_fbs_files,
            outputs = [plantuml_links_json],
            executable = ctx.executable._linker,
            arguments = ["--fbs-files"] + [f.path for f in all_fbs_files] + ["--output", plantuml_links_json.path],
            progress_message = "Generating PlantUML links JSON for %s" % ctx.label.name,
        )
    else:
        ctx.actions.write(
            output = plantuml_links_json,
            content = '{"links":[]}',
        )

    sphinx_files = depset(
        [plantuml_links_json],
        transitive = [all_source_files],
    )

    return [
        DefaultInfo(files = all_source_files),
        ArchitecturalDesignInfo(
            static = static_fbs,
            dynamic = dynamic_fbs,
            name = ctx.label.name,
            lobster_files = all_lobster,
            public_api_lobster_files = public_api_lobster,
        ),
        # Source diagram files + plantuml_links.json for the sphinx documentation build
        SphinxSourcesInfo(
            srcs = sphinx_files,
            deps = sphinx_files,
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_architectural_design = rule(
    implementation = _architectural_design_impl,
    doc = "Collects architectural design documents and diagrams for S-CORE process compliance. " +
          "Automatically parses PlantUML files to produce FlatBuffers binary representations.",
    attrs = {
        "static": attr.label_list(
            allow_files = [".puml", ".plantuml", ".svg", ".rst", ".md"],
            mandatory = False,
            doc = "Static architecture diagrams (class diagrams, component diagrams, etc.)",
        ),
        "dynamic": attr.label_list(
            allow_files = [".puml", ".plantuml", ".svg", ".rst", ".md"],
            mandatory = False,
            doc = "Dynamic architecture diagrams (sequence diagrams, activity diagrams, etc.)",
        ),
        "public_api": attr.label_list(
            allow_files = [".puml", ".plantuml"],
            mandatory = False,
            doc = "Public API diagrams (parsed identically to static/dynamic). " +
                  "Classified separately so their lobster items are exposed via " +
                  "public_api_lobster_files, enabling failure-mode-to-interface " +
                  "traceability at the dependable element level.",
        ),
        "_puml_parser": attr.label(
            default = Label("@score_tooling//plantuml/parser:parser"),
            executable = True,
            cfg = "exec",
            doc = "PlantUML parser tool that generates FlatBuffers from .puml files",
        ),
        "_linker": attr.label(
            default = Label("@score_tooling//plantuml/parser:linker"),
            executable = True,
            cfg = "exec",
            doc = "Tool that generates plantuml_links.json from FlatBuffers diagram outputs",
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
        public_api = [],
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
        public_api: Optional list of .puml files describing the public interface
            of this element. These are parsed identically to static/dynamic
            diagrams but classified separately so their lobster items are
            exposed via public_api_lobster_files, enabling failure-mode-to-
            interface traceability at the dependable element level.
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
                "component_overview.svg",
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
        public_api = public_api,
        visibility = visibility,
    )
