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
Shared internal requirements rule for S-CORE projects.

Not intended for direct use. See feature_requirements.bzl,
component_requirements.bzl, and assumed_system_requirements.bzl for the
public-facing macros.
"""

load("@lobster//:lobster.bzl", "subrule_lobster_trlc")
load("@trlc//:trlc.bzl", "TrlcProviderInfo", "subrule_trlc_image_stage")
load("//bazel/rules/rules_score:providers.bzl", "AssumedSystemRequirementsInfo", "AssumptionsOfUseInfo", "ComponentRequirementsInfo", "FeatureRequirementsInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:rst_to_trlc.bzl", "rst_to_trlc")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _requirements_impl(ctx):
    """Shared implementation for all requirement kinds.


    Args:
        ctx: Rule context.

    Returns:
        [DefaultInfo, TrlcProviderInfo, <kind>RequirementsInfo, SphinxSourcesInfo]
    """

    # -------------------------------------------------------------------------
    # Assemble TrlcProviderInfo
    # -------------------------------------------------------------------------
    transitive_spec = []
    transitive_reqs = []
    for dep in ctx.attr.deps:
        trlc_info = dep[TrlcProviderInfo]
        transitive_spec.append(trlc_info.spec)
        transitive_reqs.append(trlc_info.reqs)
        transitive_reqs.append(trlc_info.deps)

    own_spec_files = ctx.attr.spec[DefaultInfo].files
    spec_depset = depset(transitive = [own_spec_files] + transitive_spec)
    deps_depset = depset(transitive = transitive_reqs)

    # All files needed for TRLC parsing: own sources + spec RSL + transitive deps.
    # This matches DefaultInfo.files of an equivalent trlc_requirements target so
    # that trlc_requirements_test(reqs=[":this_target"]) works out of the box.
    all_trlc_files = depset(ctx.files.srcs, transitive = [spec_depset, deps_depset])

    # -------------------------------------------------------------------------
    # Render TRLC → RST for Sphinx documentation.
    # --source-files: own .trlc files — rendered AND registered for parsing.
    # --dep-files:    spec .rsl + transitive .trlc deps — parsed but not rendered.
    # -------------------------------------------------------------------------
    rendered_file = ctx.actions.declare_file("{}.rst".format(ctx.attr.name))
    dep_files_depset = depset(transitive = [spec_depset, deps_depset])

    render_args = ctx.actions.args()
    render_args.add("--output", rendered_file.path)
    render_args.add_all("--source-files", ctx.files.srcs)
    render_args.add_all("--dep-files", dep_files_depset)

    ctx.actions.run(
        inputs = all_trlc_files,
        outputs = [rendered_file],
        executable = ctx.executable._renderer,
        arguments = [render_args],
        progress_message = "Rendering TRLC to RST for %s" % ctx.label,
        mnemonic = "TrlcToRst",
    )

    # -------------------------------------------------------------------------
    # Lobster traceability extraction.
    # -------------------------------------------------------------------------
    lobster_file, _ = subrule_lobster_trlc(all_trlc_files.to_list(), ctx.file.lobster_config)

    # -------------------------------------------------------------------------
    # Build the kind-specific domain provider.
    # -------------------------------------------------------------------------
    if ctx.attr.req_kind == "feature":
        req_provider = FeatureRequirementsInfo(
            srcs = depset([lobster_file]),
            name = ctx.label.name,
        )
    elif ctx.attr.req_kind == "component":
        req_provider = ComponentRequirementsInfo(
            srcs = depset([lobster_file]),
            name = ctx.label.name,
        )
    elif ctx.attr.req_kind == "aou":
        req_provider = AssumptionsOfUseInfo(
            aou_lobster = depset([lobster_file]),
            name = ctx.label.name,
        )
    else:  # assumed_system
        req_provider = AssumedSystemRequirementsInfo(
            srcs = depset([lobster_file]),
            name = ctx.label.name,
        )

    image_outputs = subrule_trlc_image_stage(ctx.files.image_srcs)

    sphinx_srcs = depset([rendered_file] + image_outputs)

    transitive_sphinx = [sphinx_srcs]
    for dep in ctx.attr.deps:
        if SphinxSourcesInfo in dep:
            transitive_sphinx.append(dep[SphinxSourcesInfo].deps)

    return [
        DefaultInfo(files = all_trlc_files),
        TrlcProviderInfo(
            spec = spec_depset,
            reqs = depset(ctx.files.srcs),
            deps = deps_depset,
        ),
        req_provider,
        SphinxSourcesInfo(
            srcs = sphinx_srcs,
            deps = depset(transitive = transitive_sphinx),
            aux_srcs = depset(),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_score_requirements_rule = rule(
    implementation = _requirements_impl,
    doc = """Shared internal rule for all S-CORE requirement kinds.

    Accepts raw .trlc source files and emits TrlcProviderInfo so that
    downstream requirement targets can list this target in their deps
    directly, without needing an intermediate trlc_requirements wrapper.
    """,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".trlc"],
            mandatory = True,
            doc = "TRLC source files containing requirement records.",
        ),
        "deps": attr.label_list(
            providers = [TrlcProviderInfo],
            default = [],
            doc = "Other requirement targets whose TRLC records are needed for cross-reference parsing.",
        ),
        "req_kind": attr.string(
            values = ["feature", "component", "assumed_system", "aou"],
            mandatory = True,
            doc = "Kind of requirements; determines which domain provider is emitted.",
        ),
        "lobster_config": attr.label(
            allow_single_file = True,
            mandatory = True,
            doc = "Lobster YAML configuration file for traceability extraction.",
        ),
        "spec": attr.label(
            default = Label("//bazel/rules/rules_score/trlc/config:score_requirements_model"),
            doc = "TRLC specification target providing the RSL files that define the requirement types. Defaults to the S-CORE requirements model.",
        ),
        "image_srcs": attr.label_list(
            allow_files = [".svg", ".png"],
            default = [],
            doc = "Image files (.svg or .png) to stage next to the rendered RST. The package-relative path of each file (e.g. 'diagrams/arch.png') must match the path written in a ``.. image::`` directive inside the requirement description field.",
        ),
        "_renderer": attr.label(
            default = Label("@trlc//tools/trlc_rst:trlc_rst"),
            executable = True,
            cfg = "exec",
            doc = "TRLC-to-RST renderer tool.",
        ),
    },
    subrules = [subrule_lobster_trlc, subrule_trlc_image_stage],
)

def score_requirements_rule(
        name,
        srcs,
        req_kind,
        lobster_config,
        deps = [],
        spec = Label("//bazel/rules/rules_score/trlc/config:score_requirements_model"),
        ref_package = "",
        **kwargs):
    """Macro wrapper around _score_requirements_rule with RST support.

    Each entry in srcs is classified as follows:
      - ".rst" files are converted to .trlc via rst_to_trlc and treated as
        own sources.
      - ".trlc" files are passed through unchanged as own sources.
      - Any other label is assumed to already provide TrlcProviderInfo (e.g.
        an existing trlc_requirements/assumed_system_requirements/... target)
        and is routed to ``deps`` instead, since only raw TRLC files may be
        listed in the underlying rule's ``srcs``.

    Args:
        ref_package: TRLC package prefix used for derived_from cross-references
            when converting RST sources (e.g. "AssumedSystemRequirements" for
            feature requirements that derive from ASR).

    Returns:
        List of resolved labels corresponding to srcs (after any .rst-to-.trlc
        conversion), in the same order. Useful for callers that need to run
        trlc_requirements_test against the same source set.
    """
    trlc_srcs = []
    extra_deps = []
    resolved_srcs = []
    for i, src in enumerate(srcs):
        if type(src) == type("") and src.endswith(".rst"):
            gen_name = "_{}_rst_gen_{}".format(name, i)
            rst_to_trlc(
                name = gen_name,
                srcs = [src],
                ref_package = ref_package,
            )
            trlc_srcs.append(":" + gen_name)
            resolved_srcs.append(":" + gen_name)
        elif type(src) == type("") and src.endswith(".trlc"):
            trlc_srcs.append(src)
            resolved_srcs.append(src)
        else:
            extra_deps.append(src)
            resolved_srcs.append(src)

    _score_requirements_rule(
        name = name,
        srcs = trlc_srcs,
        deps = deps + extra_deps,
        req_kind = req_kind,
        lobster_config = lobster_config,
        spec = spec,
        **kwargs
    )

    return resolved_srcs
