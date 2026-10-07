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

The analysis is split into stages so that the measures addressing a root
cause (AoUs, component requirements) can reference the root causes without a
Bazel dependency cycle:

  1. ``failure_modes`` – the ``FailureMode`` records and their lobster file
     (see ``failure_modes.bzl``).
  2. ``fault_trees`` – the root causes.  Takes only ``.puml`` diagrams; the
     ``failure_modes`` targets they reference are listed in ``deps``.
     ``puml_cli`` (``--fta-output-dir`` mode) parses the
     ``$FailureMode``/``$RootCause``/gate macro calls of each diagram and emits
     ``fta_events.trlc`` — the generated ``ScoreReq.RootCause`` records, each
     linking to the ``FailureMode`` records of its tree (see
     ``puml_fta::render_trlc_stub``).  The root-cause lobster file is produced
     here.  Requirement targets (``assumptions_of_use``,
     ``component_requirements``) list it in ``deps`` to resolve
     ``<fta_package>.<RootCause>``.
  3. ``safety_analysis`` – the page.  Takes the failure modes, the fault trees
     and the ``safety_measures``: the measure targets (``assumptions_of_use`` /
     ``component_requirements``) and ``Mitigation`` ``.trlc`` files that address
     the root causes.

``safety_analysis`` generates a single, failure-mode-centric
``safety_analysis.rst`` page: an overview summary table followed by one section
per failure mode.  Each section carries the full failure-mode safety attributes
and one card per root cause (``RootCause``) listing the measures
(``Mitigation``, ``AoU``, ``CompReq``) that address it.  A "Fault Trees" section
shows every fault-tree diagram once (``.. uml::``).
Failure modes not covered by any fault tree, and measures not referencing any
generated root cause, are still rendered so nothing is dropped.

Pipeline of ``safety_analysis``:

  1. **Assembly** (``safety_analysis_assembler``) – a single in-process TRLC parse (via
     the extended ``TRLCRST`` library) over ``fta_events.trlc`` plus the
     FailureMode / Mitigation / AoU records renders the overview table, the
     root-cause cards and the fault-tree diagrams into ``safety_analysis.rst``.
     Each source diagram is staged (symlinked) alongside ``safety_analysis.rst``,
     unmodified, so ``.. uml:: <basename>`` resolves in the Sphinx tree.
  2. **Lobster** (``lobster-trlc``) – safety-measure (Mitigation / AoU)
     traceability, driven by real TRLC references (``root_causes``) rather than
     alias-name matching.

The ``.puml`` diagrams travel as ``aux_srcs`` so Sphinx can
resolve ``.. uml::`` without adding them to the toctree.

``AnalysisInfo`` carries all lobster traceability files (failuremodes,
safetymeasures, fta_root_causes) as a ``lobster_files`` dict
keyed by canonical filename.  ``SafetyAnalysisProviderInfo`` carries the AoU
targets consumed as measures; the ``dependable_element`` derives its own AoUs
from them.  All Sphinx source files travel
via ``SphinxSourcesInfo``.

