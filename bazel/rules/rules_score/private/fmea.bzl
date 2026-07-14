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

The rule generates a single, failure-mode-centric ``fmea.rst`` page: an
overview summary table followed by one section per failure mode.  Each section
carries the full failure-mode safety attributes, the fault-tree diagram inline
(``.. uml::``), and a "Control Measures" subsection holding only that chain's
basic events.  Failure modes and control measures not referenced by any fault
tree are appended under trailing "Unlinked …" sections.

Pipeline:

  1. **FTA** (``puml_cli`` in ``--fta-output-dir`` mode) – inlines
     ``fta_metamodel.puml`` into each ``root_causes`` diagram and emits the
     metamodel-inlined ``.puml`` (for ``.. uml::``), ``root_causes.lobster``
     (``lobster-act-trace``) and ``fta_chains.json`` (the ordered per-failure
     mode chains).
  2. **Assembly** (``fmea_assembler``) – a single in-process TRLC parse via the
     extended ``TRLCRST`` library renders the overview table and every chain
     section into ``fmea.rst``.
  3. **Lobster** (``lobster-trlc``) – FailureMode and ControlMeasure
     traceability files, unchanged.

The metamodel-inlined ``.puml`` diagrams travel as ``aux_srcs`` so Sphinx can
resolve ``.. uml::`` without adding them to the toctree.

``AnalysisInfo`` carries all lobster traceability files (failuremodes,
controlmeasures, and root_causes if present) as a ``lobster_files`` dict keyed
by canonical filename.  All Sphinx source files travel via
``SphinxSourcesInfo``.

