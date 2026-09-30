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
Safety analysis (FMEA – Failure Mode and Effects Analysis) build rules for S-CORE projects.

The rule generates a single, failure-mode-centric ``safety_analysis.rst`` page: an
overview summary table followed by one section per failure mode.  Each section
carries the full failure-mode safety attributes and one "Root Cause Analysis"
block per fault tree (``FtaFailureMode``) that covers it: the diagram inline
(``.. uml::``) and a "Safety Measures" subsection holding only the measures
(``Mitigation``, ``AoU``, ``CompReq``) that address that tree's root causes.
Failure modes not covered by any fault tree, and measures not referenced by
any generated stub, are still rendered (with an empty root-cause /
traceability section) so nothing is dropped.

Pipeline:

  1. **FTA** (``puml_cli`` in ``--fta-output-dir`` mode) – parses the
     ``$FailureMode``/``$RootCause``/gate macro calls straight from each
     ``root_causes`` diagram and emits ``fta_events.trlc`` — the generated
     ``ScoreReq.FtaFailureMode`` / ``ScoreReq.RootCause`` stub records (see
     ``puml_fta::render_trlc_stub``) that carry the fault-tree topology as
     regular, strongly-typed TRLC references instead of alias-matching. Each
     source diagram is staged (symlinked) alongside ``safety_analysis.rst``, unmodified,
     so ``.. uml:: <basename>`` resolves in the Sphinx tree.
  2. **Assembly** (``safety_analysis_assembler``) – a single in-process TRLC parse (via
     the extended ``TRLCRST`` library) over ``fta_events.trlc`` plus the
     FailureMode / Mitigation records renders the overview table and every
     chain section into ``safety_analysis.rst``.
  3. **Lobster** (``lobster-trlc``) – FailureMode, Mitigation, FtaFailureMode and
     RootCause traceability files, driven by real TRLC references
     (``failure_modes``, ``root_causes``) rather than
     alias-name matching.

The metamodel-inlined ``.puml`` diagrams travel as ``aux_srcs`` so Sphinx can
resolve ``.. uml::`` without adding them to the toctree.

``AnalysisInfo`` carries all lobster traceability files (failuremodes,
safetymeasures, fta_failure_modes, fta_root_causes) as a ``lobster_files`` dict
keyed by canonical filename.  ``SafetyAnalysisProviderInfo`` carries the raw TRLC
source files (failuremodes, safetymeasures, fta_events, spec) so
``dependability_analysis`` can combine them across every safety_analysis sub-target for
its root-cause-coverage completeness check.  All Sphinx source files travel
via ``SphinxSourcesInfo``.

