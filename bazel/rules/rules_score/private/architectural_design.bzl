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

The rule automatically invokes the PlantUML parser on .puml files
to produce FlatBuffers binary representations of the parsed diagrams.
"""

load("//bazel/rules/rules_score:providers.bzl", "ArchitecturalDesignInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:puml_parser.bzl", "disambiguated_stems", "run_puml_parser")
load("//bazel/rules/rules_score/private:puml_utils.bzl", "emit_view_navigation", "plan_view_layout")
load("//bazel/rules/rules_score/private:validation.bzl", "PROFILES", "VALIDATION_ATTRS", "run_validation")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")
load("//bazel/rules/rules_score/private:views.bzl", "ARCH_VIEWS")

# ============================================================================
# Private Rule Implementation
# ============================================================================

# Views passed as `--diagram-type` to `puml_cli`; every other view autodetects.
_VIEW_DIAGRAM_TYPES = {
    "static": "component",
}

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
        Dict from relative_path to its symlinked File object, one entry per
        (File, relative_path) pair in `staged_files`.
    """
    colocated = {}
    for source_file, relative_path in staged_files:
        copy = ctx.actions.declare_file("{}/{}".format(view_output_dir, relative_path))
        ctx.actions.symlink(output = copy, target_file = source_file)
        colocated[relative_path] = copy
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
    stems = disambiguated_stems(
        ctx,
        ctx.files.static + ctx.files.dynamic + ctx.files.public_api + ctx.files.internal_api,
    )

    view_fbs = {}
    view_lobster = {}
    view_idmap = {}
    view_root_indexes = {}
    view_source_files = []
    view_sphinx_srcs = []
    root_index_files = []
    view_aux_docs = []

    for view_name, root_title in ARCH_VIEWS:
        view_files = getattr(ctx.files, view_name)

        # One `puml_cli` action per view: its diagrams resolve as one group, so
        # `static` may split a decomposition across an overview and detail files.
        fbs_list, lobster_list, idmap_list = run_puml_parser(
            ctx,
            view_files,
            stems,
            diagram_type = _VIEW_DIAGRAM_TYPES.get(view_name),
            emit_lobster = True,
            progress_label = "{} ({})".format(ctx.label, view_name),
        )
        view_fbs[view_name] = fbs_list
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
        colocated_by_relative_path = _colocate_view_files(ctx, plan.staged, view_output_dir)
        colocated_files = colocated_by_relative_path.values()
        view_source_files.append(depset(colocated_files))

        navigation = emit_view_navigation(
            ctx,
            plan,
            view_output_dir,
            ctx.file._puml_rst_template,
            root_title,
            colocated_by_relative_path,
        )
        view_root_indexes[view_name] = navigation.root_index
        if navigation.root_index:
            view_sphinx_srcs.append(depset(navigation.wrappers + navigation.indexes + [navigation.root_index]))
            root_index_files.append(navigation.root_index)

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

    static_fbs = depset(view_fbs["static"])
    dynamic_fbs = depset(view_fbs["dynamic"])
    public_api_fbs = depset(view_fbs["public_api"])
    internal_api_fbs = depset(view_fbs["internal_api"])
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
        view_fbs["static"],
        view_fbs["dynamic"],
        view_fbs["public_api"],
        view_fbs["internal_api"],
    )

    # `deps` carries everything needed in the Sphinx tree for this rule
    # (colocated sources, idmap sidecars, wrappers, and indexes at every
    # level). `srcs` is only each view's top-level root index -- the single
    # toctree entry dependable_element.bzl surfaces per view -- and
    # `aux_srcs` are the wrapper/sub-index/hand-written pages that must be
    # staged but reached only via that root index's own nested toctrees.
    sphinx_deps = depset(transitive = [sphinx_files] + view_sphinx_srcs)
    sphinx_own_srcs = depset(root_index_files)
    sphinx_aux_srcs = depset(view_aux_docs)

    return [
        DefaultInfo(files = depset([validation_log.file], transitive = [all_source_files])),
        ArchitecturalDesignInfo(
            static = static_fbs,
            dynamic = dynamic_fbs,
            public_api = public_api_fbs,
            internal_api = internal_api_fbs,
            view_root_indexes = view_root_indexes,
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
            allow_files = [".puml", ".svg", ".rst", ".md"],
            mandatory = False,
            doc = "Static architecture diagrams -- component/package diagrams only " +
                  "(see unit_design's static attribute for class diagrams).",
        ),
        "dynamic": attr.label_list(
            allow_files = [".puml", ".svg", ".rst", ".md"],
            mandatory = False,
            doc = "Dynamic architecture diagrams (sequence diagrams, activity diagrams, etc.)",
        ),
        "public_api": attr.label_list(
            allow_files = [".puml", ".svg", ".rst", ".md"],
            mandatory = False,
            doc = "Public API diagrams (parsed identically to static/dynamic). " +
                  "Classified separately so their lobster items are exposed via " +
                  "public_api_lobster_files, enabling failure-mode-to-interface " +
                  "traceability at the dependable element level.",
        ),
        "internal_api": attr.label_list(
            allow_files = [".puml", ".svg", ".rst", ".md"],
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
    component: static views (component/package diagrams) and dynamic
    views (sequence/activity/state diagrams), plus public/internal API
    diagrams. Each view's diagrams are auto-wrapped into a generated,
    directory-matching navigation tree; hand-authored index/`<stem>` pages
    compose with or override that generated navigation -- see
    docs/user_guide/architectural_design.rst for the full authoring-mode
    reference.

    Args:
        name: The name of the architectural design target. Used as the base
            name for all generated targets.
        static: Optional list of labels to diagram files (.puml,
            .png, .svg) or documentation files (.rst, .md) containing static
            architecture views -- component diagrams or package diagrams as
            defined in the S-CORE process. Component diagrams only: class
            diagrams belong in the elaborating unit's own
            unit_design(static=[...]) instead (see unit_design.bzl), never
            here. All `static` files resolve together as one group: a
            top-level element with a body in one file is reparented onto the
            nested declaration it matches in another file.
        dynamic: Optional list of labels to diagram files (.puml,
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
