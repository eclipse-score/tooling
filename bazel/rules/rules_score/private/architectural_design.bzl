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
load("//bazel/rules/rules_score/private:puml_utils.bzl", "make_puml_rst_navigation")
load("//bazel/rules/rules_score/private:validation.bzl", "PROFILES", "VALIDATION_ATTRS", "run_validation")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _run_puml_parser(ctx, puml_file):
    """Run the PlantUML parser on a single .puml file to produce a FlatBuffers binary,
    a lobster traceability file, and an idmap sidecar.

    The diagram type is auto-detected by the parser and encoded in the
    FlatBuffers schema (each diagram type uses its own root_type).
    Lobster output is produced in-process for component diagrams.

    ``--source-name`` is passed as ``puml_file.short_path`` so the ``source``
    field embedded in the fbs/lobster/idmap outputs is a stable,
    workspace-relative path. This is required by the `clickable_plantuml`
    Sphinx extension, which matches idmap ``source`` keys against paths it
    derives from Sphinx's own doctree — an unstable value (e.g. a sandbox
    exec-root-relative path) would silently break cross-diagram linking.
    `BUILD_WORKSPACE_DIRECTORY`, which `puml_cli` otherwise falls back on, is
    only set for `bazel run`, never for build actions like this one.

    Args:
        ctx: Rule context
        puml_file: The .puml File object to parse
    Returns:
        Tuple of (fbs_output, lobster_output, idmap_output) declared output Files.
    """
    file_stem = puml_file.basename.rsplit(".", 1)[0]
    fbs_output = ctx.actions.declare_file(
        "{}/{}.fbs.bin".format(ctx.label.name, file_stem),
    )
    lobster_output = ctx.actions.declare_file(
        "{}/{}.lobster".format(ctx.label.name, file_stem),
    )
    idmap_output = ctx.actions.declare_file(
        "{}/{}.idmap.json".format(ctx.label.name, file_stem),
    )

    ctx.actions.run(
        inputs = [puml_file],
        outputs = [fbs_output, lobster_output, idmap_output],
        executable = ctx.executable._puml_parser,
        arguments = [
            "--file",
            puml_file.path,
            "--fbs-output-dir",
            fbs_output.dirname,
            "--lobster-output-dir",
            lobster_output.dirname,
            "--idmap-output-dir",
            idmap_output.dirname,
            "--source-name",
            puml_file.short_path,
            "--log-level",
            get_log_level(ctx),
        ],
        progress_message = "Parsing PlantUML diagram: %s" % puml_file.short_path,
    )

    return fbs_output, lobster_output, idmap_output

def _parse_puml_diagrams(ctx, files):
    """Run the PlantUML parser on all .puml/.plantuml files in a list.

    Args:
        ctx: Rule context
        files: List of File objects
    Returns:
        Tuple of (fbs_outputs, lobster_outputs, idmap_outputs) lists of generated Files.
    """
    fbs_outputs = []
    lobster_outputs = []
    idmap_outputs = []
    for f in files:
        if f.extension in ("puml", "plantuml"):
            fbs, lobster, idmap = _run_puml_parser(ctx, f)
            fbs_outputs.append(fbs)
            lobster_outputs.append(lobster)
            idmap_outputs.append(idmap)
    return fbs_outputs, lobster_outputs, idmap_outputs

