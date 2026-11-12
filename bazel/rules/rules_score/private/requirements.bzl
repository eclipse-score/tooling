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
load("//bazel/rules/rules_score:providers.bzl", "AssumedSystemRequirementsInfo", "AssumptionsOfUseInfo", "ComponentRequirementsInfo", "FailureModesInfo", "FeatureRequirementsInfo", "SphinxSourcesInfo")
load("//bazel/rules/rules_score/private:lobster_sphinx_needs.bzl", "needs_json_labels", "subrule_lobster_sphinx_needs")

_DEFAULT_SPEC = Label("//bazel/rules/rules_score/trlc/config:score_requirements_model")

# Distinct RST heading per requirement kind; trlc_rst's own default ("Requirements")
# is identical for every kind, which makes sibling docs (e.g. Assumed System
# Requirements and Assumptions of Use) indistinguishable in the Sphinx sidebar.
_REQ_KIND_TITLES = {
    "feature": "Feature Requirements",
    "component": "Component Requirements",
    "assumed_system": "Assumed System Requirements",
    "aou": "Assumptions of Use",
    "failure_mode": "Failure Modes",
}

# Sphinx-Needs type of each requirement kind (S-CORE metamodel).
_REQ_KIND_NEED_TYPES = {
    "feature": "feat_req",
    "component": "comp_req",
    "assumed_system": "stkh_req",
    "aou": "aou_req",
}

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _failure_modes_from_needs(ctx, deps_depset):
    """Generate the FailureMode records of the comp_saf_fmea needs (TRLC file)."""
    trlc_file = ctx.actions.declare_file("{}_needs.trlc".format(ctx.label.name))
    api_files = [f for f in deps_depset.to_list() if f.extension == "trlc"]
    package = ctx.attr.needs_package or "".join([c if c.isalnum() else "_" for c in ctx.label.name.elems()])

    args = ctx.actions.args()
    args.add("--package", package)
    args.add("--out", trlc_file)
    args.add_all(api_files, before_each = "--api-trlc")
    args.add_all(ctx.files.needs)
    ctx.actions.run(
        executable = ctx.executable._fmea_needs_to_trlc,
        inputs = ctx.files.needs + api_files,
        outputs = [trlc_file],
        arguments = [args],
        mnemonic = "FmeaNeedsToTrlc",
        progress_message = "Generating failure modes from needs for %s" % ctx.label,
    )
    return trlc_file, package

