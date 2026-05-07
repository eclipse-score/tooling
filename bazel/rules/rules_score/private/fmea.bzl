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
FMEA (Failure Mode and Effects Analysis) build rules for S-CORE projects.

The rule generates a single ``fmea.rst`` page (expanded from a
template) with three inline sections:

  1. **Failure Modes** – TRLC failure-mode targets are rendered to ``.inc``
     files by trlc_rst and pulled in via ``.. include::``.
  2. **Control Measures** – TRLC control-measure targets rendered to ``.inc``
     the same way as failure modes.
  3. **Root Causes** – optional FTA PlantUML diagrams (``.puml`` /
     ``.plantuml``) given via the ``root_causes`` attribute.  Each diagram
     is preprocessed to inline ``fta_metamodel.puml`` (making it
     self-contained) and then referenced via ``.. uml::`` inside the page.
     Lobster traceability items are extracted to ``{label}/root_causes.lobster``.

Using ``.inc`` (not ``.rst``) for the helper include files keeps them out of
the Sphinx toctree (``_is_document_file`` only matches ``.rst``/``.md``) while
``_filter_doc_files`` in ``dependable_element.bzl`` still symlinks them
alongside ``fmea.rst`` so ``.. include::`` resolves at build time.

``AnalysisInfo`` carries all lobster traceability files (failuremodes,
controlmeasures, and root_causes if present) as a ``lobster_files`` dict
keyed by canonical filename (e.g. ``{"failuremodes.lobster": File, ...}``).
All Sphinx source files travel via ``SphinxSourcesInfo``.