This is a **build-only** rule.  The combined traceability *test* is owned by the
``dependability_analysis`` rule which wraps this one.
"""

load("//bazel/rules/rules_score:providers.bzl", "AnalysisInfo", "ArchitecturalDesignInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

# ============================================================================
# Root-cause (FTA) processing helper
# ============================================================================

def _process_root_causes(ctx):
    """Extract FTA traceability + chains and stage the diagrams for rendering.

    ``puml_cli`` (FTA mode) parses the ``$TopEvent``/``$BasicEvent``/gate macro
    calls straight from each diagram and emits, into ``{label}/``:

      * ``root_causes.lobster`` (``lobster-act-trace`` traceability), and
      * ``fta_chains.json`` (the ordered per-failure-mode chains).

    The diagrams are *not* rewritten: each source ``.puml`` is symlinked next to
    ``fmea.rst`` so ``.. uml:: <basename>`` resolves to the authored diagram.
    Its ``!include fta_metamodel.puml`` is resolved at render time via the docs
    toolchain's global PlantUML include path (the metamodel is shipped with
    ``//tools/sphinx:sphinx-build``), so the metamodel is not staged here.

    Returns:
        Tuple ``(diagram_aux_files, root_causes_idmaps, root_causes_lobster_or_None, chains_json)``.
        ``diagram_aux_files`` (the staged ``.puml`` diagrams) is empty when there
        are no PlantUML inputs; ``chains_json`` is always a File (an empty ``[]``
        array when there are no diagrams).
    """
    puml_inputs = [
        f
        for f in ctx.files.root_causes
        if f.extension in ("puml", "plantuml")
    ]

    chains_json = ctx.actions.declare_file("{}/fta_chains.json".format(ctx.label.name))

    if not puml_inputs:
        # No fault trees: emit an empty chains file so the assembler still runs
        # (rendering every failure mode without an FTA).
        ctx.actions.write(chains_json, "[]\n")
        return [], [], None, chains_json

    # Symlink each authored diagram next to fmea.rst so .. uml:: <basename>
    # resolves in the Sphinx tree. Basenames must be unique because staging is flat.
    diagram_aux_files = []
    seen_basenames = {}
    duplicate_basenames = []
    for src in puml_inputs:
        if src.basename in seen_basenames:
            duplicate_basenames.append(src.basename)
        else:
            seen_basenames[src.basename] = True
            staged = ctx.actions.declare_file("{}/{}".format(ctx.label.name, src.basename))
            ctx.actions.symlink(output = staged, target_file = src)
            diagram_aux_files.append(staged)

    if duplicate_basenames:
        fail(
            "root_causes contains duplicate basenames not supported by fmea staging: {}. "
                .format(", ".join(duplicate_basenames)) +
            "Rename diagrams to unique basenames.",
        )

    root_causes_lobster = ctx.actions.declare_file("{}/root_causes.lobster".format(ctx.label.name))
    root_cause_idmaps = [
        ctx.actions.declare_file(
            "{}/{}.idmap.json".format(ctx.label.name, src.basename.rsplit(".", 1)[0]),
        )
        for src in puml_inputs
    ]

    args = ctx.actions.args()
    for src in puml_inputs:
        args.add("--file", src.path)
    args.add("--fta-output-dir", root_causes_lobster.dirname)
    args.add("--idmap-output-dir", root_causes_lobster.dirname)
    args.add("--log-level", get_log_level(ctx))
    ctx.actions.run(
        inputs = puml_inputs,
        outputs = [root_causes_lobster, chains_json] + root_cause_idmaps,
        executable = ctx.executable._puml_cli,
        arguments = [args],
        progress_message = "Processing root cause FTA diagrams for %s" % ctx.label.name,
    )

    return diagram_aux_files, root_cause_idmaps, root_causes_lobster, chains_json

# ============================================================================
# Lobster (TRLC traceability) helper
# ============================================================================

def _lobster_trlc(ctx, trlc_files, config, out_name):
    """Run ``lobster-trlc`` over *trlc_files* producing ``{label}/<out_name>``."""
    if not trlc_files:
        return None
    out = ctx.actions.declare_file("{}/{}".format(ctx.label.name, out_name))
    args = ctx.actions.args()
    args.add("--config", config.path)
    args.add("--out", out.path)
    ctx.actions.run(
        inputs = trlc_files + ctx.files.spec + [config],
        outputs = [out],
        executable = ctx.executable._lobster_trlc,
        arguments = [args],
        progress_message = "lobster-trlc {}".format(out.path),
    )
    return out

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _fmea_impl(ctx):
    output_files = []

    # 0. FTA: extract chains/lobster + stage diagrams (and metamodel) for rendering.
    diagram_aux_files, root_cause_idmaps, root_causes_lobster, chains_json = _process_root_causes(ctx)
    output_files.extend(diagram_aux_files)
    output_files.extend(root_cause_idmaps)

    # 1. Assemble fmea.rst from the chains + TRLC records (single in-process parse).
    fmea_rst = ctx.actions.declare_file("{}/fmea.rst".format(ctx.label.name))
    title = ctx.label.name

    args = ctx.actions.args()
    args.add("--output", fmea_rst.path)
    args.add("--template", ctx.file._template.path)
    args.add("--title", title)
    args.add("--chains", chains_json.path)
    args.add("--log-level", get_log_level(ctx))
    if ctx.files.failuremodes:
        args.add("--failuremodes")
        args.add_all(ctx.files.failuremodes)
    if ctx.files.controlmeasures:
        args.add("--controlmeasures")
        args.add_all(ctx.files.controlmeasures)
    if ctx.files.spec:
        args.add("--spec")
        args.add_all(ctx.files.spec)
    ctx.actions.run(
        inputs = (
            ctx.files.failuremodes +
            ctx.files.controlmeasures +
            ctx.files.spec +
            [chains_json, ctx.file._template]
        ),
        outputs = [fmea_rst],
        executable = ctx.executable._fmea_assembler,
        arguments = [args],
        progress_message = "Assembling FMEA page for %s" % ctx.label.name,
    )
    output_files.append(fmea_rst)

    # 2. lobster-trlc traceability for FailureMode / ControlMeasure records.
    fm_lobster = _lobster_trlc(ctx, ctx.files.failuremodes, ctx.file._fm_lobster_config, "failuremodes.lobster")
    cm_lobster = _lobster_trlc(ctx, ctx.files.controlmeasures, ctx.file._cm_lobster_config, "controlmeasures.lobster")

    # 3. Providers.
    lobster_files = {}
    if fm_lobster:
        lobster_files["failuremodes.lobster"] = fm_lobster
    if cm_lobster:
        lobster_files["controlmeasures.lobster"] = cm_lobster
    if root_causes_lobster:
        lobster_files["root_causes.lobster"] = root_causes_lobster

    # The preprocessed .puml diagrams are referenced inline via ``.. uml::`` but
    # must not be toctree documents, so they travel as aux_srcs (symlinked
    # alongside fmea.rst by dependable_element without being indexed).
    sphinx_srcs = depset([fmea_rst])

    return [
        DefaultInfo(
            files = depset(output_files + [v for v in lobster_files.values()]),
        ),
        AnalysisInfo(
            name = ctx.label.name,
            lobster_files = lobster_files,
        ),
        SphinxSourcesInfo(
            srcs = sphinx_srcs,
            deps = depset(transitive = [sphinx_srcs]),
            aux_srcs = depset(diagram_aux_files + root_cause_idmaps),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_fmea = rule(
    implementation = _fmea_impl,
    doc = "Renders a failure-mode-centric FMEA page (overview table + one chain " +
          "section per failure mode) and lobster traceability files. " +
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
            "_puml_cli": attr.label(
                default = Label("//plantuml/parser/puml_cli:puml_cli"),
                executable = True,
                allow_files = True,
                cfg = "exec",
                doc = "puml_cli binary used in FTA mode to inline the metamodel and " +
                      "extract root_causes.lobster + fta_chains.json.",
            ),
            "_fmea_assembler": attr.label(
                default = Label("//bazel/rules/rules_score:fmea_assembler"),
                executable = True,
                allow_files = True,
                cfg = "exec",
                doc = "FMEA page assembler (imports the extended TRLCRST library).",
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
                doc = "RST template for the FMEA page (single ``{body}`` placeholder).",
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

    Generates a single, failure-mode-centric ``fmea.rst`` page: an overview
    summary table followed by one section per failure mode (failure-mode detail,
    the fault tree inline, and that chain's control measures).

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
        **kwargs: Additional arguments (e.g. ``visibility``, ``tags``).
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
