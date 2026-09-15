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

load("@bazel_skylib//lib:paths.bzl", "paths")
load("//bazel/rules/rules_score:providers.bzl", "ArchitecturalDesignInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:puml_utils.bzl", "emit_view_navigation", "plan_view_layout", "relative_source_path")
load("//bazel/rules/rules_score/private:validation.bzl", "PROFILES", "VALIDATION_ATTRS", "run_validation")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

# Views recognized by architectural_design, mapped to their display name used
# as the title of that view's top-level navigation index page.
_VIEWS = {
    "static": "Static Design",
    "dynamic": "Dynamic Design",
    "public_api": "Public API",
    "internal_api": "Internal API",
}

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _disambiguated_stems(ctx, files):
    """Compute a unique output stem (no directory, no extension) for every
    .puml/.plantuml file in `files`.

    All diagrams of one architectural_design target share a flat output
    directory (keyed by ctx.label.name) for their fbs/lobster/idmap
    artifacts, so two files with the same basename but different source
    directories (e.g. two `for_impl_apis.puml` files under different
    subpackages) would otherwise collide on the same generated output path.
    When a basename is unique, the plain stem is kept unchanged (preserving
    existing filenames/titles); only colliding basenames are disambiguated,
    using the file's package-relative directory.

    Args:
        ctx: Rule context.
        files: Iterable of File objects (non-.puml/.plantuml entries ignored).
    Returns:
        Dict from File.path to a unique stem string.
    """
    puml_files = [f for f in files if f.extension in ("puml", "plantuml")]
    basename_counts = {}
    for f in puml_files:
        basename_counts[f.basename] = basename_counts.get(f.basename, 0) + 1

    stems = {}
    for f in puml_files:
        stem = f.basename.rsplit(".", 1)[0]
        if basename_counts[f.basename] > 1:
            dir_part = paths.dirname(relative_source_path(f, ctx.label.package, ctx.label.workspace_name))
            stem = "{}__{}".format(dir_part.replace("/", "_"), stem) if dir_part else stem
        stems[f.path] = stem
    return stems