def _colocate_puml_with_wrapper(ctx, puml_files, output_dir):
    """Symlink .puml/.plantuml sources next to their generated RST wrapper.

    make_puml_rst_navigation() declares each wrapper below the source diagram's
    relative directory and embeds the diagram via a same-directory sibling reference
    (``.. uml:: {basename}``). The .puml source itself, however, usually
    lives directly in this target's package -- one directory above
    `output_dir` -- not nested under it. When dependable_element.bzl later
    stages every SphinxSourcesInfo file for the HTML build, it flattens paths
    based on the *shortest common directory* across all of a label's files
    (see its `_find_common_directory`); mixing a file that sits directly in
    the package with one nested one level deeper collapses the common
    directory to the package itself, so the wrapper ends up staged one
    level deeper than the raw .puml file and the `.. uml::` sibling
    reference breaks (PlantUML file "x.puml" cannot be read). Symlinking a
    same-named copy of every diagram alongside its wrapper keeps them
    siblings under `output_dir` regardless of the diagram's original
    on-disk location, so downstream flattening logic stages them together.

    Args:
        ctx: Rule context.
        puml_files: Iterable of File objects; non-.puml/.plantuml files are
            passed through unchanged.
            output_dir: String prefix matching the one passed to
                make_puml_rst_navigation() (typically ctx.label.name).

    Returns:
        List of File objects with .puml/.plantuml entries replaced by
        same-directory symlinked copies.
    """
    colocated = []
    for f in puml_files:
        if f.extension not in ("puml", "plantuml"):
            colocated.append(f)
            continue
        relative_path = f.short_path
        package_prefix = ctx.label.package + "/" if ctx.label.package else ""
        if relative_path.startswith(package_prefix):
            relative_path = relative_path[len(package_prefix):]
        copy = ctx.actions.declare_file(
            "{}/{}".format(output_dir, relative_path),
        )
        ctx.actions.symlink(output = copy, target_file = f)
        colocated.append(copy)
    return colocated