This is a **build-only** rule.  The combined traceability *test* is owned
by the ``dependability_analysis`` rule which wraps this one.
"""

load("//bazel/rules/rules_score:providers.bzl", "AnalysisInfo", "ArchitecturalDesignInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:puml_utils.bzl", "make_puml_rst_wrappers")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

# ============================================================================
# Root-cause (FTA) processing helper
# ============================================================================

def _process_root_causes(ctx):
    """Preprocess FTA diagrams (inline metamodel) and extract lobster items in one action.

    Args:
        ctx: Rule context.  Reads ``ctx.files.root_causes``,
             ``ctx.file._fta_metamodel``, and
             ``ctx.executable._safety_analysis_tools``.

    Returns:
        Tuple ``(preprocessed_diagrams, detail_rsts, [root_causes_lobster], rst_section_text)``.
        All lists are empty and the section text is ``""`` when there are no
        PlantUML root-cause inputs.
    """
    puml_inputs = [
        f
        for f in ctx.files.root_causes
        if f.extension in ("puml", "plantuml")
    ]
    if not puml_inputs:
        return [], [], [], ""

    # Declare one preprocessed output per input diagram (same directory).
    preprocessed_diagrams = [
        ctx.actions.declare_file("{}/{}".format(ctx.label.name, src.basename))
        for src in puml_inputs
    ]
    root_causes_lobster = ctx.actions.declare_file(
        "{}/root_causes.lobster".format(ctx.label.name),
    )

    # Single action: preprocess every diagram and extract lobster traceability.
    output_dir = preprocessed_diagrams[0].dirname
    args = ctx.actions.args()
    args.add("--metamodel", ctx.file._fta_metamodel)
    args.add("--output-dir", output_dir)
    args.add("--lobster", root_causes_lobster)
    args.add("--log-level", get_log_level(ctx))
    args.add_all(puml_inputs)
    ctx.actions.run(
        inputs = puml_inputs + [ctx.file._fta_metamodel],
        outputs = preprocessed_diagrams + [root_causes_lobster],
        executable = ctx.executable._safety_analysis_tools,
        arguments = [args],
        progress_message = "Processing root cause FTA diagrams for %s" % ctx.label.name,
    )

    # Generate one detail RST per preprocessed FTA diagram via the shared
    # puml_diagram template.  The "fta_" prefix is stripped from the stem so
    # the page is titled e.g. "Server Not Listening" instead of
    # "Fta Server Not Listening".
    detail_rsts = make_puml_rst_wrappers(
        ctx,
        preprocessed_diagrams,
        ctx.label.name,
        ctx.file._puml_rst_template,
        strip_prefix = "fta_",
        filename_prefix = "detail_",
    )

    # Build toctree entries directly from the declared RST wrapper filenames so
    # the toctree is always consistent with what make_puml_rst_wrappers produces,
    # regardless of any prefix convention on the input files.
    toctree_entries = [
        "   " + rst.basename[:-4]  # strip ".rst"
        for rst in detail_rsts
    ]

    root_causes_rst_section = (
        "Root Cause Analysis\n-------------------\n\n" +
        ".. toctree::\n   :maxdepth: 1\n\n" +
        "\n".join(toctree_entries) + "\n"
    )

    return preprocessed_diagrams, detail_rsts, [root_causes_lobster], root_causes_rst_section

# ============================================================================
# Private Helpers
# ============================================================================

def _render_trlc_inc(ctx, trlc_files, spec_files, out_name):
    """Render a list of ``.trlc`` source files to an ``.inc`` file via trlc_rst.

    The ``.inc`` extension means the file is symlinked into the output
    directory (via ``_filter_doc_files``) but is NOT added to any Sphinx
    toctree (``_is_document_file`` only matches ``.rst`` / ``.md``).

    Args:
        ctx:        Rule context.
        trlc_files: List of ``.trlc`` File objects to render.
        spec_files: List of ``.rsl`` spec File objects needed for TRLC import
                    resolution (passed as sandbox inputs only).
        out_name:   Output filename (e.g. ``"failuremodes.inc"``).

    Returns:
        Declared ``.inc`` output File inside ``{label.name}/``, or ``None``
        when ``trlc_files`` is empty.
    """
    if not trlc_files:
        return None
    rendered = ctx.actions.declare_file(
        "{}/{}".format(ctx.label.name, out_name),
    )
    args = ctx.actions.args()
    args.add("--output", rendered.path)
    args.add("--input-dir", ".")
    args.add("--title", "")
    args.add("--source-files")
    args.add_all(trlc_files)
    ctx.actions.run(
        inputs = trlc_files + spec_files,
        outputs = [rendered],
        arguments = [args],
        executable = ctx.executable._renderer,
    )
    return rendered

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _fmea_impl(ctx):
    output_files = []

    # -------------------------------------------------------------------------
    # 0. Process root causes (FTA diagrams) if provided
    # -------------------------------------------------------------------------
    preprocessed_diagrams, detail_rsts, root_cause_lobster_files, root_causes_rst_section = _process_root_causes(ctx)
    output_files.extend(preprocessed_diagrams)
    output_files.extend(detail_rsts)

    # -------------------------------------------------------------------------
    # 1. Render failure modes: TRLC -> .inc via trlc_rst
    # -------------------------------------------------------------------------
    spec_files = ctx.files.spec
    fm_inc = _render_trlc_inc(ctx, ctx.files.failuremodes, spec_files, "failuremodes.inc")
    failuremodes_inc = [fm_inc] if fm_inc else []
    output_files.extend(failuremodes_inc)

    # -------------------------------------------------------------------------
    # 2. Render control measures: TRLC -> .inc via trlc_rst
    # -------------------------------------------------------------------------
    cm_inc = _render_trlc_inc(ctx, ctx.files.controlmeasures, spec_files, "controlmeasures.inc")
    controlmeasures_inc = [cm_inc] if cm_inc else []
    output_files.extend(controlmeasures_inc)

    # -------------------------------------------------------------------------
    # 3. Run lobster-trlc on TRLC sources -> lobster files.
    #    Spec files must be sandbox inputs so the TRLC parser can resolve
    #    ``import ScoreReq`` etc.
    # -------------------------------------------------------------------------
    failuremodes_lobster_files = []
    if ctx.files.failuremodes:
        failuremodes_lobster = ctx.actions.declare_file(
            "{}/failuremodes.lobster".format(ctx.label.name),
        )
        args = ctx.actions.args()
        args.add("--config", ctx.file._fm_lobster_config.path)
        args.add("--out", failuremodes_lobster.path)
        ctx.actions.run(
            inputs = ctx.files.failuremodes + spec_files + [ctx.file._fm_lobster_config],
            outputs = [failuremodes_lobster],
            executable = ctx.executable._lobster_trlc,
            arguments = [args],
            progress_message = "lobster-trlc {}".format(failuremodes_lobster.path),
        )
        failuremodes_lobster_files.append(failuremodes_lobster)

    controlmeasures_lobster_files = []
    if ctx.files.controlmeasures:
        controlmeasures_lobster = ctx.actions.declare_file(
            "{}/controlmeasures.lobster".format(ctx.label.name),
        )
        args = ctx.actions.args()
        args.add("--config", ctx.file._cm_lobster_config.path)
        args.add("--out", controlmeasures_lobster.path)
        ctx.actions.run(
            inputs = ctx.files.controlmeasures + spec_files + [ctx.file._cm_lobster_config],
            outputs = [controlmeasures_lobster],
            executable = ctx.executable._lobster_trlc,
            arguments = [args],
            progress_message = "lobster-trlc {}".format(controlmeasures_lobster.path),
        )
        controlmeasures_lobster_files.append(controlmeasures_lobster)

    # -------------------------------------------------------------------------
    # 4. Generate fmea.rst via expand_template
    # -------------------------------------------------------------------------
    fmea_rst = ctx.actions.declare_file(
        "{}/fmea.rst".format(ctx.label.name),
    )

    title = ctx.label.name

    failure_modes_rst_includes = "\n\n".join(
        [".. include:: " + f.basename for f in failuremodes_inc],
    )
    control_measures_rst_includes = "\n\n".join(
        [".. include:: " + f.basename for f in controlmeasures_inc],
    )

    failure_modes_section = ""
    if failuremodes_inc:
        failure_modes_section = "Failure Modes\n-------------\n\n" + failure_modes_rst_includes

    control_measures_section = ""
    if controlmeasures_inc:
        control_measures_section = "Control Measures\n----------------\n\n" + control_measures_rst_includes

    ctx.actions.expand_template(
        template = ctx.file._template,
        output = fmea_rst,
        substitutions = {
            "{title}": title,
            "{underline}": "=" * len(title),
            "{failure_modes_section}": failure_modes_section,
            "{control_measures_section}": control_measures_section,
            "{root_causes_section}": root_causes_rst_section,
        },
    )
    output_files.append(fmea_rst)

    # -------------------------------------------------------------------------
    # 5. Build providers
    # -------------------------------------------------------------------------
    lobster_files = {}
    for f in failuremodes_lobster_files:
        lobster_files["failuremodes.lobster"] = f
    for f in controlmeasures_lobster_files:
        lobster_files["controlmeasures.lobster"] = f
    for f in root_cause_lobster_files:
        lobster_files["root_causes.lobster"] = f

    # detail_rsts are ancillary: they must be present next to fmea.rst for the
    # sub-toctree to resolve, but they are NOT top-level toctree entries.
    toctree_files = [f for f in output_files if f not in detail_rsts]
    all_sphinx_srcs = depset(toctree_files)

    sphinx_deps = [all_sphinx_srcs]
    if ctx.attr.arch_design and SphinxSourcesInfo in ctx.attr.arch_design:
        sphinx_deps.append(ctx.attr.arch_design[SphinxSourcesInfo].deps)

    return [
        DefaultInfo(
            files = depset(output_files),
        ),
        AnalysisInfo(
            name = ctx.label.name,
            lobster_files = lobster_files,
        ),
        SphinxSourcesInfo(
            srcs = all_sphinx_srcs,
            deps = depset(transitive = sphinx_deps),
            ancillary = depset(detail_rsts),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_fmea = rule(
    implementation = _fmea_impl,
    doc = "Renders FMEA TRLC sources to .inc files and generates lobster traceability files. " +
          "Build-only rule; traceability testing is owned by dependability_analysis.",
    attrs = dict(
        {
            "failuremodes": attr.label_list(
                allow_files = [".trlc"],
                mandatory = False,
                doc = "Failure mode ``.trlc`` source files.",
            ),
            "controlmeasures": attr.label_list(
                allow_files = [".trlc"],
                mandatory = False,
                doc = "Control measure ``.trlc`` source files.",
            ),
            "spec": attr.label_list(
                allow_files = [".rsl", ".trlc"],
                default = [Label("//bazel/rules/rules_score/trlc/config:score_requirements_model")],
                doc = "TRLC model specification files (``.rsl``) required for import resolution. " +
                      "Defaults to the S-CORE requirements model.",
            ),
            "root_causes": attr.label_list(
                allow_files = [".puml", ".plantuml"],
                mandatory = False,
                doc = "Root cause FTA PlantUML diagram files.  " +
                      "``fta_metamodel.puml`` is inlined automatically; " +
                      "lobster items are extracted to ``root_causes.lobster``.",
            ),
            "arch_design": attr.label(
                providers = [ArchitecturalDesignInfo],
                mandatory = False,
                doc = "Reference to architectural_design target for traceability.",
            ),
            "_safety_analysis_tools": attr.label(
                default = Label("//bazel/rules/rules_score:safety_analysis_tools"),
                executable = True,
                allow_files = True,
                cfg = "exec",
                doc = "safety_analysis_tools binary: preprocess and extract subcommands.",
            ),
            "_fta_metamodel": attr.label(
                default = Label("//plantuml:fta_metamodel"),
                allow_single_file = True,
                doc = "fta_metamodel.puml whose content is inlined into root cause diagrams.",
            ),
            "_renderer": attr.label(
                default = Label("@trlc//tools/trlc_rst:trlc_rst"),
                executable = True,
                allow_files = True,
                cfg = "exec",
            ),
            "_lobster_trlc": attr.label(
                default = Label("@lobster//:lobster-trlc"),
                executable = True,
                allow_files = True,
                cfg = "exec",
                doc = "lobster-trlc executable used to generate FM and CM lobster files.",
            ),
            "_fm_lobster_config": attr.label(
                default = Label("//bazel/rules/rules_score/lobster/config:failuremodes_config"),
                allow_single_file = True,
                doc = "lobster-trlc YAML config for FailureMode records.",
            ),
            "_cm_lobster_config": attr.label(
                default = Label("//bazel/rules/rules_score/lobster/config:controlmeasures_config"),
                allow_single_file = True,
                doc = "lobster-trlc YAML config for ControlMeasure records.",
            ),
            "_template": attr.label(
                default = Label("//bazel/rules/rules_score:templates/fmea.template.rst"),
                allow_single_file = True,
                doc = "RST template for the FMEA page.",
            ),
            "_puml_rst_template": attr.label(
                default = Label("//bazel/rules/rules_score:templates/puml_diagram.template.rst"),
                allow_single_file = True,
                doc = "RST template for PlantUML diagram wrapper pages.",
            ),
        },
        **VERBOSITY_ATTR
    ),
)

# ============================================================================
# Public Macro
# ============================================================================

def fmea(
        name,
        spec = None,
        failuremodes = [],
        controlmeasures = [],
        root_causes = [],
        arch_design = None,
        **kwargs):
    """Define FMEA (Failure Mode and Effects Analysis) following S-CORE process guidelines.

    Generates a single ``fmea.rst`` page with up to three sections:
    Failure Modes (TRLC), Control Measures (TRLC), and optionally a
    Root Causes section with FTA PlantUML diagrams.

    FTA diagrams passed via ``root_causes`` are preprocessed to inline
    ``fta_metamodel.puml`` (hermetic, no ``!include`` at render time) and
    lobster traceability items are extracted to ``root_causes.lobster``.

    This is a **build-only** rule.  The combined traceability test
    (FM + CM + FTA) is owned by the ``dependability_analysis`` that wraps
    this target.

    Args:
        name: Target name.
        spec: TRLC model specification files (``.rsl``) for resolving imports.
            Defaults to the S-CORE requirements model. Override only when using
            a custom TRLC schema.
        failuremodes: Failure mode ``.trlc`` source files.
        controlmeasures: Control measure ``.trlc`` source files.
        root_causes: Optional FTA PlantUML diagram files (``.puml`` /
            ``.plantuml``) representing the root causes of failure modes.
        arch_design: Optional ``architectural_design`` target for traceability.
        visibility: Bazel visibility.
        tags: Additional Bazel tags.
    """
    _fmea(
        name = name,
        spec = spec,
        failuremodes = failuremodes,
        controlmeasures = controlmeasures,
        root_causes = root_causes,
        arch_design = arch_design,
        **kwargs
    )