def _run_puml_parser(ctx, puml_file, file_stem):
    """Run the PlantUML parser on a single .puml file to produce a FlatBuffers binary,
    a lobster traceability file, and an idmap sidecar.

    The diagram type is auto-detected by the parser and encoded in the
    FlatBuffers schema (each diagram type uses its own root_type).
    Lobster output is produced in-process for component diagrams.

    When the input file basename is not unique across all diagrams being
    parsed by this target (see _disambiguated_stems), a symlink with a
    disambiguated name is created and passed to puml_cli. This ensures
    puml_cli produces outputs with unique names even when two source
    diagrams share the same basename but live in different directories.

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
        file_stem: Unique output stem for this file (see _disambiguated_stems).
    Returns:
        Tuple of (fbs_output, lobster_output, idmap_output) declared output Files.
    """
    fbs_output = ctx.actions.declare_file(
        "{}/{}.fbs.bin".format(ctx.label.name, file_stem),
    )
    lobster_output = ctx.actions.declare_file(
        "{}/{}.lobster".format(ctx.label.name, file_stem),
    )
    idmap_output = ctx.actions.declare_file(
        "{}/{}.idmap.json".format(ctx.label.name, file_stem),
    )

    # A symlink under this target's own _puml_inputs/ dir, named after the
    # disambiguated stem, so puml_cli's output filenames (derived from input
    # basename) match the declared output files, and two architectural_design
    # targets in the same package sharing a diagram basename never collide
    # on the same _puml_inputs/ path.
    input_symlink = ctx.actions.declare_file(
        "{}/_puml_inputs/{}.{}".format(ctx.label.name, file_stem, puml_file.extension),
    )
    ctx.actions.symlink(output = input_symlink, target_file = puml_file)

    ctx.actions.run(
        inputs = [input_symlink],
        outputs = [fbs_output, lobster_output, idmap_output],
        executable = ctx.executable._puml_parser,
        arguments = [
            "--file",
            input_symlink.path,
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

def _parse_puml_diagrams(ctx, files, stems):
    """Run the PlantUML parser on all .puml/.plantuml files in a list.

    Args:
        ctx: Rule context
        files: List of File objects
        stems: Dict from File.path to unique output stem (see _disambiguated_stems).
    Returns:
        Tuple of (fbs_outputs, lobster_outputs, idmap_outputs) lists of generated Files.
    """
    fbs_outputs = []
    lobster_outputs = []
    idmap_outputs = []
    for f in files:
        if f.extension in ("puml", "plantuml"):
            fbs, lobster, idmap = _run_puml_parser(ctx, f, stems[f.path])
            fbs_outputs.append(fbs)
            lobster_outputs.append(lobster)
            idmap_outputs.append(idmap)
    return fbs_outputs, lobster_outputs, idmap_outputs

def _colocate_view_files(ctx, staged_files, view_output_dir):
    """Symlink each (source File, staged relative path) pair from a view's
    layout plan into `view_output_dir`.

    emit_view_navigation() declares wrappers and per-directory indexes under
    this same `view_output_dir`, at the paths plan_view_layout() computed.
    Any other file participating in that view's navigation -- a diagram's
    own .puml source, a hand-written .rst/.md page, or an asset such as
    .svg -- must be staged as a sibling under its identical relative path,
    or the generated `.. uml::`/toctree/`.. include::` references (resolved
    as same-directory siblings) will not resolve once dependable_element.bzl
    stages SphinxSourcesInfo files for the HTML build.

    Args:
        ctx: Rule context.
        staged_files: List of (File, relative_path) tuples -- plan.staged
            from plan_view_layout().
        view_output_dir: String prefix for declared output files, e.g.
            "{ctx.label.name}/{view_name}".

    Returns:
        List of symlinked File objects, one per (File, relative_path) pair.
    """
    colocated = []
    for source_file, relative_path in staged_files:
        copy = ctx.actions.declare_file("{}/{}".format(view_output_dir, relative_path))
        ctx.actions.symlink(output = copy, target_file = source_file)
        colocated.append(copy)
    return colocated

def _run_validation(ctx, component_fbs_files, sequence_fbs_files, public_api_fbs_files, internal_api_fbs_files):
    """Run the architectural-design validation profile.

    Args:
        ctx: Rule context
        component_fbs_files: Component-diagram FlatBuffer files generated from this target's static inputs.
        sequence_fbs_files: Sequence-diagram FlatBuffer files generated from this target's dynamic inputs.
        public_api_fbs_files: List of public-API FlatBuffer files generated from this target's public_api inputs.
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
            "public_api_diagrams": [f.path for f in public_api_fbs_files],
            "internal_api_diagrams": [f.path for f in internal_api_fbs_files],
        },
        inputs = component_fbs_files + sequence_fbs_files + public_api_fbs_files + internal_api_fbs_files,
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

    # All diagrams of this target share one flat fbs/lobster/idmap namespace
    # (keyed by ctx.label.name), so stems must be disambiguated across all
    # four views together, not per-view.
    stems = _disambiguated_stems(
        ctx,
        ctx.files.static + ctx.files.dynamic + ctx.files.public_api + ctx.files.internal_api,
    )

    view_fbs = {}
    view_fbs_files = {}
    view_lobster = {}
    view_idmap = {}
    view_indexes = {}
    view_source_files = []
    view_sphinx_srcs = []
    view_root_indexes = []
    view_aux_docs = []

    for view_name, root_title in _VIEWS.items():
        view_files = getattr(ctx.files, view_name)

        fbs_list, lobster_list, idmap_list = _parse_puml_diagrams(ctx, view_files, stems)
        view_fbs[view_name] = depset(fbs_list)
        view_fbs_files[view_name] = fbs_list
        view_lobster[view_name] = lobster_list
        view_idmap[view_name] = idmap_list

        # Reconcile generated wrappers/indexes against any hand-authored
        # rst/md pages before staging anything -- see plan_view_layout for
        # the compose/override rules.
        plan = plan_view_layout(view_files, ctx.label.package, ctx.label.workspace_name)
        if plan.errors:
            fail("architectural_design {} view '{}': {}".format(ctx.label, view_name, "; ".join(plan.errors)))

        # Colocate every source file of this view (diagrams, hand-written
        # rst/md pages, assets) under its own "{name}/{view}/" tree, mirroring
        # on-disk directory structure -- see _colocate_view_files for why
        # this is required for `.. uml::`/toctree sibling references to
        # resolve once dependable_element.bzl stages these files.
        view_output_dir = "{}/{}".format(ctx.label.name, view_name)
        colocated_files = _colocate_view_files(ctx, plan.staged, view_output_dir)
        view_source_files.append(depset(colocated_files))

        navigation = emit_view_navigation(
            ctx,
            plan,
            view_output_dir,
            ctx.file._puml_rst_template,
            root_title,
            colocated_files,
        )
        view_indexes[view_name] = navigation if navigation.root_index else None
        if navigation.root_index:
            view_sphinx_srcs.append(depset(navigation.wrappers + navigation.indexes + [navigation.root_index]))
            view_root_indexes.append(navigation.root_index)

            # Wrapper pages and non-root indexes must be staged (so the root
            # index's nested toctrees resolve) but are not themselves
            # top-level toctree entries -- only the view's root index is.
            view_aux_docs.extend(navigation.wrappers + navigation.indexes)

        # Hand-written .rst/.md pages colocated as-is (not generated
        # wrappers) are likewise reached only via the directory navigation's
        # nested toctrees, never as direct top-level entries -- except the
        # one that emit_view_navigation surfaced as the root index itself
        # (a single-file view with no generated navigation), which must
        # stay out of aux_docs or it would be staged as both a top-level
        # entry and an aux doc.
        view_aux_docs.extend([f for f in colocated_files if f.extension in ("rst", "md") and f != navigation.root_index])

    static_fbs = view_fbs["static"]
    dynamic_fbs = view_fbs["dynamic"]
    public_api_fbs = view_fbs["public_api"]
    internal_api_fbs = view_fbs["internal_api"]
    public_api_lobster = depset(view_lobster["public_api"])

    all_source_files = depset(transitive = view_source_files)

    # All idmap sidecars (across static/dynamic/public_api/internal_api) are
    # staged into the sphinx sources so the `clickable_plantuml` extension can
    # discover them (it scans `srcdir` recursively for `*.idmap.json`) and
    # resolve cross-diagram links — including component diagrams linking to
    # the class diagrams that elaborate their public/internal API interfaces.
    all_idmap_files = depset(
        view_idmap["static"] + view_idmap["dynamic"] + view_idmap["public_api"] + view_idmap["internal_api"],
    )

    sphinx_files = depset(
        transitive = [all_idmap_files, all_source_files],
    )

    validation_log = _run_validation(
        ctx,
        view_fbs_files["static"],
        view_fbs_files["dynamic"],
        view_fbs_files["public_api"],
        view_fbs_files["internal_api"],
    )

    # `deps` carries everything needed in the Sphinx tree for this rule
    # (colocated sources, idmap sidecars, wrappers, and indexes at every
    # level). `srcs` is only each view's top-level root index -- the single
    # toctree entry dependable_element.bzl surfaces per view -- and
    # `aux_srcs` are the wrapper/sub-index/hand-written pages that must be
    # staged but reached only via that root index's own nested toctrees.
    sphinx_deps = depset(transitive = [sphinx_files] + view_sphinx_srcs)
    sphinx_own_srcs = depset(view_root_indexes)
    sphinx_aux_srcs = depset(view_aux_docs)

    return [
        DefaultInfo(files = depset([validation_log.file], transitive = [all_source_files])),
        ArchitecturalDesignInfo(
            static = static_fbs,
            dynamic = dynamic_fbs,
            public_api = public_api_fbs,
            internal_api = internal_api_fbs,
            view_indexes = view_indexes,
            name = ctx.label.name,
            public_api_lobster_files = public_api_lobster,
            validation_logs = [validation_log],
        ),
        # Each view's root index is the only top-level toctree entry;
        # everything else (wrappers, sub-indexes, idmap sidecars, colocated
        # sources) is staged via aux_srcs/deps for the sphinx documentation build.
        SphinxSourcesInfo(
            srcs = sphinx_own_srcs,
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
            allow_files = [".puml", ".plantuml", ".svg", ".rst", ".md"],
            mandatory = False,
            doc = "Public API diagrams (parsed identically to static/dynamic). " +
                  "Classified separately so their lobster items are exposed via " +
                  "public_api_lobster_files, enabling failure-mode-to-interface " +
                  "traceability at the dependable element level.",
        ),
        "internal_api": attr.label_list(
            allow_files = [".puml", ".plantuml", ".svg", ".rst", ".md"],
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
        maturity = "release",
        **kwargs):
    """Define architectural design following S-CORE process guidelines.

    Architectural design documents describe the software architecture of a
    component, including both static and dynamic views. Static views show
    the structural organization (classes, components, modules), while dynamic
    views show the behavioral aspects (sequences, activities, states).

    Each view's diagrams are auto-wrapped and organized into a directory-
    matching navigation tree (one generated index.rst per source directory).
    A hand-authored index.rst/index.md placed alongside diagrams composes
    with (its text is included above) that directory's generated toctree,
    rather than being replaced by it. A hand-authored <stem>.rst/<stem>.md
    next to a same-named <stem>.puml suppresses that diagram's generated
    wrapper page, so real authored prose is always used over the generated
    placeholder. For full control over a diagram's page, omit the .puml from
    the view attribute below and reference it with your own `.. uml::`.

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