Both rules are **build-only**.  The combined traceability *test* is owned by the
``dependability_analysis`` rule which wraps ``safety_analysis``.
"""

load("@trlc//:trlc.bzl", "TrlcProviderInfo")
load("//bazel/rules/rules_score:providers.bzl", "AnalysisInfo", "ArchitecturalDesignInfo", "AssumptionsOfUseInfo", "ComponentRequirementsInfo", "FailureModesInfo", "FaultTreesInfo", "SafetyAnalysisProviderInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:lobster_config.bzl", "MERGE_LOBSTER_ITEMS_ATTR", "merge_lobster_files")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

def _default_fta_package(label):
    """Best-effort conversion of a Bazel *label* into a valid TRLC package
    identifier (``[A-Za-z_][A-Za-z0-9_]*``).

    Package and name are joined with ``__`` so equally named targets of
    different packages get distinct packages in one TRLC build. The repository
    is left out: it differs between a root build and a build as a dependency,
    which would change the package that downstream records reference.
    Non-identifier characters (e.g. ``-``, ``/``) become ``_``; a result
    starting with a digit gets a leading ``_``.

    A ``_fta`` suffix is appended: TRLC rejects packages whose name is "too
    similar" (same modulo case/underscores) to another declared package, and
    FailureMode/SafetyMeasure packages conventionally use a PascalCase form
    of the very same target name (e.g. target ``my_safety_analysis`` + package
    ``MySafetyAnalysis``) -- using the bare sanitized name here would then always
    collide with that package.
    """
    name = "__".join([p for p in [label.package, label.name] if p])
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
    """Generate the TRLC stub of the fault-tree diagrams.

    ``puml_cli`` (FTA mode) parses the ``$FailureMode``/``$RootCause``/gate macro
    calls straight from each diagram and emits ``fta_events.trlc`` into ``{label}/``
    (the generated ``RootCause`` records, package *fta_package*; see
    ``puml_fta::render_trlc_stub``).

    The diagrams are *not* rewritten; ``safety_analysis`` stages them unmodified.
    Their ``!include fta_metamodel.puml`` is resolved at render time via the docs
    toolchain's global PlantUML include path (the metamodel is shipped with
    the registered ``sphinx_toolchain``).

    Args:
        ctx: Rule context.
        fta_package: TRLC package name for the generated ``fta_events.trlc``.

    Returns:
        Tuple ``(puml_inputs, fta_events_trlc)``.
        ``puml_inputs`` (the authored ``.puml`` diagrams) is empty when
        there are no PlantUML inputs; ``fta_events_trlc`` is always a File
        (an empty stub package when there are no diagrams).
    """
    puml_inputs = ctx.files.srcs

    fta_events_trlc = ctx.actions.declare_file("{}/fta_events.trlc".format(ctx.label.name))

    if not puml_inputs:
        # No fault trees: emit an empty stub so the assembler still runs
        # (rendering every failure mode without root causes). No
        # "import ScoreReq" here: with zero records it would be flagged as an
        # unused import by TRLC.
        ctx.actions.write(
            fta_events_trlc,
            "// no root cause diagrams\npackage {}\n".format(fta_package),
        )
        return [], fta_events_trlc

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

    return puml_inputs, fta_events_trlc

# ============================================================================
# Lobster (TRLC traceability) helper
# ============================================================================

def _lobster_trlc(ctx, trlc_files, config, out_name):
    """Run ``lobster-trlc`` over *trlc_files* (incl. the model spec) producing ``{label}/<out_name>``."""
    if not trlc_files:
        return None
    out = ctx.actions.declare_file("{}/{}".format(ctx.label.name, out_name))
    args = ctx.actions.args()
    args.add("--config", config.path)
    args.add("--out", out.path)
    ctx.actions.run(
        inputs = depset(trlc_files + [config]),
        outputs = [out],
        executable = ctx.executable._lobster_trlc,
        arguments = [args],
        progress_message = "lobster-trlc {}".format(out.path),
    )
    return out

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _unseen_files(depsets, known):
    """Files of *depsets* whose path is not in *known*; adds them to *known*."""
    files = []
    for f in depset(transitive = depsets).to_list():
        if f.path not in known:
            known[f.path] = True
            files.append(f)
    return files

def _fault_trees_impl(ctx):
    fta_package = ctx.attr.fta_package if ctx.attr.fta_package else _default_fta_package(ctx.label)

    diagrams, fta_events_trlc = _process_root_causes(ctx, fta_package)

    # fta_events.trlc ``import``s every FailureMode package its RootCause
    # records reference, so the failure mode files travel with it in every
    # lobster-trlc input set and in the provider's deps.
    fm_infos = [dep[TrlcProviderInfo] for dep in ctx.attr.deps]
    fm_files = depset(transitive = [info.reqs for info in fm_infos] + [info.deps for info in fm_infos])
    spec = depset(ctx.files.spec, transitive = [info.spec for info in fm_infos])

    lobster_files = {}
    if diagrams:
        lobster_files["fta_root_causes.lobster"] = _lobster_trlc(
            ctx,
            [fta_events_trlc] + fm_files.to_list() + spec.to_list(),
            ctx.file._rc_lobster_config,
            "fta_root_causes.lobster",
        )

    return [
        DefaultInfo(files = depset([fta_events_trlc] + lobster_files.values())),
        FaultTreesInfo(
            fta_package = fta_package,
            fta_events = fta_events_trlc,
            diagrams = diagrams,
            lobster_files = lobster_files,
        ),
        # Lets requirement targets (assumptions_of_use, component_requirements)
        # list this target in `deps` to resolve `<fta_package>.<RootCause>`.
        TrlcProviderInfo(
            spec = spec,
            reqs = depset([fta_events_trlc]),
            deps = fm_files,
        ),
    ]

def _split_safety_measures(ctx):
    """Split ``safety_measures`` into measure targets and raw Mitigation files."""
    measure_targets = []
    mitigation_files = []
    for target in ctx.attr.safety_measures:
        if AssumptionsOfUseInfo in target or ComponentRequirementsInfo in target:
            measure_targets.append(target)
        elif TrlcProviderInfo in target:
            fail("{}: safety_measures entry {} must be an assumptions_of_use or component_requirements target, or a Mitigation .trlc file".format(ctx.label, target.label))
        else:
            mitigation_files.extend(target[DefaultInfo].files.to_list())
    return measure_targets, mitigation_files

def _safety_analysis_impl(ctx):
    fault_trees = ctx.attr.fault_trees[FaultTreesInfo]
    fault_trees_trlc = ctx.attr.fault_trees[TrlcProviderInfo]
    fta_events_trlc = fault_trees.fta_events

    fm_infos = [fm[TrlcProviderInfo] for fm in ctx.attr.failure_modes]
    failuremode_files = depset(transitive = [info.reqs for info in fm_infos]).to_list()

    # Failure mode files the root causes import. Parsed with the page, but only
    # the records of ``failure_modes`` are rendered and traced.
    fm_context_files = [
        f
        for f in depset(transitive = [fault_trees_trlc.deps] + [info.deps for info in fm_infos]).to_list()
        if f not in failuremode_files
    ]
    fta_and_fm_files = [fta_events_trlc] + failuremode_files + fm_context_files

    # Stage each authored diagram next to safety_analysis.rst so
    # ``.. uml:: <basename>`` resolves in the Sphinx tree.
    diagram_aux_files = []
    for src in fault_trees.diagrams:
        staged = ctx.actions.declare_file("{}/{}".format(ctx.label.name, src.basename))
        ctx.actions.symlink(output = staged, target_file = src)
        diagram_aux_files.append(staged)

    # The records of the measure targets are rendered and traced. The TRLC they
    # resolve against (their deps) is only parsed, so records held there (e.g. a
    # received AoU a component requirement derives from) are not treated as
    # measures of this analysis. The fault-tree files are passed separately.
    measure_targets, mitigation_files = _split_safety_measures(ctx)
    infos = [measure[TrlcProviderInfo] for measure in measure_targets]
    spec_files = depset(ctx.files.spec, transitive = [fault_trees_trlc.spec] + [info.spec for info in fm_infos + infos]).to_list()
    known = {f.path: True for f in fta_and_fm_files + mitigation_files + spec_files}
    aou_targets = [m for m in measure_targets if AssumptionsOfUseInfo in m]
    measure_files = _unseen_files([info.reqs for info in infos], known)
    context_files = _unseen_files([info.deps for info in infos], known)

    # lobster-trlc registers every input and extracts Mitigation / AoU records
    # only, so it gets the AoU measures plus the context they need to parse; a
    # component requirement measure is traced via its own level.
    aou_paths = {
        f.path: True
        for f in depset(transitive = [m[TrlcProviderInfo].reqs for m in aou_targets] + [m[TrlcProviderInfo].deps for m in aou_targets]).to_list()
    }
    lobster_measure_files = mitigation_files + [f for f in measure_files + context_files if f.path in aou_paths]

    # 1. Assemble safety_analysis.rst from the generated FTA stub + TRLC records (single in-process parse).
    safety_analysis_rst = ctx.actions.declare_file("{}/safety_analysis.rst".format(ctx.label.name))

    args = ctx.actions.args()
    args.add("--output", safety_analysis_rst.path)
    args.add("--template", ctx.file._template.path)
    args.add("--title", ctx.label.name)
    args.add("--fta-package", fault_trees.fta_package)
    args.add("--fta-events", fta_events_trlc.path)
    args.add_all("--diagrams", [f.basename for f in diagram_aux_files])
    args.add("--log-level", get_log_level(ctx))
    if failuremode_files:
        args.add("--failuremodes")
        args.add_all(failuremode_files)
    if mitigation_files:
        args.add("--mitigations")
        args.add_all(mitigation_files)
    if measure_files:
        args.add("--safetymeasures")
        args.add_all(measure_files)
    dep_files = spec_files + fm_context_files + context_files
    if dep_files:
        args.add("--dep-files")
        args.add_all(dep_files)
    ctx.actions.run(
        inputs = depset(
            failuremode_files +
            mitigation_files +
            measure_files +
            dep_files +
            [fta_events_trlc, ctx.file._template],
        ),
        outputs = [safety_analysis_rst],
        executable = ctx.executable._safety_analysis_assembler,
        arguments = [args],
        progress_message = "Assembling safety-analysis page for %s" % ctx.label.name,
    )

    # 2. Lobster traceability: failure modes (merged over the failure_modes
    # targets), root causes (from the fault trees) and Mitigation / AoU records.
    lobster_files = dict(fault_trees.lobster_files)
    fm_lobster_files, _ = merge_lobster_files(
        ctx,
        depset(transitive = [fm[FailureModesInfo].srcs for fm in ctx.attr.failure_modes]).to_list(),
        "{}/failuremodes.lobster".format(ctx.label.name),
    )
    if fm_lobster_files:
        lobster_files["failuremodes.lobster"] = fm_lobster_files[0]

    safetymeasures_lobster = None
    if lobster_measure_files:
        safetymeasures_lobster = _lobster_trlc(
            ctx,
            lobster_measure_files + fta_and_fm_files + spec_files,
            ctx.file._safetymeasures_lobster_config,
            "safetymeasures.lobster",
        )
    if safetymeasures_lobster:
        lobster_files["safetymeasures.lobster"] = safetymeasures_lobster

    # The .puml diagrams are referenced inline via ``.. uml::`` but
    # must not be toctree documents, so they travel as aux_srcs (symlinked
    # alongside safety_analysis.rst by dependable_element without being indexed).
    sphinx_srcs = depset([safety_analysis_rst])

    return [
        DefaultInfo(
            files = depset(diagram_aux_files + [safety_analysis_rst] + lobster_files.values()),
        ),
        AnalysisInfo(
            name = ctx.label.name,
            lobster_files = lobster_files,
        ),
        SafetyAnalysisProviderInfo(aou_targets = aou_targets),
        SphinxSourcesInfo(
            srcs = sphinx_srcs,
            deps = depset(transitive = [sphinx_srcs]),
            aux_srcs = depset(diagram_aux_files),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_fault_trees = rule(
    implementation = _fault_trees_impl,
    doc = "Generates the fault-tree root-cause TRLC stub from FTA PlantUML diagrams and the " +
          "root-cause lobster traceability file. Build-only rule.",
    attrs = dict(
        {
            "srcs": attr.label_list(
                allow_files = [".puml", ".plantuml"],
                mandatory = False,
                doc = "FTA PlantUML diagram files.  " +
                      "The ``fta_events.trlc`` TRLC stub is generated from them.",
            ),
            "deps": attr.label_list(
                providers = [[FailureModesInfo, TrlcProviderInfo]],
                mandatory = False,
                doc = "``failure_modes`` targets holding the failure modes the diagrams reference.",
            ),
            "spec": attr.label_list(
                allow_files = [".rsl", ".trlc"],
                default = [Label("//bazel/rules/rules_score/trlc/config:score_requirements_model")],
                doc = "TRLC model specification files (``.rsl``) required for import resolution. " +
                      "Defaults to the S-CORE requirements model.",
            ),
            "fta_package": attr.string(
                mandatory = False,
                default = "",
                doc = "TRLC package name for the generated ``fta_events.trlc`` stub file " +
                      "(the ``RootCause`` records generated from ``srcs``). " +
                      "Defaults to a sanitized form of the target's package and name.",
            ),
            "_puml_cli": attr.label(
                default = Label("//plantuml/parser/puml_cli:puml_cli"),
                executable = True,
                allow_files = True,
                cfg = "exec",
                doc = "puml_cli binary used in FTA mode to extract fta_events.trlc.",
            ),
            "_lobster_trlc": attr.label(
                default = Label("@lobster//:lobster-trlc"),
                executable = True,
                allow_files = True,
                cfg = "exec",
                doc = "lobster-trlc executable used to generate the RC lobster file.",
            ),
            "_rc_lobster_config": attr.label(
                default = Label("//bazel/rules/rules_score/lobster/config:fta_root_causes_config"),
                allow_single_file = True,
                doc = "lobster-trlc YAML config for generated RootCause records.",
            ),
        },
        **VERBOSITY_ATTR
    ),
)

_safety_analysis = rule(
    implementation = _safety_analysis_impl,
    doc = "Renders a failure-mode-centric safety-analysis page (overview table, one section per " +
          "failure mode with its root causes and measures, and the fault-tree diagrams) and lobster " +
          "traceability files. Build-only rule; traceability testing is owned by dependability_analysis.",
    attrs = dict(
        {
            "failure_modes": attr.label_list(
                providers = [[FailureModesInfo, TrlcProviderInfo]],
                mandatory = False,
                doc = "``failure_modes`` targets whose records are rendered and traced.",
            ),
            "fault_trees": attr.label(
                providers = [[FaultTreesInfo, TrlcProviderInfo]],
                mandatory = True,
                doc = "``fault_trees`` target providing the root causes.",
            ),
            "safety_measures": attr.label_list(
                allow_files = [".trlc"],
                mandatory = False,
                doc = "Measures addressing the root causes: ``assumptions_of_use`` / ``component_requirements`` " +
                      "targets and ``Mitigation`` ``.trlc`` files. " +
                      "Only the own records of a target are rendered and traced; the targets in its ``deps`` are used to resolve references only.",
            ),
            "spec": attr.label_list(
                allow_files = [".rsl", ".trlc"],
                default = [Label("//bazel/rules/rules_score/trlc/config:score_requirements_model")],
                doc = "TRLC model specification files (``.rsl``) required for import resolution. " +
                      "Defaults to the S-CORE requirements model.",
            ),
            "arch_design": attr.label(
                providers = [ArchitecturalDesignInfo],
                mandatory = True,
                doc = "architectural_design target for traceability.",
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
                doc = "lobster-trlc executable used to generate the safety measure lobster file.",
            ),
            "_safetymeasures_lobster_config": attr.label(
                default = Label("//bazel/rules/rules_score/lobster/config:safetymeasures_config"),
                allow_single_file = True,
                doc = "lobster-trlc YAML config for Mitigation / AoU records.",
            ),
            "_template": attr.label(
                default = Label("//bazel/rules/rules_score:templates/safety_analysis.template.rst"),
                allow_single_file = True,
                doc = "RST template for the safety-analysis page (single ``{body}`` placeholder).",
            ),
        },
        **dict(VERBOSITY_ATTR, **MERGE_LOBSTER_ITEMS_ATTR)
    ),
)

# ============================================================================
# Public Macros
# ============================================================================

def fault_trees(
        name,
        srcs = [],
        deps = [],
        spec = None,
        fta_package = None,
        **kwargs):
    """Define the fault trees of a safety analysis: the root causes of the failure modes.

    FTA diagrams passed via ``srcs`` are preprocessed to extract fault-tree
    topology, emitted as strongly-typed TRLC ``RootCause`` records
    (``fta_events.trlc``, package ``fta_package``), each linking to the
    ``FailureMode`` records of its tree.

    The target emits ``TrlcProviderInfo`` (FTA stub; failure modes as deps), so
    the ``assumptions_of_use`` or ``component_requirements`` targets that
    reference these root causes list it in their ``deps``.  A ``safety_analysis``
    target consumes it via ``fault_trees``.

    Args:
        name: Target name.
        srcs: FTA PlantUML diagram files (``.puml`` / ``.plantuml``).
        deps: ``failure_modes`` targets holding the failure modes the diagrams
            reference.
        spec: TRLC model specification files (``.rsl``) for resolving imports.
            Defaults to the S-CORE requirements model. Override only when using
            a custom TRLC schema.
        fta_package: TRLC package name for the generated ``fta_events.trlc``
            stub file. Defaults to a sanitized form of the target's package
            and ``name``.
        **kwargs: Additional arguments (e.g. ``visibility``, ``tags``).
    """
    _fault_trees(
        name = name,
        srcs = srcs,
        deps = deps,
        spec = spec,
        fta_package = fta_package or "",
        **kwargs
    )

def safety_analysis(
        name,
        arch_design,
        fault_trees,
        failure_modes = [],
        safety_measures = [],
        spec = None,
        **kwargs):
    """Define a safety analysis (FMEA - Failure Mode and Effects Analysis) following S-CORE process guidelines.

    Generates a single, failure-mode-centric ``safety_analysis.rst`` page: an overview
    summary table followed by one section per failure mode (failure-mode detail and
    one card per root cause with its measures), plus the fault-tree diagrams.

    ``Mitigation``, ``AoU`` (own, preventive measure) and ``CompReq`` (control
    measure) records reference a root cause via ``root_causes``/``derived_from``.
    Pass the targets holding the ``AoU`` / ``CompReq`` records and the
    ``Mitigation`` files as ``safety_measures``; they appear on the page and in
    the traceability report.  The ``AoU`` targets among them are the AoUs the
    ``dependable_element`` exposes and forwards to its dependees.

    This is a **build-only** rule.  The combined traceability test
    (FM + FTA root causes) is owned by the ``dependability_analysis`` that wraps
    this target; root-cause coverage by measures is checked by the
    ``dependable_element`` traceability report.

    Args:
        name: Target name.
        arch_design: ``architectural_design`` target for traceability; its
            ``public_api`` is what ``FailureMode.interface`` references.
        fault_trees: ``fault_trees`` target providing the root causes.
        failure_modes: ``failure_modes`` targets whose records are rendered and
            traced. Every failure mode a root cause references must be listed.
        safety_measures: ``assumptions_of_use`` / ``component_requirements``
            targets and ``Mitigation`` ``.trlc`` files addressing the root
            causes. Only the own records of a target are rendered and traced;
            the targets in its ``deps`` are used to resolve references. Other
            record types in a ``.trlc`` file are rejected.
        spec: TRLC model specification files (``.rsl``) for resolving imports.
            Defaults to the S-CORE requirements model. Override only when using
            a custom TRLC schema.
        **kwargs: Additional arguments (e.g. ``visibility``, ``tags``).
    """
    _safety_analysis(
        name = name,
        spec = spec,
        fault_trees = fault_trees,
        failure_modes = failure_modes,
        safety_measures = safety_measures,
        arch_design = arch_design,
        **kwargs
    )
