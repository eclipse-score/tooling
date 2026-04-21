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
Dependability Analysis build rules for S-CORE projects.

A dependability analysis aggregates sub-analysis rules

  * **fmea**              – ``fmea`` rule targets (failure modes, control
                            measures, and optional root cause FTA diagrams).
  * **security_analysis** – security analysis rule targets (placeholder,
                            optional).
"""

load("@lobster//:lobster.bzl", "subrule_lobster_html_report", "subrule_lobster_report")
load("//bazel/rules/rules_score:providers.bzl", "AnalysisInfo", "ArchitecturalDesignInfo", "DependabilityAnalysisInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:lobster_config.bzl", "format_lobster_sources")

# ============================================================================
# Private Helpers
# ============================================================================

def _collect_analysis_providers(sa, rst_srcs_list, rst_deps_list, lobster_files):
    """Collect analysis providers from a single sub-analysis target.

    Updates the provided lists/dicts in-place.

    Args:
        sa:             A sub-analysis target (fmea or security).
        rst_srcs_list:  List of depsets to extend with SphinxSourcesInfo.srcs.
        rst_deps_list:  List of depsets to extend with SphinxSourcesInfo.deps.
        lobster_files:  Dict to update with AnalysisInfo.lobster_files
                        (canonical name → File).
    """
    if SphinxSourcesInfo in sa:
        rst_srcs_list.append(sa[SphinxSourcesInfo].srcs)
        rst_deps_list.append(sa[SphinxSourcesInfo].deps)
    if AnalysisInfo in sa:
        lobster_files.update(sa[AnalysisInfo].lobster_files)

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _dependability_analysis_impl(ctx):
    """Implementation for dependability_analysis rule.

    Collects artefacts from all sub-analysis targets, generates the
    combined lobster traceability config and report, and creates a
    lobster-ci-report test executable.

    Args:
        ctx: Rule context.

    Returns:
        List of providers: DefaultInfo, DependabilityAnalysisInfo,
        SphinxSourcesInfo.
    """
    dfa_rst_files = depset(ctx.files.dfa)

    arch_design_info = None
    if ctx.attr.arch_design and ArchitecturalDesignInfo in ctx.attr.arch_design:
        arch_design_info = ctx.attr.arch_design[ArchitecturalDesignInfo]

    rst_srcs_transitive = [dfa_rst_files]
    rst_deps_transitive = [dfa_rst_files]
    lobster_files = {}  # canonical name → File, merged from all sub-analyses

    # -------------------------------------------------------------------------
    # Collect from fmea targets
    # -------------------------------------------------------------------------
    fmea_output_files = []
    for sa in ctx.attr.fmea:
        fmea_output_files.append(sa[DefaultInfo].files)
        _collect_analysis_providers(sa, rst_srcs_transitive, rst_deps_transitive, lobster_files)

    # -------------------------------------------------------------------------
    # Collect from security_analysis targets
    # -------------------------------------------------------------------------
    security_output_files = []
    for sa in ctx.attr.security_analysis:
        security_output_files.append(sa[DefaultInfo].files)
        _collect_analysis_providers(sa, rst_srcs_transitive, rst_deps_transitive, lobster_files)

    # Architectural design sphinx deps (optional)
    if ctx.attr.arch_design and SphinxSourcesInfo in ctx.attr.arch_design:
        rst_deps_transitive.append(ctx.attr.arch_design[SphinxSourcesInfo].deps)

    all_rst_srcs = depset(transitive = rst_srcs_transitive)
    all_rst_deps = depset(transitive = rst_deps_transitive)

    # =========================================================================
    # Lobster traceability report (combined FM + CM + FTA)
    # =========================================================================
    lobster_report_file = None
    lobster_html_file = None
    report_files = []

    all_lobster_file_objects = lobster_files.values()
    arch_lobster_files = arch_design_info.lobster_files.to_list() if arch_design_info else []
    all_lobster_file_objects = list(all_lobster_file_objects) + arch_lobster_files
    if all_lobster_file_objects:
        lobster_config = ctx.actions.declare_file(
            "{}/traceability_config".format(ctx.label.name),
        )

        ctx.actions.expand_template(
            template = ctx.file._lobster_sa_template,
            output = lobster_config,
            substitutions = {
                "{ARCH_SOURCES}": format_lobster_sources(arch_lobster_files),
                "{FM_SOURCES}": format_lobster_sources([lobster_files["failuremodes.lobster"]] if "failuremodes.lobster" in lobster_files else []),
                "{CM_SOURCES}": format_lobster_sources([lobster_files["controlmeasures.lobster"]] if "controlmeasures.lobster" in lobster_files else []),
                "{RC_SOURCES}": format_lobster_sources([lobster_files["root_causes.lobster"]] if "root_causes.lobster" in lobster_files else []),
            },
        )

        lobster_report_file = subrule_lobster_report(all_lobster_file_objects, lobster_config)
        lobster_html_file = subrule_lobster_html_report(lobster_report_file)

        if lobster_report_file:
            report_files.append(lobster_report_file)
        if lobster_html_file:
            report_files.append(lobster_html_file)

    # =========================================================================
    # Test executable (lobster-ci-report)
    # =========================================================================
    test_executable = ctx.actions.declare_file(
        "{}_lobster_ci_test_executable".format(ctx.attr.name),
    )

    runfiles_files = []
    if lobster_report_file:
        ctx.actions.write(
            output = test_executable,
            content = "set -o pipefail; {} {}".format(
                ctx.executable._lobster_ci_report.short_path,
                lobster_report_file.short_path,
            ),
        )
        runfiles_files = [ctx.executable._lobster_ci_report, lobster_report_file]
    else:
        ctx.actions.write(output = test_executable, content = "exit 0")

    runfiles = ctx.runfiles(files = runfiles_files)
    if lobster_report_file:
        runfiles = runfiles.merge(ctx.attr._lobster_ci_report[DefaultInfo].default_runfiles)

    # =========================================================================
    # Build providers
    # =========================================================================
    all_output_files = depset(
        report_files,
        transitive = [dfa_rst_files] + fmea_output_files + security_output_files,
    )

    return [
        DefaultInfo(
            runfiles = runfiles,
            files = all_output_files,
            executable = test_executable,
        ),
        DependabilityAnalysisInfo(
            fmea = depset(transitive = fmea_output_files),
            security_analysis = depset(transitive = security_output_files),
            dfa = dfa_rst_files,
            arch_design = arch_design_info,
            name = ctx.label.name,
            lobster_files = lobster_files,
        ),
        SphinxSourcesInfo(
            srcs = all_rst_srcs,
            deps = all_rst_deps,
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_dependability_analysis_test = rule(
    implementation = _dependability_analysis_impl,
    doc = "Aggregates dependability analysis sub-analyses (fmea, security_analysis) " +
          "and validates the combined traceability chain via lobster-ci-report.",
    attrs = {
        "fmea": attr.label_list(
            providers = [AnalysisInfo],
            mandatory = False,
            doc = "fmea rule targets (failure modes + control measures).",
        ),
        "security_analysis": attr.label_list(
            providers = [AnalysisInfo],
            mandatory = False,
            doc = "Security analysis rule targets (placeholder -- not yet implemented).",
        ),
        "dfa": attr.label_list(
            allow_files = [".rst", ".md"],
            mandatory = False,
            doc = "Dependent Failure Analysis (DFA) documentation (placeholder).",
        ),
        "arch_design": attr.label(
            providers = [ArchitecturalDesignInfo],
            mandatory = False,
            doc = "Reference to architectural_design target for interface tracing.",
        ),
        "_lobster_ci_report": attr.label(
            default = "@lobster//:lobster-ci-report",
            executable = True,
            cfg = "exec",
            doc = "lobster-ci-report executable for test execution.",
        ),
        "_lobster_sa_template": attr.label(
            default = Label("//bazel/rules/rules_score/lobster/config:lobster_sa_template"),
            allow_single_file = True,
            doc = "Lobster config template for safety analysis traceability.",
        ),
    },
    subrules = [subrule_lobster_report, subrule_lobster_html_report],
    test = True,
)

# ============================================================================
# Public Macro
# ============================================================================

def dependability_analysis(
        name,
        fmea = [],
        security_analysis = [],
        dfa = [],
        arch_design = None,
        visibility = None,
        tags = []):
    """Define dependability analysis following S-CORE process guidelines.

    Aggregates up to three sub-analysis rules and validates the combined
    traceability chain (FM + CM + FTA) via ``lobster-ci-report``.

    The target is a Bazel **test rule**.  During ``bazel build`` it produces
    aggregated documentation artefacts and the lobster traceability report.
    During ``bazel test`` it runs ``lobster-ci-report`` to validate the chain.

    Args:
        name: The name of the dependability analysis target.
        fmea: Optional list of ``fmea`` rule target labels.
        security_analysis: Optional list of security analysis rule target
            labels (placeholder -- not yet implemented).
        dfa: Optional list of ``.rst``/``.md`` DFA documentation files
            (placeholder).
        arch_design: Optional label to an ``architectural_design`` target
            (placeholder).
        visibility: Bazel visibility.
        tags: Additional Bazel tags.

    Example:
        .. code-block:: starlark

            dependability_analysis(
                name = "my_da",
                fmea = [":my_fmea"],
            )
    """
    _dependability_analysis_test(
        name = name,
        fmea = fmea,
        security_analysis = security_analysis,
        dfa = dfa,
        arch_design = arch_design,
        visibility = visibility,
        tags = tags,
    )
