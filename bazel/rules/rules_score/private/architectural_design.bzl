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
to produce FlatBuffers binary representations of the parsed diagrams. The
class diagrams of the `public_api` view also yield a `public_api.trlc` of
`ScoreReq.ApiMethod` records (one per public method name, overloads collapsed,
and one per free function), which `FailureMode.interface` references.
"""

load("@lobster//:lobster.bzl", "subrule_lobster_trlc")
load("@trlc//:trlc.bzl", "TrlcProviderInfo")
load("//bazel/rules/rules_score:providers.bzl", "ArchitecturalDesignInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:puml_parser.bzl", "disambiguated_stems", "run_puml_parser")
load("//bazel/rules/rules_score/private:puml_utils.bzl", "emit_view_navigation", "plan_view_layout", "relative_source_path")
load("//bazel/rules/rules_score/private:validation.bzl", "PROFILES", "VALIDATION_ATTRS", "run_validation")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")
load("//bazel/rules/rules_score/private:views.bzl", "ARCH_VIEWS")

# ============================================================================
# Private Rule Implementation
# ============================================================================

# Views passed as `--diagram-type` to `puml_cli`; every other view autodetects.
# `public_api` is class diagrams only: the API records come from them.
_VIEW_DIAGRAM_TYPES = {
    "public_api": "class",
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

def _split_fragments(view_files_by_name):
    """Splits `.iuml` include fragments off the files of all views.

    Args:
        view_files_by_name: Dict of view name to its List of File objects.

    Returns:
        Struct with `views` (Dict of view name to its files without
        fragments) and `fragments` (List of the `.iuml` File objects of all
        views, each once).
    """
    views = {}
    fragments = {}
    for view_name, files in view_files_by_name.items():
        views[view_name] = [f for f in files if f.extension != "iuml"]
        for f in files:
            if f.extension == "iuml":
                fragments[f.path] = f
    return struct(views = views, fragments = fragments.values())

def _fragment_error(ctx, f):
    """Why `.iuml` fragment `f` cannot be included by this target, or None.

    Args:
        ctx: Rule context.
        f: `.iuml` File object of a view.

    Returns:
        Human-readable message, or None when `f` is usable.
    """
    if not f.is_source:
        return "'{}' is generated; .iuml fragments must be source files, as the `!include` path of a diagram cannot name a generated file".format(f.short_path)
    if relative_source_path(f, ctx.label.package, ctx.label.workspace_name).startswith("../"):
        return "'{}' lives outside this repository; .iuml fragments must live in the same repository as the target".format(f.short_path)
    if _outside_package(ctx, f):
        return "'{}' lives outside package '{}'; .iuml fragments must live in the package of the target, or below it, so their `!include` paths resolve when the docs are staged".format(f.short_path, ctx.label.package)
    return None

def _outside_package(ctx, f):
    """Whether `f` is not in the package of the target or below it."""
    package = ctx.label.package
    owner_package = f.owner.package
    return bool(package) and owner_package != package and not owner_package.startswith(package + "/")

def _diagram_error(ctx, f):
    """Why diagram `f` cannot `!include` fragments of this target, or None.

    Staging keeps the package-relative path of a fragment and the full path of
    a diagram outside the package, so their relative `!include` path would
    no longer resolve.

    Args:
        ctx: Rule context.
        f: `.puml` File object of a view that has fragments.

    Returns:
        Human-readable message, or None when `f` is usable.
    """
    if _outside_package(ctx, f) or relative_source_path(f, ctx.label.package, ctx.label.workspace_name).startswith("../"):
        return "'{}' lives outside package '{}'; with `.iuml` fragments in the target, every diagram must live in the package of the target, or below it, so `!include` paths resolve when the docs are staged".format(f.short_path, ctx.label.package)
    return None

def _plan_include_staging(ctx, include_files):
    """Plan where the `.iuml` fragments stage inside one view's tree.

    A diagram's `!include` path is relative to its own directory, so each
    fragment stages at its package-relative path, beside the diagrams.

    Args:
        ctx: Rule context.
        include_files: List of `.iuml` File objects of all views.

    Returns:
        List of (File, relative_path) tuples.
    """
    return [(f, relative_source_path(f, ctx.label.package, ctx.label.workspace_name)) for f in include_files]

def _architectural_design_impl(ctx):
    """Implementation for architectural_design rule.

    Collects architectural design artifacts including static, dynamic, public
    API, and internal API diagrams, runs the PlantUML parser on .puml files to
    generate FlatBuffers binaries, and provides them through the
    ArchitecturalDesignInfo provider. The class diagrams of the public API
    view also yield the `ApiMethod` records, provided as TrlcProviderInfo.

    The `static` view accepts component diagrams only and the `public_api` view
    class diagrams only (the parser fails the action otherwise); the other
    views are auto-detected. The diagram type (component, class, sequence) is
    encoded in the FlatBuffers binary via its schema root_type.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo, ArchitecturalDesignInfo, TrlcProviderInfo, SphinxSourcesInfo
    """

    # `.iuml` files of any view are include fragments, not diagrams: every
    # view's parser action and docs tree gets all of them.
    split = _split_fragments({view_name: getattr(ctx.files, view_name) for view_name, _ in ARCH_VIEWS})
    include_files = split.fragments

    # All diagrams of this target share one flat fbs namespace
    # (keyed by ctx.label.name), so stems must be disambiguated across all
    # four views together, not per-view.
    stems = disambiguated_stems(
        ctx,
        split.views["static"] + split.views["dynamic"] + split.views["public_api"] + split.views["internal_api"],
    )

    api_package = ctx.attr.package
    has_api_diagrams = any([f.extension == "puml" for f in ctx.files.public_api])
    if has_api_diagrams and not api_package:
        fail("{}: `package` is required with `public_api` diagrams; failure mode files import it".format(ctx.label))
    if api_package and not has_api_diagrams:
        fail("{}: `package` is only used with `public_api` .puml diagrams".format(ctx.label))

    for f in include_files:
        error = _fragment_error(ctx, f)
        if error:
            fail("{}: {}".format(ctx.label, error))

    view_fbs = {}
    view_root_indexes = {}
    view_source_files = []
    view_sphinx_srcs = []
    root_index_files = []
    view_aux_docs = []
    api_trlc = None

    for view_name, root_title in ARCH_VIEWS:
        view_files = split.views[view_name]

        # One `puml_cli` action per view: its diagrams resolve as one group, so
        # `static` may split a decomposition across an overview and detail files.
        is_public_api = view_name == "public_api"
        parsed = run_puml_parser(
            ctx,
            view_files,
            stems,
            diagram_type = _VIEW_DIAGRAM_TYPES.get(view_name),
            public_api_package = api_package if is_public_api else None,
            progress_label = "{} ({})".format(ctx.label, view_name),
            include_files = include_files,
        )
        view_fbs[view_name] = parsed.fbs
        if is_public_api:
            api_trlc = parsed.public_api

        # Reconcile generated wrappers/indexes against any hand-authored
        # rst/md pages before staging anything -- see plan_view_layout for
        # the compose/override rules.
        plan = plan_view_layout(view_files, ctx.label.package, ctx.label.workspace_name)
        if plan.errors:
            fail("architectural_design {} view '{}': {}".format(ctx.label, view_name, "; ".join(plan.errors)))

        # Fragments stage beside the diagrams, so their `!include` paths
        # resolve when Sphinx renders them. They get no wrapper or navigation.
        staged = plan.staged
        if include_files and any([f.extension == "puml" for f in view_files]):
            for f in view_files:
                error = _diagram_error(ctx, f) if f.extension == "puml" else None
                if error:
                    fail("architectural_design {} view '{}': {}".format(ctx.label, view_name, error))
            staged = plan.staged + _plan_include_staging(ctx, include_files)

        # Colocate every source file of this view (diagrams, hand-written
        # rst/md pages, assets) under its own "{name}/{view}/" tree, mirroring
        # on-disk directory structure -- see _colocate_view_files for why
        # this is required for `.. uml::`/toctree sibling references to
        # resolve once dependable_element.bzl stages these files.
        view_output_dir = "{}/{}".format(ctx.label.name, view_name)
        colocated_by_relative_path = _colocate_view_files(ctx, staged, view_output_dir)
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

    # `ApiMethod` traceability exists only with public API diagrams.
    public_api_lobster = []
    if api_trlc:
        api_lobster, _ = subrule_lobster_trlc([api_trlc] + ctx.files._spec, ctx.file._api_methods_lobster_config)
        public_api_lobster = [api_lobster]

    all_source_files = depset(transitive = view_source_files)

    validation_log = _run_validation(
        ctx,
        view_fbs["static"],
        view_fbs["dynamic"],
        view_fbs["public_api"],
        view_fbs["internal_api"],
    )

    # `deps` carries everything needed in the Sphinx tree for this rule
    # (colocated sources, wrappers, and indexes at every
    # level). `srcs` is only each view's top-level root index -- the single
    # toctree entry dependable_element.bzl surfaces per view -- and
    # `aux_srcs` are the wrapper/sub-index/hand-written pages that must be
    # staged but reached only via that root index's own nested toctrees.
    sphinx_deps = depset(transitive = [all_source_files] + view_sphinx_srcs)
    sphinx_own_srcs = depset(root_index_files)
    sphinx_aux_srcs = depset(view_aux_docs)

    return [
        DefaultInfo(files = depset([validation_log.file] + ([api_trlc] if api_trlc else []), transitive = [all_source_files])),
        ArchitecturalDesignInfo(
            static = static_fbs,
            dynamic = dynamic_fbs,
            public_api = public_api_fbs,
            internal_api = internal_api_fbs,
            view_root_indexes = view_root_indexes,
            name = ctx.label.name,
            public_api_lobster_files = depset(public_api_lobster),
            validation_logs = [validation_log],
        ),
        # Lets `failure_modes` resolve `<package>.<ApiMethod>`.
        TrlcProviderInfo(
            spec = depset(ctx.files._spec),
            reqs = depset([api_trlc] if api_trlc else []),
            deps = depset(),
        ),
        # Each view's root index is the only top-level toctree entry;
        # everything else (wrappers, sub-indexes, colocated
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

# `.iuml` files are include fragments; see `_split_fragments`.
_VIEW_FILE_TYPES = [".puml", ".iuml", ".svg", ".rst", ".md"]

_FRAGMENT_DOC = " `.iuml` files are include fragments that `!include` directives of any view may name."

def _architectural_design_attrs():
    attrs = {
        "static": attr.label_list(
            allow_files = _VIEW_FILE_TYPES,
            mandatory = False,
            doc = "Static architecture diagrams -- component/package diagrams only " +
                  "(see unit_design's static attribute for class diagrams)." + _FRAGMENT_DOC,
        ),
        "dynamic": attr.label_list(
            allow_files = _VIEW_FILE_TYPES,
            mandatory = False,
            doc = "Dynamic architecture diagrams (sequence diagrams, activity diagrams, etc.)." + _FRAGMENT_DOC,
        ),
        "public_api": attr.label_list(
            allow_files = _VIEW_FILE_TYPES,
            mandatory = False,
            doc = "Public API class diagrams (.puml; the .svg/.rst/.md pages are staged as documentation). " +
                  "The diagrams yield the ScoreReq.ApiMethod records " +
                  "(one per public method name, overloads collapsed, and one " +
                  "per free function) that FailureMode.interface references; " +
                  "their lobster items are exposed via public_api_lobster_files, " +
                  "enabling failure-mode-to-interface traceability at the " +
                  "dependable element level." + _FRAGMENT_DOC,
        ),
        "internal_api": attr.label_list(
            allow_files = _VIEW_FILE_TYPES,
            mandatory = False,
            doc = "Internal API diagrams (class diagrams). " +
                  "Classified separately so their FlatBuffers outputs are exposed via " +
                  "ArchitecturalDesignInfo.internal_api for downstream validation." + _FRAGMENT_DOC,
        ),
        "maturity": attr.string(
            default = "release",
            values = ["release", "development"],
            doc = "Maturity level of the architectural design. 'release' treats validation findings as errors; 'development' emits warnings and continues.",
        ),
        "package": attr.string(
            default = "",
            doc = "TRLC package of the generated public_api.trlc (the ApiMethod records). " +
                  "Required with `public_api` diagrams; failure mode files `import` it.",
        ),
        "_spec": attr.label_list(
            allow_files = [".rsl", ".trlc"],
            default = [Label("//bazel/rules/rules_score/trlc/config:score_requirements_model")],
            doc = "S-CORE requirements model (.rsl) defining ApiMethod.",
        ),
        "_puml_parser": attr.label(
            default = Label("@score_tooling//plantuml/parser:parser"),
            executable = True,
            cfg = "exec",
            doc = "PlantUML parser tool that generates FlatBuffers/ApiMethod TRLC files from .puml files",
        ),
        "_api_methods_lobster_config": attr.label(
            default = Label("//bazel/rules/rules_score/lobster/config:api_methods_config"),
            allow_single_file = True,
            doc = "lobster-trlc YAML config for ApiMethod records.",
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
    subrules = [subrule_lobster_trlc],
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

    A `.iuml` file in any view is an include fragment, not a diagram:
    `!include` directives of the diagrams of any view may name it. It is
    neither parsed on its own nor documented, needs no `@startuml`, and must
    be a source file in the package of the target or below it, as must every
    `.puml` of a view that holds fragments. A directive's
    path is relative to the directory of the file holding it, as in PlantUML.
    Errors inside a fragment are reported at its own file and line.

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
        public_api: Optional list of .puml class diagrams describing the public
            interface of this element, classified separately from the other
            views: the diagrams yield the
            ScoreReq.ApiMethod records (one per public method name, overloads
            collapsed, and one per free function) that FailureMode.interface
            references, and their lobster items are exposed via
            public_api_lobster_files, enabling failure-mode-to-interface
            traceability at the dependable element level. A `failure_modes`
            target lists this target in its `deps` to resolve the records.
            Constructors and destructors are public methods like any other and
            get a record each.
        internal_api: Optional list of .puml files describing internal
            interfaces of this element. These are parsed identically to
            static/dynamic diagrams but classified separately so their
            FlatBuffers outputs are exposed via ArchitecturalDesignInfo.
            internal_api for downstream validation.
        maturity: Maturity level of the architectural design. Use
            "development" to write validation findings without failing the
            Bazel action.
        package: TRLC package of the generated `public_api.trlc` (passed via
            kwargs); required with `public_api` diagrams. Failure mode files
            `import` it.
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
            ] + glob(["dynamic_design/subdiagrams/*.iuml"]),
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
