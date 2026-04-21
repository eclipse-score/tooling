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
load("@trlc//:trlc.bzl", "TrlcProviderInfo", "trlc_requirements_test")
load("//bazel/rules/rules_score:providers.bzl", "ComponentRequirementsInfo", "FeatureRequirementsInfo", "SphinxSourcesInfo")

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

    if ctx.attr.req_kind == "feature":
        req_provider = FeatureRequirementsInfo(
            srcs = depset([lobster_trlc_file]),
            name = ctx.label.name,
        )
    else:
        req_provider = ComponentRequirementsInfo(
            srcs = depset([lobster_trlc_file]),
            name = ctx.label.name,
        )

    return [
        DefaultInfo(files = all_srcs),
        req_provider,
        SphinxSourcesInfo(
            srcs = all_srcs,
            deps = all_srcs,
        ),
    ]

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
            values = ["feature", "component"],
            mandatory = True,
            doc = "Kind of requirements: 'feature' or 'component'",
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

def feature_requirements(
        name,
        srcs,
        visibility = None):
    """Define feature requirements following S-CORE process guidelines.

    Args:
        name: The name of the target.
        srcs: List of labels to trlc_requirements targets providing TrlcProviderInfo.
        visibility: Bazel visibility specification.
    """
    _requirements(
        name = name,
        srcs = srcs,
        lobster_config = Label("//bazel/rules/rules_score/config:feature_requirement"),
        req_kind = "feature",
        visibility = visibility,
    )

    trlc_requirements_test(
        name = name + "_test",
        reqs = srcs,
        visibility = visibility,
    )

def component_requirements(
        name,
        srcs = [],
        visibility = None):
    """Define component requirements following S-CORE process guidelines.

    Args:
        name: The name of the target.
        srcs: List of labels to trlc_requirements targets providing TrlcProviderInfo.
        visibility: Bazel visibility specification.
    """
    _requirements(
        name = name,
        srcs = srcs,
        lobster_config = Label("//bazel/rules/rules_score/config:component_requirement"),
        req_kind = "component",
        visibility = visibility,
    )

    trlc_requirements_test(
        name = name + "_test",
        reqs = srcs,
        visibility = visibility,
    )
