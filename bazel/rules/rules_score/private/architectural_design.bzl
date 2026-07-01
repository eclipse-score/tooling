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
load("//bazel/rules/rules_score/private:puml_utils.bzl", "make_puml_rst_wrappers")
load("//bazel/rules/rules_score/private:validation.bzl", "PROFILES", "VALIDATION_ATTRS", "run_validation")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

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
    json_output = ctx.actions.declare_file(
        "{}/{}.logic.ast.json".format(ctx.label.name, file_stem),
    )

    ctx.actions.run(
        inputs = [puml_file],
        outputs = [fbs_output, lobster_output, json_output],
        executable = ctx.executable._puml_parser,
        arguments = [
            "--file",
            puml_file.path,
            "--fbs-output-dir",
            fbs_output.dirname,
            "--lobster-output-dir",
            lobster_output.dirname,
            "--log-level",
            # gToDo: log level needs to be debug for ast.json to be generated
            "debug",
            # get_log_level(ctx),
        ],
        progress_message = "Parsing PlantUML diagram: %s" % puml_file.short_path,
    )

    return fbs_output, lobster_output, json_output

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

def _run_validation(ctx, component_fbs_files, sequence_fbs_files, internal_api_fbs_files):
    """Run the architectural-design validation profile.

    Args:
        ctx: Rule context
        component_fbs_files: Component-diagram FlatBuffer files generated from this target's static inputs.
        sequence_fbs_files: Sequence-diagram FlatBuffer files generated from this target's dynamic inputs.
        internal_api_fbs_files: List of internal-API FlatBuffer files generated from this target's internal_api inputs.
    Returns:
        Struct with file and name fields describing the validation log entry.
    """

    return run_validation(
        ctx = ctx,
        validation_cli = ctx.executable._validation_cli,
        profile = PROFILES.ARCHITECTURAL_DESIGN,
        input_bundle = {
            "component_diagrams": [f.path for f in component_fbs_files],
            "sequence_diagrams": [f.path for f in sequence_fbs_files],
            "internal_api_diagrams": [f.path for f in internal_api_fbs_files],
        },
        inputs = component_fbs_files + sequence_fbs_files + internal_api_fbs_files,
        mnemonic = "ArchitecturalDesignValidate",
        maturity = ctx.attr.maturity,
        log_level = get_log_level(ctx),
    )