def _requirements_impl(ctx):
    """Shared implementation for all requirement kinds.


    Args:
        ctx: Rule context.

    Returns:
        [DefaultInfo, TrlcProviderInfo, <kind>RequirementsInfo, SphinxSourcesInfo]
    """

    # -------------------------------------------------------------------------
    # Split srcs: targets providing TrlcProviderInfo are dependencies, anything
    # else contributes its files as own TRLC sources.
    # -------------------------------------------------------------------------
    own_srcs = []
    deps = list(ctx.attr.deps)
    for src in ctx.attr.srcs:
        if TrlcProviderInfo in src:
            deps.append(src)
        else:
            own_srcs.extend(src.files.to_list())

    # -------------------------------------------------------------------------
    # Assemble TrlcProviderInfo
    # -------------------------------------------------------------------------
    transitive_spec = []
    transitive_reqs = []
    for dep in deps:
        if ctx.attr.req_kind == "aou" and AssumptionsOfUseInfo in dep:
            fail("{}: deps must not hold the assumptions_of_use target {}; its AoUs would be extracted as own AoUs".format(ctx.label, dep.label))
        trlc_info = dep[TrlcProviderInfo]
        transitive_spec.append(trlc_info.spec)
        transitive_reqs.append(trlc_info.reqs)
        transitive_reqs.append(trlc_info.deps)

    own_spec_files = depset(transitive = [t[DefaultInfo].files for t in ctx.attr.spec])
    spec_depset = depset(transitive = [own_spec_files] + transitive_spec)
    deps_depset = depset(transitive = transitive_reqs)

    # FMEA needs become generated FailureMode records, so fault trees can reference them.
    from_needs_as_trlc = ctx.attr.req_kind == "failure_mode" and bool(ctx.files.needs)
    needs_package = None
    if from_needs_as_trlc:
        needs_trlc, needs_package = _failure_modes_from_needs(ctx, deps_depset)
        own_srcs.append(needs_trlc)

    # All files needed for TRLC parsing: own sources + spec RSL + transitive deps.
    # This matches DefaultInfo.files of an equivalent trlc_requirements target so
    # that trlc_requirements_test(reqs=[":this_target"]) works out of the box.
    all_trlc_files = depset(own_srcs, transitive = [spec_depset, deps_depset])

    # A target fed only with needs.json files has no TRLC records to render or trace.
    use_trlc = bool(own_srcs) or not ctx.files.needs
    lobster_files = []
    rendered_files = []

    if use_trlc:
        # ---------------------------------------------------------------------
        # Render TRLC → RST for Sphinx documentation.
        # --source-files: own .trlc/.trlc.md files — rendered AND registered for parsing.
        # --dep-files:    spec .rsl + transitive .trlc deps — parsed but not rendered.
        # ---------------------------------------------------------------------
        rendered_file = ctx.actions.declare_file("{}.rst".format(ctx.attr.name))
        dep_files_depset = depset(transitive = [spec_depset, deps_depset])

        render_args = ctx.actions.args()
        render_args.add("--output", rendered_file.path)
        render_args.add("--title", _REQ_KIND_TITLES[ctx.attr.req_kind])
        render_args.add_all("--source-files", own_srcs)
        render_args.add_all("--dep-files", dep_files_depset)

        ctx.actions.run(
            inputs = all_trlc_files,
            outputs = [rendered_file],
            executable = ctx.executable._renderer,
            arguments = [render_args],
            progress_message = "Rendering TRLC to RST for %s" % ctx.label,
            mnemonic = "TrlcToRst",
        )
        rendered_files.append(rendered_file)

        # ---------------------------------------------------------------------
        # Lobster traceability extraction.
        # ---------------------------------------------------------------------
        lobster_file, _ = subrule_lobster_trlc(all_trlc_files.to_list(), ctx.file.lobster_config)
        lobster_files.append(lobster_file)

    # -------------------------------------------------------------------------
    # Sphinx-Needs requirements: the same LOBSTER items as for TRLC records.
    # -------------------------------------------------------------------------
    if ctx.files.needs and not from_needs_as_trlc:
        if ctx.attr.req_kind not in _REQ_KIND_NEED_TYPES:
            fail("{}: needs is not supported for {} requirements".format(ctx.label, ctx.attr.req_kind))
        needs_lobster, needs_rst = subrule_lobster_sphinx_needs(
            ctx.files.needs,
            _REQ_KIND_NEED_TYPES[ctx.attr.req_kind],
            ctx.attr.needs_satisfied_by,
            "{}{}.rst".format(ctx.attr.name, "_needs" if use_trlc else ""),
            _REQ_KIND_TITLES[ctx.attr.req_kind],
        )
        lobster_files.append(needs_lobster)
        rendered_files.append(needs_rst)

    lobster_depset = depset(lobster_files)

    # -------------------------------------------------------------------------
    # Build the kind-specific domain provider.
    # -------------------------------------------------------------------------
    if ctx.attr.req_kind == "feature":
        req_provider = FeatureRequirementsInfo(
            srcs = lobster_depset,
            name = ctx.label.name,
        )
    elif ctx.attr.req_kind == "component":
        req_provider = ComponentRequirementsInfo(
            srcs = lobster_depset,
            name = ctx.label.name,
        )
    elif ctx.attr.req_kind == "aou":
        req_provider = AssumptionsOfUseInfo(
            aou_lobster = lobster_depset,
            name = ctx.label.name,
        )
    elif ctx.attr.req_kind == "failure_mode":
        req_provider = FailureModesInfo(
            srcs = lobster_depset,
            name = ctx.label.name,
            needs = depset(ctx.files.needs if from_needs_as_trlc else []),
            package = needs_package,
        )
    else:  # assumed_system
        req_provider = AssumedSystemRequirementsInfo(
            srcs = lobster_depset,
            name = ctx.label.name,
        )

    image_outputs = subrule_trlc_image_stage(ctx.files.image_srcs)

    sphinx_srcs = depset(rendered_files + image_outputs)

    transitive_sphinx = [sphinx_srcs]
    for dep in deps:
        if SphinxSourcesInfo in dep:
            transitive_sphinx.append(dep[SphinxSourcesInfo].deps)

    return [
        DefaultInfo(files = all_trlc_files),
        TrlcProviderInfo(
            spec = spec_depset,
            reqs = depset(own_srcs),
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

    Accepts raw .trlc or .trlc.md source files (or targets generating them) and
    emits TrlcProviderInfo so that downstream requirement targets can list this
    target in their deps directly, without needing an intermediate
    trlc_requirements wrapper.

    Sphinx-Needs requirements are accepted as ``needs.json`` files in ``needs``;
    they are converted by ``lobster_sphinx_needs`` into the same LOBSTER items.
    """,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".trlc", ".trlc.md"],
            default = [],
            doc = "TRLC source files containing requirement records, or targets generating them. Targets providing TrlcProviderInfo are treated as deps.",
        ),
        "needs": attr.label_list(
            allow_files = [".json"],
            default = [],
            doc = "Sphinx-Needs needs.json files whose requirements of this kind's need type are converted by lobster_sphinx_needs.",
        ),
        "needs_satisfied_by": attr.string(
            doc = "Only convert needs whose satisfied_by link names this feature or component ID.",
        ),
        "needs_package": attr.string(
            doc = "failure_mode only: TRLC package of the FailureMode records generated from the needs. Defaults to the sanitized target name.",
        ),
        "deps": attr.label_list(
            providers = [TrlcProviderInfo],
            default = [],
            doc = "Other requirement targets whose TRLC records are needed for cross-reference parsing.",
        ),
        "req_kind": attr.string(
            values = ["feature", "component", "assumed_system", "aou", "failure_mode"],
            mandatory = True,
            doc = "Kind of requirements; determines which domain provider is emitted.",
        ),
        "lobster_config": attr.label(
            allow_single_file = True,
            mandatory = True,
            doc = "Lobster YAML configuration file for traceability extraction.",
        ),
        "spec": attr.label_list(
            default = [_DEFAULT_SPEC],
            doc = "TRLC specification targets providing the RSL files that define the requirement types. The S-CORE requirements model is always included; this adds zero or more additional spec targets on top of it. Populated by the score_requirements_rule macro, which performs the merge.",
        ),
        "image_srcs": attr.label_list(
            allow_files = [".svg", ".png", ".puml"],
            default = [],
            doc = "Image and diagram files (.svg, .png, or .puml) to stage next to the rendered RST.",
        ),
        "_renderer": attr.label(
            default = Label("@trlc//tools/trlc_rst:trlc_rst"),
            executable = True,
            cfg = "exec",
            doc = "TRLC-to-RST renderer tool.",
        ),
        "_fmea_needs_to_trlc": attr.label(
            default = Label("//bazel/rules/rules_score:fmea_needs_to_trlc"),
            executable = True,
            cfg = "exec",
            doc = "Generator of FailureMode records from comp_saf_fmea needs.",
        ),
    },
    subrules = [subrule_lobster_trlc, subrule_lobster_sphinx_needs, subrule_trlc_image_stage],
)

def score_requirements_rule(
        name,
        srcs,
        req_kind,
        lobster_config,
        deps = [],
        spec = [],
        needs = [],
        needs_satisfied_by = None,
        needs_package = None,
        **kwargs):
    """Macro wrapper around _score_requirements_rule that merges the spec.

    Args:
        srcs: Raw TRLC files, targets producing TRLC files (e.g. rst_to_trlc),
            and/or targets providing TrlcProviderInfo (treated as deps).
        spec: Additional TRLC specification target(s) (single label or list)
            to merge in on top of the S-CORE requirements model, which is
            always included automatically -- do not re-list it here.
        needs: score_docs_as_code ``docs_bundle`` targets or ``needs.json``
            files with Sphinx-Needs requirements.
        needs_satisfied_by: Feature or component ID the converted needs must
            name in their ``satisfied_by`` link.
        needs_package: ``failure_mode`` only: TRLC package of the records
            generated from the needs.
    """
    if srcs == [] and not needs:
        fail("{}: srcs or needs must be given".format(name))
    extra_spec = spec if type(spec) == type([]) else [spec]
    merged_spec = [_DEFAULT_SPEC] + [s for s in extra_spec if s != _DEFAULT_SPEC]

    _score_requirements_rule(
        name = name,
        srcs = srcs,
        deps = deps,
        req_kind = req_kind,
        lobster_config = lobster_config,
        spec = merged_spec,
        needs = needs_json_labels(needs),
        needs_satisfied_by = needs_satisfied_by,
        needs_package = needs_package,
        **kwargs
    )
