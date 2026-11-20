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
Requirement macros reading Sphinx-Needs (``needs.json``) instead of TRLC.

The ``*_spn`` macros are the Sphinx-Needs counterparts of
``feature_requirements``, ``component_requirements``,
``assumed_system_requirements`` and ``assumptions_of_use``. They convert the
needs with ``lobster_sphinx_needs`` and emit the same domain providers as the
TRLC macros, so ``component`` and ``dependable_element`` accept them unchanged.
They do not emit ``TrlcProviderInfo`` and therefore cannot be ``deps`` of TRLC
requirement targets.
"""

load(
    "//bazel/rules/rules_score:providers.bzl",
    "AssumedSystemRequirementsInfo",
    "AssumptionsOfUseInfo",
    "ComponentRequirementsInfo",
    "FeatureRequirementsInfo",
    "SphinxSourcesInfo",
)
load("//bazel/rules/rules_score/private:lobster_sphinx_needs.bzl", "needs_json_labels", "subrule_lobster_sphinx_needs")

_REQ_KINDS = {
    "feature": struct(need_type = "feat_req", title = "Feature Requirements"),
    "component": struct(need_type = "comp_req", title = "Component Requirements"),
    "assumed_system": struct(need_type = "stkh_req", title = "Assumed System Requirements"),
    "aou": struct(need_type = "aou_req", title = "Assumptions of Use"),
}

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _requirements_spn_impl(ctx):
    kind = _REQ_KINDS[ctx.attr.req_kind]
    lobster_file, rst_file = subrule_lobster_sphinx_needs(
        ctx.files.needs,
        kind.need_type,
        ctx.attr.satisfied_by,
        "{}.rst".format(ctx.attr.name),
        kind.title,
    )
    lobster_depset = depset([lobster_file])

    if ctx.attr.req_kind == "feature":
        req_provider = FeatureRequirementsInfo(srcs = lobster_depset, name = ctx.label.name)
    elif ctx.attr.req_kind == "component":
        req_provider = ComponentRequirementsInfo(srcs = lobster_depset, name = ctx.label.name)
    elif ctx.attr.req_kind == "aou":
        req_provider = AssumptionsOfUseInfo(aou_lobster = lobster_depset, name = ctx.label.name)
    else:  # assumed_system
        req_provider = AssumedSystemRequirementsInfo(srcs = lobster_depset, name = ctx.label.name)

    sphinx_srcs = depset([rst_file])
    return [
        DefaultInfo(files = depset([lobster_file, rst_file])),
        req_provider,
        SphinxSourcesInfo(
            srcs = sphinx_srcs,
            deps = sphinx_srcs,
            aux_srcs = depset(),
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_score_requirements_spn_rule = rule(
    implementation = _requirements_spn_impl,
    doc = "Shared internal rule of the *_spn requirement macros (requirements from Sphinx-Needs).",
    attrs = {
        "needs": attr.label_list(
            allow_files = [".json"],
            mandatory = True,
            doc = "Sphinx-Needs needs.json files whose needs of this kind's need type are converted.",
        ),
        "req_kind": attr.string(
            values = _REQ_KINDS.keys(),
            mandatory = True,
            doc = "Kind of requirements; determines the need type and the emitted domain provider.",
        ),
        "satisfied_by": attr.string(
            doc = "Only convert needs whose satisfied_by link names this feature or component ID.",
        ),
    },
    subrules = [subrule_lobster_sphinx_needs],
)

# ============================================================================
# Public Macros
# ============================================================================

def feature_requirements_spn(name, needs, feat_id, **kwargs):
    """Feature requirements from Sphinx-Needs; counterpart of ``feature_requirements``.

    Converts the ``feat_req`` needs whose ``satisfied_by`` link names ``feat_id``
    and provides FeatureRequirementsInfo and SphinxSourcesInfo.

    Args:
        name: The name of the target.
        needs: score_docs_as_code ``docs_bundle`` targets or ``needs.json`` files.
        feat_id: ID of the feature (``feat`` need) the requirements belong to.
        **kwargs: Common Bazel attributes (e.g. visibility, testonly).
    """
    _score_requirements_spn_rule(
        name = name,
        needs = needs_json_labels(needs),
        req_kind = "feature",
        satisfied_by = feat_id,
        **kwargs
    )

def component_requirements_spn(name, needs, comp_id, **kwargs):
    """Component requirements from Sphinx-Needs; counterpart of ``component_requirements``.

    Converts the ``comp_req`` needs whose ``satisfied_by`` link names ``comp_id``
    and provides ComponentRequirementsInfo and SphinxSourcesInfo.

    Args:
        name: The name of the target.
        needs: score_docs_as_code ``docs_bundle`` targets or ``needs.json`` files.
        comp_id: ID of the component (``comp`` need) the requirements belong to.
        **kwargs: Common Bazel attributes (e.g. visibility, testonly).
    """
    _score_requirements_spn_rule(
        name = name,
        needs = needs_json_labels(needs),
        req_kind = "component",
        satisfied_by = comp_id,
        **kwargs
    )

def assumed_system_requirements_spn(name, needs, **kwargs):
    """Assumed system requirements from Sphinx-Needs (``stkh_req`` needs).

    Counterpart of ``assumed_system_requirements``; provides
    AssumedSystemRequirementsInfo and SphinxSourcesInfo.

    Args:
        name: The name of the target.
        needs: score_docs_as_code ``docs_bundle`` targets or ``needs.json`` files.
        **kwargs: Common Bazel attributes (e.g. visibility, testonly).
    """
    _score_requirements_spn_rule(
        name = name,
        needs = needs_json_labels(needs),
        req_kind = "assumed_system",
        **kwargs
    )

def assumptions_of_use_spn(name, needs, **kwargs):
    """Assumptions of use from Sphinx-Needs (``aou_req`` needs).

    Counterpart of ``assumptions_of_use``; provides AssumptionsOfUseInfo and
    SphinxSourcesInfo.

    Args:
        name: The name of the target.
        needs: score_docs_as_code ``docs_bundle`` targets or ``needs.json`` files.
        **kwargs: Common Bazel attributes (e.g. visibility, testonly).
    """
    _score_requirements_spn_rule(
        name = name,
        needs = needs_json_labels(needs),
        req_kind = "aou",
        **kwargs
    )