def _architectural_design_impl(ctx):
    """Implementation for architectural_design rule.

    Collects architectural design artifacts including static, dynamic, public
    API, and internal API diagrams, runs the PlantUML parser on .puml files to
    generate FlatBuffers binaries, and provides them through the
    ArchitecturalDesignInfo provider.

    The diagram type (component, class, sequence) is auto-detected by the
    parser and encoded in the FlatBuffers binary via its schema root_type.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo, ArchitecturalDesignInfo, SphinxSourcesInfo
    """

    # Parse each architectural view separately so each provider field carries
    # the flatbuffers for its own category.
    static_fbs_list, static_lobster_list = _parse_puml_diagrams(ctx, ctx.files.static)
    dynamic_fbs_list, dynamic_lobster_list = _parse_puml_diagrams(ctx, ctx.files.dynamic)
    public_api_fbs_list, public_api_lobster_list = _parse_puml_diagrams(ctx, ctx.files.public_api)
    internal_api_fbs_list, _internal_api_lobster_list = _parse_puml_diagrams(ctx, ctx.files.internal_api)

    static_fbs = depset(static_fbs_list)
    dynamic_fbs = depset(dynamic_fbs_list)
    public_api_fbs = depset(public_api_fbs_list)
    internal_api_fbs = depset(internal_api_fbs_list)
    public_api_lobster = depset(public_api_lobster_list)

    # Source files for SphinxSourcesInfo (sphinx documentation pipeline)
    all_source_files = depset(
        transitive = [depset(ctx.files.static), depset(ctx.files.dynamic), depset(ctx.files.public_api), depset(ctx.files.internal_api)],
    )

    # Run the linker on all generated .fbs.bin files to produce a
    # plantuml_links.json for the clickable_plantuml Sphinx extension.
    all_fbs_files = static_fbs.to_list() + dynamic_fbs.to_list() + public_api_fbs.to_list() + internal_api_fbs.to_list()
    plantuml_links_json = ctx.actions.declare_file(
        "{}/plantuml_links.json".format(ctx.label.name),
    )
    if all_fbs_files:
        ctx.actions.run(
            inputs = all_fbs_files,
            outputs = [plantuml_links_json],
            executable = ctx.executable._linker,
            arguments = ["--fbs-files"] + [f.path for f in all_fbs_files] + ["--output", plantuml_links_json.path, "--log-level", get_log_level(ctx)],
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

    # Generate a thin RST wrapper for every .puml diagram so it appears as a
    # toctree entry in the dependable_element index.
    rst_wrappers = make_puml_rst_wrappers(
        ctx,
        ctx.files.static + ctx.files.dynamic + ctx.files.public_api + ctx.files.internal_api,
        ctx.label.name,
        ctx.file._puml_rst_template,
    )

    validation_log = _run_validation(
        ctx,
        static_fbs_list,
        dynamic_fbs_list,
        internal_api_fbs_list,
    )

    sphinx_srcs = depset(rst_wrappers, transitive = [sphinx_files])

    return [
        DefaultInfo(files = depset([validation_log.file], transitive = [all_source_files])),
        ArchitecturalDesignInfo(
            static = static_fbs,
            dynamic = dynamic_fbs,
            internal_api = internal_api_fbs,
            name = ctx.label.name,
            public_api_lobster_files = public_api_lobster,
            validation_logs = [validation_log],
        ),
        # Source diagram files + plantuml_links.json for the sphinx documentation build
        SphinxSourcesInfo(
            srcs = sphinx_srcs,
            deps = sphinx_srcs,
            aux_srcs = depset(),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

def _architectural_design_attrs():
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
        "internal_api": attr.label_list(
            allow_files = [".puml", ".plantuml"],
            mandatory = False,
            doc = "Internal API diagrams (class diagrams). " +
                  "Classified separately so their FlatBuffers outputs are exposed via " +
                  "ArchitecturalDesignInfo.internal_api for downstream validation.",
        ),
        "maturity": attr.string(
            default = "release",
            values = ["release", "development"],
            doc = "Maturity level of the architectural design. 'release' treats validation findings as errors; 'development' emits warnings and continues.",
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
        "_puml_rst_template": attr.label(
            default = Label("//bazel/rules/rules_score:templates/puml_diagram.template.rst"),
            allow_single_file = True,
            doc = "RST template for PlantUML diagram wrapper pages.",
        ),
    }
    attrs.update(VALIDATION_ATTRS)
    attrs.update(VERBOSITY_ATTR)
    return attrs

_architectural_design = rule(
    implementation = _architectural_design_impl,
    doc = "Collects architectural design documents and diagrams for S-CORE process compliance. " +
          "Automatically parses PlantUML files to produce FlatBuffers binary representations.",
    attrs = _architectural_design_attrs(),
)
def _parse_puml_diagrams_impl(ctx):
    print(ctx.attr.files)
    print(ctx.files.files[0])
    # gToDo: shaky mit [0]
    fbs_output, lobster_output, json_output = _run_puml_parser(ctx, ctx.files.files[0])
    return [
        DefaultInfo(
            files = depset([fbs_output, lobster_output, json_output]),
        ),
    ]

parse_puml_diagrams = rule(
    implementation = _parse_puml_diagrams_impl,
    doc = "Helper rule to run the PlantUML parser on a list of .puml files and produce FlatBuffers binaries and lobster files.",
    attrs = {
        "files": attr.label(
            mandatory = True,
            doc = "List of .puml/.plantuml files to parse",
            allow_single_file = True,
        ),
        "_puml_parser": attr.label(
            default = Label("@score_tooling//plantuml/parser:parser"),
            executable = True,
            cfg = "exec",
            doc = "PlantUML parser tool that generates FlatBuffers from .puml files",
        ),
        "_verbosity": attr.label(
            default = Label("//bazel/rules/rules_score:verbosity"),
            doc = "Verbosity level build setting (warn/info/debug).",
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
        internal_api = [],
        maturity = "release",
        **kwargs):
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
        internal_api: Optional list of .puml files describing internal
            interfaces of this element. These are parsed identically to
            static/dynamic diagrams but classified separately so their
            FlatBuffers outputs are exposed via ArchitecturalDesignInfo.
            internal_api for downstream validation.
        maturity: Maturity level of the architectural design. Use
            "development" to write validation findings without failing the
            Bazel action.
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
            internal_api = ["internal_api.puml"],
        )
        ```
    """

    _architectural_design(
        name = name,
        static = static,
        dynamic = dynamic,
        public_api = public_api,
        internal_api = internal_api,
        maturity = maturity,
        **kwargs
    )