This is a **build-only** rule.  The combined traceability *test* is owned by the
``dependability_analysis`` rule which wraps this one.
"""

load("//bazel/rules/rules_score:providers.bzl", "AnalysisInfo", "ArchitecturalDesignInfo", "SafetyAnalysisProviderInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

def _default_fta_package(name):
    """Best-effort conversion of a Bazel target *name* into a valid TRLC
    package identifier (``[A-Za-z_][A-Za-z0-9_]*``): non-identifier
    characters (e.g. ``-``) become ``_``; a name starting with a digit gets a
    leading ``_``.

    A ``_fta`` suffix is appended: TRLC rejects packages whose name is "too
    similar" (same modulo case/underscores) to another declared package, and
    FailureMode/SafetyMeasure packages conventionally use a PascalCase form
    of the very same target name (e.g. target ``my_safety_analysis`` + package
    ``MySafetyAnalysis``) -- using the bare sanitized name here would then always
    collide with that package.
    """
    out = ""
    for i in range(len(name)):
        c = name[i]
        out += c if (c.isalnum() or c == "_") else "_"
    if out[:1].isdigit():
        out = "_" + out
    return out + "_fta"

# ============================================================================
# Root-cause (FTA) processing helper
# ============================================================================

def _process_root_causes(ctx, fta_package):
    """Extract the generated TRLC stub, and stage the diagrams for rendering.

    ``puml_cli`` (FTA mode) parses the ``$FailureMode``/``$RootCause``/gate macro
    calls straight from each diagram and emits, into ``{label}/``:

      * ``fta_events.trlc`` (the generated ``FtaFailureMode``/``RootCause`` stub
        records, package *fta_package*; see ``puml_fta::render_trlc_stub``).

    The diagrams are *not* rewritten: each source ``.puml`` is symlinked next to
    ``safety_analysis.rst`` so ``.. uml:: <basename>`` resolves to the authored diagram.
    Its ``!include fta_metamodel.puml`` is resolved at render time via the docs
    toolchain's global PlantUML include path (the metamodel is shipped with
    the registered ``sphinx_toolchain``), so the metamodel is not staged here.

    Args:
        ctx: Rule context.
        fta_package: TRLC package name for the generated ``fta_events.trlc``.

    Returns:
        Tuple ``(diagram_aux_files, fta_events_trlc)``.  ``diagram_aux_files``
        (the staged ``.puml`` diagrams) is empty when there are no PlantUML
        inputs; ``fta_events_trlc`` is always a File (a stub package with zero
        records when there are no diagrams).
    """
    puml_inputs = [
        f
        for f in ctx.files.root_causes
        if f.extension in ("puml", "plantuml")
    ]

    fta_events_trlc = ctx.actions.declare_file("{}/fta_events.trlc".format(ctx.label.name))

    if not puml_inputs:
        # No fault trees: emit an empty stub artifact so the assembler still runs
        # (rendering every failure mode without a root-cause analysis). No
        # "import ScoreReq" here: with zero records it would be flagged as an
        # unused import by TRLC.
        ctx.actions.write(
            fta_events_trlc,
            "// no root cause diagrams\npackage {}\n".format(fta_package),
        )
        return [], fta_events_trlc

    # Symlink each authored diagram next to safety_analysis.rst so ``.. uml:: <basename>``
    # resolves in the Sphinx tree.
    diagram_aux_files = []
    for src in puml_inputs:
        staged = ctx.actions.declare_file("{}/{}".format(ctx.label.name, src.basename))
        ctx.actions.symlink(output = staged, target_file = src)
        diagram_aux_files.append(staged)

    args = ctx.actions.args()
    for src in puml_inputs:
        args.add("--file", src.path)
    args.add("--fta-output-dir", fta_events_trlc.dirname)
    args.add("--fta-package", fta_package)
    args.add("--log-level", get_log_level(ctx))
    ctx.actions.run(
        inputs = puml_inputs,
        outputs = [fta_events_trlc],
        executable = ctx.executable._puml_cli,
        arguments = [args],
        progress_message = "Processing root cause FTA diagrams for %s" % ctx.label.name,
    )

    return diagram_aux_files, fta_events_trlc

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

def _safety_analysis_impl(ctx):
    output_files = []

    fta_package = ctx.attr.fta_package if ctx.attr.fta_package else _default_fta_package(ctx.label.name)

    # 0. FTA: extract the generated TRLC stub, and stage diagrams for rendering.
    diagram_aux_files, fta_events_trlc = _process_root_causes(ctx, fta_package)
    output_files.extend(diagram_aux_files)
    has_root_causes = bool(diagram_aux_files)

    # 1. Assemble safety_analysis.rst from the generated FTA stub + TRLC records (single in-process parse).
    safety_analysis_rst = ctx.actions.declare_file("{}/safety_analysis.rst".format(ctx.label.name))
    title = ctx.label.name

    args = ctx.actions.args()
    args.add("--output", safety_analysis_rst.path)
    args.add("--template", ctx.file._template.path)
    args.add("--title", title)
    args.add("--fta-events", fta_events_trlc.path)
    args.add("--log-level", get_log_level(ctx))
    if ctx.files.failuremodes:
        args.add("--failuremodes")
        args.add_all(ctx.files.failuremodes)
    if ctx.files.safetymeasures:
        args.add("--safetymeasures")
        args.add_all(ctx.files.safetymeasures)
    if ctx.files.spec:
        args.add("--spec")
        args.add_all(ctx.files.spec)
    ctx.actions.run(
        inputs = (
            ctx.files.failuremodes +
            ctx.files.safetymeasures +
            ctx.files.spec +
            [fta_events_trlc, ctx.file._template]
        ),
        outputs = [safety_analysis_rst],
        executable = ctx.executable._safety_analysis_assembler,
        arguments = [args],
        progress_message = "Assembling safety-analysis page for %s" % ctx.label.name,
    )
    output_files.append(safety_analysis_rst)

    # 2. lobster-trlc traceability for FailureMode / Mitigation / FtaFailureMode / RootCause records.
    #
    # fta_events.trlc always ``import``s every FailureMode package referenced by
    # its FtaFailureMode records (regardless of which record a given invocation is
    # actually converting), so ctx.files.failuremodes must travel alongside it
    # in every lobster-trlc input set, or TRLC parsing fails on the unresolved
    # import.
    fta_and_fm_files = [fta_events_trlc] + ctx.files.failuremodes

    fm_lobster = _lobster_trlc(ctx, ctx.files.failuremodes, ctx.file._fm_lobster_config, "failuremodes.lobster")
    safetymeasures_trlc_files = ctx.files.safetymeasures + (fta_and_fm_files if ctx.files.safetymeasures else [])
    safetymeasures_lobster = _lobster_trlc(ctx, safetymeasures_trlc_files, ctx.file._safetymeasures_lobster_config, "safetymeasures.lobster")

    fta_fm_lobster = None
    rc_lobster = None
    if has_root_causes:
        fta_fm_lobster = _lobster_trlc(
            ctx,
            fta_and_fm_files,
            ctx.file._fta_fm_lobster_config,
            "fta_failure_modes.lobster",
        )
        rc_lobster = _lobster_trlc(
            ctx,
            fta_and_fm_files,
            ctx.file._rc_lobster_config,
            "fta_root_causes.lobster",
        )

    # 3. Providers.
    lobster_files = {}
    if fm_lobster:
        lobster_files["failuremodes.lobster"] = fm_lobster
    if safetymeasures_lobster:
        lobster_files["safetymeasures.lobster"] = safetymeasures_lobster
    if fta_fm_lobster:
        lobster_files["fta_failure_modes.lobster"] = fta_fm_lobster
    if rc_lobster:
        lobster_files["fta_root_causes.lobster"] = rc_lobster

    # The preprocessed .puml diagrams are referenced inline via ``.. uml::`` but
    # must not be toctree documents, so they travel as aux_srcs (symlinked
    # alongside safety_analysis.rst by dependable_element without being indexed).
    sphinx_srcs = depset([safety_analysis_rst])

    return [
        DefaultInfo(
            files = depset(output_files + [v for v in lobster_files.values()]),
        ),
        AnalysisInfo(
            name = ctx.label.name,
            lobster_files = lobster_files,
        ),
        SafetyAnalysisProviderInfo(
            failuremodes = depset(ctx.files.failuremodes),
            safetymeasures = depset(ctx.files.safetymeasures),
            fta_events = depset([fta_events_trlc]),
            spec = depset(ctx.files.spec),
        ),
        SphinxSourcesInfo(
            srcs = sphinx_srcs,
            deps = depset(transitive = [sphinx_srcs]),
            aux_srcs = depset(diagram_aux_files),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_safety_analysis = rule(
    implementation = _safety_analysis_impl,
    doc = "Renders a failure-mode-centric safety-analysis page (overview table + one chain " +
          "section per failure mode) and lobster traceability files. " +
          "Build-only rule; traceability testing is owned by dependability_analysis.",
    attrs = dict(
        {
            "failuremodes": attr.label_list(
                allow_files = [".trlc"],
                mandatory = False,
                doc = "Failure mode ``.trlc`` source files.",
            ),
            "safetymeasures": attr.label_list(
                allow_files = [".trlc"],
                mandatory = False,
                doc = "Safety measure (``Mitigation``/``AoU``/``CompReq``) ``.trlc`` source files.",
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
                      "lobster items and the ``fta_events.trlc`` TRLC stub are generated from them.",
            ),
            "fta_package": attr.string(
                mandatory = False,
                default = "",
                doc = "TRLC package name for the generated ``fta_events.trlc`` stub file " +
                      "(the ``FtaFailureMode``/``RootCause`` records generated from ``root_causes``). " +
                      "Defaults to a sanitized form of the target name.",
            ),
            "arch_design": attr.label(
                providers = [ArchitecturalDesignInfo],
                mandatory = True,
                doc = "architectural_design target for traceability.",
            ),
            "_puml_cli": attr.label(
                default = Label("//plantuml/parser/puml_cli:puml_cli"),
                executable = True,
                allow_files = True,
                cfg = "exec",
                doc = "puml_cli binary used in FTA mode to extract fta_events.trlc.",
            ),
            "_safety_analysis_assembler": attr.label(
                default = Label("//bazel/rules/rules_score:safety_analysis_assembler"),
                executable = True,
                allow_files = True,
                cfg = "exec",
                doc = "Safety-analysis page assembler (imports the extended TRLCRST library).",
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
            "_safetymeasures_lobster_config": attr.label(
                default = Label("//bazel/rules/rules_score/lobster/config:safetymeasures_config"),
                allow_single_file = True,
                doc = "lobster-trlc YAML config for Mitigation records.",
            ),
            "_fta_fm_lobster_config": attr.label(
                default = Label("//bazel/rules/rules_score/lobster/config:fta_failure_modes_config"),
                allow_single_file = True,
                doc = "lobster-trlc YAML config for generated FtaFailureMode records.",
            ),
            "_rc_lobster_config": attr.label(
                default = Label("//bazel/rules/rules_score/lobster/config:fta_root_causes_config"),
                allow_single_file = True,
                doc = "lobster-trlc YAML config for generated RootCause records.",
            ),
            "_template": attr.label(
                default = Label("//bazel/rules/rules_score:templates/safety_analysis.template.rst"),
                allow_single_file = True,
                doc = "RST template for the safety-analysis page (single ``{body}`` placeholder).",
            ),
        },
        **VERBOSITY_ATTR
    ),
)

# ============================================================================
# Public Macro
# ============================================================================

def safety_analysis(
        name,
        arch_design,
        spec = None,
        failuremodes = [],
        safetymeasures = [],
        root_causes = [],
        fta_package = None,
        **kwargs):
    """Define a safety analysis (FMEA - Failure Mode and Effects Analysis) following S-CORE process guidelines.

    Generates a single, failure-mode-centric ``safety_analysis.rst`` page: an overview
    summary table followed by one section per failure mode (failure-mode detail,
    one inline fault tree per covering ``FtaFailureMode``, and that tree's measures).

    FTA diagrams passed via ``root_causes`` are preprocessed to extract
    fault-tree topology, emitted as strongly-typed TRLC ``FtaFailureMode``/
    ``RootCause`` records (``fta_events.trlc``, package ``fta_package``) that
    ``Mitigation``, ``AoU``, and ``CompReq`` records can reference (via
    ``root_causes``/``derived_from``) as a measure addressing that root cause.

    This is a **build-only** rule.  The combined traceability test
    (FM + measures + FTA, including root-cause-coverage completeness) is
    owned by the ``dependability_analysis`` that wraps this target.

    Args:
        name: Target name.
        arch_design: ``architectural_design`` target for traceability; its
            ``public_api`` is what ``FailureMode.interface`` references.
        spec: TRLC model specification files (``.rsl``) for resolving imports.
            Defaults to the S-CORE requirements model. Override only when using
            a custom TRLC schema.
        failuremodes: Failure mode ``.trlc`` source files.
        safetymeasures: Safety measure (``Mitigation``/``AoU``/``CompReq``) ``.trlc`` source files.
        root_causes: Optional FTA PlantUML diagram files (``.puml`` /
            ``.plantuml``) representing the root causes of failure modes.
        fta_package: TRLC package name for the generated ``fta_events.trlc``
            stub file. Defaults to a sanitized form of ``name``.
        **kwargs: Additional arguments (e.g. ``visibility``, ``tags``).
    """
    _safety_analysis(
        name = name,
        spec = spec,
        failuremodes = failuremodes,
        safetymeasures = safetymeasures,
        root_causes = root_causes,
        fta_package = fta_package or "",
        arch_design = arch_design,
        **kwargs
    )
