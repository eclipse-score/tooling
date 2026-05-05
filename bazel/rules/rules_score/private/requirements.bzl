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
Requirements build rules for S-CORE projects.

This module provides macros and rules for defining requirements at any level
(feature, component, etc.) following S-CORE process guidelines.
"""

load("@lobster//:lobster.bzl", "subrule_lobster_trlc")
load("@trlc//:trlc.bzl", "TrlcProviderInfo", "trlc_requirements", "trlc_requirements_test")
load("//bazel/rules/rules_score:providers.bzl", "AssumedSystemRequirementsInfo", "ComponentRequirementsInfo", "FeatureRequirementsInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:rst_to_trlc.bzl", "rst_srcs_to_trlc")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _requirements_impl(ctx):
    """Implementation for requirements rule.

    Collects requirement source files, renders TRLC to RST,
    and extracts lobster traceability items.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo, FeatureRequirementsInfo or ComponentRequirementsInfo,
        and SphinxSourcesInfo
    """
    rendered_files = []

    for src in ctx.attr.srcs:
        trlc_provider = src[TrlcProviderInfo]
        rendered_file = ctx.actions.declare_file("{}_{}.rst".format(ctx.attr.name, src.label.name))

        args = ctx.actions.args()
        args.add("--output", rendered_file.path)
        args.add("--input-dir", ".")
        args.add("--title", ctx.label.name.replace("_", " ").title())
        args.add("--source-files")
        args.add_all(trlc_provider.reqs)

        ctx.actions.run(
            inputs = src[DefaultInfo].files,
            outputs = [rendered_file],
            arguments = [args],
            executable = ctx.executable._renderer,
        )
        rendered_files.append(rendered_file)

    all_srcs = depset(rendered_files)

    lobster_trlc_file, _lobster_trlc = subrule_lobster_trlc(ctx.files.srcs, ctx.file.lobster_config)

    providers = [DefaultInfo(files = all_srcs)]

    if ctx.attr.req_kind == "feature":
        providers.append(FeatureRequirementsInfo(
            srcs = depset([lobster_trlc_file]),
            name = ctx.label.name,
        ))

        # Propagate AssumedSystemRequirementsInfo from deps so the parent
        # dependable_element/component can include assumed system requirement
        # lobster files without listing them separately.
        asr_srcs = [
            dep[AssumedSystemRequirementsInfo].srcs
            for dep in ctx.attr.deps
            if AssumedSystemRequirementsInfo in dep
        ]
        if asr_srcs:
            providers.append(AssumedSystemRequirementsInfo(
                srcs = depset(transitive = asr_srcs),
                name = ctx.label.name,
            ))
    elif ctx.attr.req_kind == "assumed_system":
        providers.append(AssumedSystemRequirementsInfo(
            srcs = depset([lobster_trlc_file]),
            name = ctx.label.name,
        ))
    else:  # component
        providers.append(ComponentRequirementsInfo(
            srcs = depset([lobster_trlc_file]),
            name = ctx.label.name,
        ))

        # Propagate FeatureRequirementsInfo from deps so the parent component
        # can include feature requirement lobster files in its traceability
        # config without listing them separately.
        feat_req_srcs = [
            dep[FeatureRequirementsInfo].srcs
            for dep in ctx.attr.deps
            if FeatureRequirementsInfo in dep
        ]
        if feat_req_srcs:
            providers.append(FeatureRequirementsInfo(
                srcs = depset(transitive = feat_req_srcs),
                name = ctx.label.name,
            ))

    providers.append(SphinxSourcesInfo(
        srcs = all_srcs,
        deps = all_srcs,
        ancillary = depset(),
    ))
    return providers

# ============================================================================
# Rule Definition
# ============================================================================

_requirements = rule(
    implementation = _requirements_impl,
    doc = "Collects requirements documents for S-CORE process compliance",
    attrs = {
        "srcs": attr.label_list(
            providers = [TrlcProviderInfo],
            mandatory = True,
            doc = "TRLC requirement targets providing TrlcProviderInfo",
        ),
        "lobster_config": attr.label(
            allow_single_file = True,
            mandatory = True,
            doc = "Lobster YAML configuration file for traceability extraction",
        ),
        "req_kind": attr.string(
            values = ["feature", "component", "assumed_system"],
            mandatory = True,
            doc = "Kind of requirements: 'feature', 'component', or 'assumed_system'.",
        ),
        "deps": attr.label_list(
            providers = [[FeatureRequirementsInfo], [AssumedSystemRequirementsInfo]],
            doc = "Requirements targets this target derives from. " +
                  "For 'component' req_kind: feature_requirements targets whose lobster files " +
                  "are propagated as FeatureRequirementsInfo. " +
                  "For 'feature' req_kind: assumed_system_requirements targets whose lobster " +
                  "files are propagated as AssumedSystemRequirementsInfo.",
        ),
        "_renderer": attr.label(
            default = Label("@trlc//tools/trlc_rst:trlc_rst"),
            executable = True,
            allow_files = True,
            cfg = "exec",
        ),
    },
    subrules = [subrule_lobster_trlc],
)

# ============================================================================
# Public Macros
# ============================================================================
def _create_trlc_aliases(name, srcs, visibility):
    """Expose stable public aliases for generated trlc_requirements targets.

    For each RST file in *srcs*, a named alias is created so that downstream
    requirement macros can reference the generated trlc_requirements target via
    ``deps`` for cross-package TRLC validation without knowing internal names.
    When a single RST file is given the alias is ``<name>_trlc``; for multiple
    RST files the per-source index is appended (``<name>_trlc_0``, …).

    Args:
        name: Base name used by the enclosing macro (same as passed to
            rst_srcs_to_trlc).
        srcs: Original srcs list passed to the enclosing macro.
        visibility: Bazel visibility to apply to the generated aliases.
    """
    rst_count = len([s for s in srcs if s.endswith(".rst")])
    rst_index = 0
    for i, src in enumerate(srcs):
        if src.endswith(".rst"):
            alias_name = name + "_trlc" if rst_count == 1 else "{}_trlc_{}".format(name, rst_index)
            native.alias(
                name = alias_name,
                actual = ":_{}_trlc_{}".format(name, i),
                visibility = visibility,
            )
            rst_index += 1

def _score_requirements(name, srcs, deps, ref_package, visibility, req_kind, req_deps = []):
    """Shared implementation for feature_requirements and component_requirements.

    Args:
        name: Target name.
        srcs: Mixed list of trlc_requirements labels or RST file paths.
        deps: trlc_requirements labels used as parsing dependencies for RST files.
        ref_package: TRLC package prefix for derived_from cross-references.
        visibility: Bazel visibility specification.
        req_kind: Either "feature" or "component".
        req_deps: Requirements targets for provider propagation (e.g. assumed_system_requirements
            targets for a feature_requirements target).
    """
    trlc_srcs = rst_srcs_to_trlc(name, srcs, deps = deps, ref_package = ref_package or "")
    _requirements(
        name = name,
        srcs = trlc_srcs,
        deps = req_deps,
        lobster_config = Label("//bazel/rules/rules_score/lobster/config:{}_requirement".format(req_kind)),
        req_kind = req_kind,
        visibility = visibility,
    )
    trlc_requirements_test(
        name = name + "_test",
        reqs = trlc_srcs,
        visibility = visibility,
    )
    _create_trlc_aliases(name, srcs, visibility)

def assumed_system_requirements(
        name,
        srcs,
        deps = [],
        ref_package = None,
        visibility = None):
    """Define Assumed System Requirements following S-CORE process guidelines.

    Creates an assumed_system_requirements target (providing AssumedSystemRequirementsInfo
    and SphinxSourcesInfo) and a validation test target named *name*_test.

    Args:
        name: The name of the target.
        srcs: List of trlc_requirements labels (providing TrlcProviderInfo)
            or RST file paths containing ``asr_req`` directives.
            RST files are converted to TRLC automatically.
        deps: Optional list of trlc_requirements labels to include as
            parsing dependencies.  Only used when RST files are present in *srcs*.
        ref_package: TRLC package prefix for derived_from cross-references
            when converting RST sources.
        visibility: Bazel visibility specification.
    """
    _score_requirements(name, srcs, deps, ref_package, visibility, "assumed_system")

def feature_requirements(
        name,
        srcs,
        deps = [],
        req_deps = [],
        ref_package = None,
        visibility = None):
    """Define feature requirements following S-CORE process guidelines.

    Args:
        name: The name of the target.
        srcs: List of trlc_requirements labels (providing TrlcProviderInfo)
            or RST file paths containing ``feat_req`` directives.
            RST files are converted to TRLC automatically.
        deps: Optional list of trlc_requirements labels to include as
            parsing dependencies.  Only used when RST files are present in *srcs*.
        req_deps: Optional list of assumed_system_requirements targets whose
            AssumedSystemRequirementsInfo is propagated alongside this target's
            FeatureRequirementsInfo so that consumers can resolve assumed system
            requirement references without listing them separately.
        ref_package: TRLC package prefix for derived_from cross-references
            when converting RST sources.
        visibility: Bazel visibility specification.
    """
    _score_requirements(name, srcs, deps, ref_package, visibility, "feature", req_deps = req_deps)

def component_requirements(
        name,
        srcs = [],
        deps = [],
        trlc_deps = [],
        ref_package = None,
        visibility = None):
    """Define component requirements following S-CORE process guidelines.

    Args:
        name: The name of the target.
        srcs: List of trlc_requirements labels (providing TrlcProviderInfo) or
            RST file paths containing ``comp_req`` directives.
            RST files are converted to TRLC automatically.
        deps: Optional list of feature_requirements targets whose lobster files are
              propagated alongside this target's ComponentRequirementsInfo, so that
              a parent component rule can resolve derived_from references without
              having to list the feature requirements separately.
        trlc_deps: Optional list of trlc_requirements labels to include as
            parsing dependencies when RST files are present in *srcs*.
        ref_package: TRLC package prefix for derived_from cross-references
            when converting RST sources.
        visibility: Bazel visibility specification.
    """
    trlc_srcs = rst_srcs_to_trlc(name, srcs, deps = trlc_deps, ref_package = ref_package or "")
    _requirements(
        name = name,
        srcs = trlc_srcs,
        deps = deps,
        lobster_config = Label("//bazel/rules/rules_score/lobster/config:component_requirement"),
        req_kind = "component",
        visibility = visibility,
    )

    trlc_requirements_test(
        name = name + "_test",
        reqs = trlc_srcs,
        visibility = visibility,
    )
    _create_trlc_aliases(name, srcs, visibility)
