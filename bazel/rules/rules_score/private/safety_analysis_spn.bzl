# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
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
Safety analysis read from a Sphinx-Needs FMEA instead of TRLC.

``safety_analysis_spn`` is the Sphinx-Needs counterpart of ``failure_modes``,
``fault_trees`` and ``safety_analysis`` together. The failure modes are
``comp_saf_fmea`` needs of a ``needs.json``; the fault trees stay ``.puml``
diagrams. ``safety_analysis_spn`` converts both into the LOBSTER files of the
TRLC rules and emits ``AnalysisInfo``, so ``dependability_analysis`` and
``dependable_element`` accept it unchanged.
"""

load("//bazel/rules/rules_score:providers.bzl", "AnalysisInfo", "ArchitecturalDesignInfo", "AssumptionsOfUseInfo", "ComponentRequirementsInfo", "SafetyAnalysisProviderInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:lobster_sphinx_needs.bzl", "needs_json_labels")
load("//bazel/rules/rules_score/private:verbosity.bzl", "VERBOSITY_ATTR", "get_log_level")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _safety_analysis_spn_impl(ctx):
    package = ctx.attr.package
    fta_package = package + "_fta"
    diagrams = [f for f in ctx.files.fault_trees if f.basename != "fta_metamodel.puml"]

    # 1. Root causes: the same puml_cli FTA stub the fault_trees rule generates.
    fta_events = None
    if diagrams:
        fta_events = ctx.actions.declare_file("{}/fta_events.trlc".format(ctx.label.name))
        args = ctx.actions.args()
        for src in diagrams:
            args.add("--file", src.path)
        args.add("--fta-output-dir", fta_events.dirname)
        args.add("--fta-package", fta_package)
        args.add("--log-level", get_log_level(ctx))
        ctx.actions.run(
            inputs = diagrams,
            outputs = [fta_events],
            executable = ctx.executable._puml_cli,
            arguments = [args],
            mnemonic = "PumlFta",
            progress_message = "Processing root cause FTA diagrams for %s" % ctx.label.name,
        )

    # Staged next to the page so ``.. uml:: <basename>`` resolves.
    staged_diagrams = []
    for src in diagrams:
        staged = ctx.actions.declare_file("{}/{}".format(ctx.label.name, src.basename))
        ctx.actions.symlink(output = staged, target_file = src)
        staged_diagrams.append(staged)

    # 2. Failure modes, root causes and assessments as LOBSTER items.
    api_files = []
    if ctx.attr.arch_design:
        api_files = ctx.attr.arch_design[ArchitecturalDesignInfo].public_api_lobster_files.to_list()
    measure_depsets = []
    aou_targets = []
    for measure in ctx.attr.safety_measures:
        if AssumptionsOfUseInfo in measure:
            measure_depsets.append(measure[AssumptionsOfUseInfo].aou_lobster)
            aou_targets.append(measure)
        else:
            measure_depsets.append(measure[ComponentRequirementsInfo].srcs)
    measure_files = depset(transitive = measure_depsets).to_list()

    outputs = {
        name: ctx.actions.declare_file("{}/{}".format(ctx.label.name, name))
        for name in ["failuremodes.lobster", "fta_root_causes.lobster", "assessments.lobster"]
    }
    rst_file = ctx.actions.declare_file("{}/safety_analysis.rst".format(ctx.label.name))

    args = ctx.actions.args()
    args.add("--arch-id", ctx.attr.arch_id)
    args.add("--package", package)
    args.add_all([f for f in ctx.files._spec if f.extension == "rsl"], before_each = "--spec")
    args.add_all(api_files, before_each = "--api")
    args.add_all(measure_files, before_each = "--measures")
    if fta_events:
        args.add("--fta-events", fta_events)
    args.add_all([f.basename for f in staged_diagrams], before_each = "--diagram")
    args.add("--failuremodes-out", outputs["failuremodes.lobster"])
    args.add("--root-causes-out", outputs["fta_root_causes.lobster"])
    args.add("--assessments-out", outputs["assessments.lobster"])
    args.add("--rst-out", rst_file)
    args.add("--title", ctx.label.name)
    args.add("--log-level", get_log_level(ctx))
    args.add_all(ctx.files.needs)
    ctx.actions.run(
        executable = ctx.executable._safety_analysis_spn,
        inputs = ctx.files.needs + ctx.files._spec + api_files + measure_files + ([fta_events] if fta_events else []),
        outputs = outputs.values() + [rst_file],
        arguments = [args],
        mnemonic = "SafetyAnalysisSpn",
        progress_message = "Converting the Sphinx-Needs FMEA of %s" % ctx.label.name,
    )

    # Without fault trees there are no root causes to trace or assess.
    lobster_files = {"failuremodes.lobster": outputs["failuremodes.lobster"]}
    if diagrams:
        lobster_files["fta_root_causes.lobster"] = outputs["fta_root_causes.lobster"]
        lobster_files["assessments.lobster"] = outputs["assessments.lobster"]

    sphinx_srcs = depset([rst_file])
    return [
        DefaultInfo(files = depset(staged_diagrams + [rst_file] + lobster_files.values())),
        AnalysisInfo(name = ctx.label.name, lobster_files = lobster_files),
        SafetyAnalysisProviderInfo(aou_targets = aou_targets),
        SphinxSourcesInfo(
            srcs = sphinx_srcs,
            deps = sphinx_srcs,
            aux_srcs = depset(staged_diagrams),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_safety_analysis_spn = rule(
    implementation = _safety_analysis_spn_impl,
    doc = "Safety analysis from Sphinx-Needs FMEA needs and FTA diagrams; counterpart of safety_analysis. Build-only rule.",
    attrs = dict(
        {
            "needs": attr.label_list(
                allow_files = [".json"],
                mandatory = True,
                doc = "needs.json files with the comp_saf_fmea needs and the needs they violate.",
            ),
            "arch_id": attr.string(
                mandatory = True,
                doc = "Only convert the comp_saf_fmea needs whose violates link names this need ID.",
            ),
            "package": attr.string(
                mandatory = True,
                doc = "Package of the failure modes: the $FailureMode macros name them as <package>.<need ID>.",
            ),
            "fault_trees": attr.label_list(
                allow_files = [".puml"],
                doc = "FTA PlantUML diagrams; fta_metamodel.puml is ignored.",
            ),
            "arch_design": attr.label(
                providers = [ArchitecturalDesignInfo],
                doc = "architectural_design target whose public API methods the violated real_arc_int_op needs name.",
            ),
            "safety_measures": attr.label_list(
                providers = [[AssumptionsOfUseInfo], [ComponentRequirementsInfo]],
                doc = "AoU and component requirement targets an assessment may list as measures.",
            ),
            "_spec": attr.label_list(
                allow_files = [".rsl", ".trlc"],
                default = [Label("//bazel/rules/rules_score/trlc/config:score_requirements_model")],
                doc = "TRLC model documenting the fault model IDs on its Guideword enum.",
            ),
            "_puml_cli": attr.label(
                default = Label("//plantuml/parser/puml_cli:puml_cli"),
                executable = True,
                cfg = "exec",
            ),
            "_safety_analysis_spn": attr.label(
                default = Label("//bazel/rules/rules_score:safety_analysis_spn"),
                executable = True,
                cfg = "exec",
            ),
        },
        **VERBOSITY_ATTR
    ),
)

# ============================================================================
# Public Macro
# ============================================================================

def safety_analysis_spn(
        name,
        needs,
        arch_id,
        package,
        fault_trees = [],
        arch_design = None,
        safety_measures = [],
        **kwargs):
    """Safety analysis from a Sphinx-Needs FMEA; counterpart of ``safety_analysis``.

    Converts the ``comp_saf_fmea`` needs violating ``arch_id`` into failure
    modes and the ``fault_trees`` diagrams into root causes. A need rated
    ``sufficient: yes`` is the assessment of the root causes of its failure
    mode, with its ``mitigated_by`` links among ``safety_measures`` as
    measures. Pass the target to ``dependability_analysis(safety_analysis = ...)``.

    Checks: the ``fault_id`` values must be fault model IDs (build error);
    unknown failure modes in the fault trees and violated operations without
    a public API method are unknown tracing targets for LOBSTER.

    Args:
        name: The name of the target.
        needs: score_docs_as_code ``docs_bundle`` targets or ``needs.json`` files
            holding the FMEA needs and the ``real_arc_int``/``real_arc_int_op``
            needs they violate.
        arch_id: ID of the ``comp_arc_sta``/``comp_arc_dyn`` need the failure
            modes violate.
        package: Package of the failure modes. The ``$FailureMode`` macros of
            the fault trees name a failure mode as ``<package>.<need ID>``.
        fault_trees: FTA ``.puml`` diagrams.
        arch_design: ``architectural_design`` target with the public API.
        safety_measures: ``assumptions_of_use(_spn)`` and
            ``component_requirements(_spn)`` targets holding the measures. The
            AoU targets become the own AoUs of the dependable element.
        **kwargs: Common Bazel attributes (e.g. visibility, testonly).
    """
    _safety_analysis_spn(
        name = name,
        needs = needs_json_labels(needs),
        arch_id = arch_id,
        package = package,
        fault_trees = fault_trees,
        arch_design = arch_design,
        safety_measures = safety_measures,
        **kwargs
    )