def _run_validation(ctx, component_fbs_files, sequence_fbs_files, public_api_fbs_files, internal_api_fbs_files, static_view_fbs_files):
    """Run the architectural-design validation profile.

    Args:
        ctx: Rule context
        component_fbs_files: Component-diagram FlatBuffer files generated from this target's static inputs.
        sequence_fbs_files: Sequence-diagram FlatBuffer files generated from this target's dynamic inputs.
        public_api_fbs_files: List of public-API FlatBuffer files generated from this target's public_api inputs.
        internal_api_fbs_files: List of internal-API FlatBuffer files generated from this target's internal_api inputs.
        static_view_fbs_files: Component-diagram FlatBuffer files generated from this target's static_view inputs.
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
            "public_api_diagrams": [f.path for f in public_api_fbs_files],
            "internal_api_diagrams": [f.path for f in internal_api_fbs_files],
            "static_view": [f.path for f in static_view_fbs_files],
        },
        inputs = component_fbs_files + sequence_fbs_files + public_api_fbs_files + internal_api_fbs_files + static_view_fbs_files,
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
    static_fbs_list, static_lobster_list, static_idmap_list = _parse_puml_diagrams(ctx, ctx.files.static)
    dynamic_fbs_list, dynamic_lobster_list, dynamic_idmap_list = _parse_puml_diagrams(ctx, ctx.files.dynamic)
    public_api_fbs_list, public_api_lobster_list, public_api_idmap_list = _parse_puml_diagrams(ctx, ctx.files.public_api)
    internal_api_fbs_list, _internal_api_lobster_list, internal_api_idmap_list = _parse_puml_diagrams(ctx, ctx.files.internal_api)
    static_view_fbs_list, _static_view_lobster_list, static_view_idmap_list = _parse_puml_diagrams(ctx, ctx.files.static_view)

    static_fbs = depset(static_fbs_list)
    dynamic_fbs = depset(dynamic_fbs_list)
    public_api_fbs = depset(public_api_fbs_list)
    internal_api_fbs = depset(internal_api_fbs_list)
    public_api_lobster = depset(public_api_lobster_list)
    static_view_fbs = depset(static_view_fbs_list)

    # Source files for SphinxSourcesInfo (sphinx documentation pipeline).
    # .puml/.plantuml sources are colocated (symlinked) next to their
    # generated RST wrapper -- see _colocate_puml_with_wrapper for why this
    # is required for the `.. uml::` sibling reference to resolve once
    # dependable_element.bzl stages these files for the HTML build.
    all_source_files = depset(
        transitive = [
            depset(_colocate_puml_with_wrapper(ctx, ctx.files.static, ctx.label.name)),
            depset(_colocate_puml_with_wrapper(ctx, ctx.files.dynamic, ctx.label.name)),
            depset(_colocate_puml_with_wrapper(ctx, ctx.files.public_api, ctx.label.name)),
            depset(_colocate_puml_with_wrapper(ctx, ctx.files.internal_api, ctx.label.name)),
            depset(_colocate_puml_with_wrapper(ctx, ctx.files.static_view, ctx.label.name)),
        ],
    )

    # All idmap sidecars (across static/dynamic/public_api/internal_api/static_view) are
    # staged into the sphinx sources so the `clickable_plantuml` extension can
    # discover them (it scans `srcdir` recursively for `*.idmap.json`) and
    # resolve cross-diagram links — including component diagrams linking to
    # the class diagrams that elaborate their public/internal API interfaces.
    all_idmap_files = depset(
        static_idmap_list + dynamic_idmap_list + public_api_idmap_list + internal_api_idmap_list + static_view_idmap_list,
    )

    sphinx_files = depset(
        transitive = [all_idmap_files, all_source_files],
    )

    # Generate path-preserving wrappers and directory index pages. The root
    # index is the only direct entry in the dependable_element index; nested
    # indexes and wrappers are reached through its toctrees.
    navigation = make_puml_rst_navigation(
        ctx,
        ctx.files.static + ctx.files.dynamic + ctx.files.public_api + ctx.files.internal_api + ctx.files.static_view,
        ctx.label.name,
        ctx.file._puml_rst_template,
    )

    validation_log = _run_validation(
        ctx,
        static_fbs_list,
        dynamic_fbs_list,
        public_api_fbs_list,
        internal_api_fbs_list,
        static_view_fbs_list,
    )

    sphinx_srcs = depset([navigation.root_index])
    sphinx_aux_srcs = depset(navigation.wrappers + navigation.indexes)
    sphinx_deps = depset(transitive = [sphinx_files, sphinx_srcs, sphinx_aux_srcs])

    return [
        DefaultInfo(files = depset([validation_log.file], transitive = [all_source_files])),
        ArchitecturalDesignInfo(
            static = static_fbs,
            dynamic = dynamic_fbs,
            public_api = public_api_fbs,
            internal_api = internal_api_fbs,
            static_view = static_view_fbs,
            name = ctx.label.name,
            public_api_lobster_files = public_api_lobster,
            validation_logs = [validation_log],
        ),
        # Source diagram files + *.idmap.json sidecars for the sphinx documentation build
        SphinxSourcesInfo(
            srcs = sphinx_srcs,
            deps = sphinx_deps,
            aux_srcs = sphinx_aux_srcs,
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
        "static_view": attr.label_list(
            allow_files = [".puml", ".plantuml"],
            mandatory = False,
            doc = "Component diagrams that present a partial view of the static architecture. " +
                  "Parsed identically to `static`, but never used to define the units/components " +
                  "validated against the Bazel component graph. Instead, every component/unit " +
                  "defined here must also be defined, under the same parent, in `static`; " +
                  "the build fails if a `static_view` diagram introduces a component/unit that is " +
                  "not present in the `static` diagrams.",
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
            doc = "PlantUML parser tool that generates FlatBuffers/lobster/idmap files from .puml files",
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

# ============================================================================
# Public Macro
# ============================================================================

def architectural_design(
        name,
        static = [],
        dynamic = [],
        public_api = [],
        internal_api = [],
        static_view = [],
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
        static_view: Optional list of .puml component diagrams that present a
            partial view of the static architecture. These are parsed
            identically to `static`, but are not used to define the
            units/components validated against the Bazel component graph.
            Instead, every component/unit defined in a `static_view` diagram
            must also be defined, under the same parent, in `static`: it may
            only contain a subset of the units/components of the matching
            `static` diagram. The build fails if a `static_view` diagram
            introduces a component/unit that is not present in `static`.
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
        static_view = static_view,
        maturity = maturity,
        **kwargs
    )
