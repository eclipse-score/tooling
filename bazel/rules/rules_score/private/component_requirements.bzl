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
Component Requirements build rules for S-CORE projects.

Component requirements are derived from feature requirements and define the
specific requirements for a software component.
"""

load("@trlc//:trlc.bzl", "trlc_requirements_test")
load("//bazel/rules/rules_score/private:requirements.bzl", "score_requirements_rule")

# ============================================================================
# Public Macro
# ============================================================================

def component_requirements(
        name,
        srcs,
        deps = [],
        spec = Label("//bazel/rules/rules_score/trlc/config:score_requirements_model"),
        lobster_config = Label("//bazel/rules/rules_score/lobster/config:component_requirement"),
        ref_package = "",
        package = "",
        image_srcs = [],
        **kwargs):
    """Define component requirements following S-CORE process guidelines.

    Creates a target providing ComponentRequirementsInfo, TrlcProviderInfo,
    and SphinxSourcesInfo, plus a validation test target ``<name>_test``.

    Because this target emits TrlcProviderInfo, downstream targets can
    reference it directly in their ``deps`` without any intermediate
    trlc_requirements wrapper.

    Args:
        name: The name of the target.
        srcs: List of .trlc source files containing CompReq records as defined
            in the S-CORE requirements model.
        deps: Optional list of requirement targets (e.g. assumed_system_requirements,
            feature_requirements) whose TRLC records are needed for cross-reference
            parsing. Also list any assumptions_of_use target(s) here (this
            element's own, or one received/forwarded from a dependable_element
            dependency) to resolve AoU references in `derived_from`
            (CompReqSourceId). These targets must provide TrlcProviderInfo.
        spec: TRLC specification target(s) providing RSL type definitions.
            Accepts a single label or a list of labels; all are merged into the
            spec passed to TRLC.  Defaults to the S-CORE requirements model
            (``@score_tooling//bazel/rules/rules_score/trlc/config:score_requirements_model``).
        lobster_config: Optional Lobster extraction config label. Defaults to the
            S-CORE component requirement config. Its conversion rules always
            include `derived_from` as a tracing target, including any AoU
            entries within it (resolved only at the dependable_element level,
            which has a "Received AoUs" level).
        package: Optional TRLC package name override, only used when srcs
            contains .rst files (ignored for raw .trlc srcs). Defaults to a
            name derived from the .rst file's stem; set this explicitly to
            avoid collisions when multiple requirement targets are converted
            from same-named .rst files (e.g. multiple "index.rst").
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>:      Main target providing ComponentRequirementsInfo, TrlcProviderInfo,
                     and SphinxSourcesInfo.
        <name>_test: TRLC validation test (runs ``trlc --verify``).

    Example:
        ```starlark
        component_requirements(
            name = "comp_req",
            srcs = ["component_requirements.trlc"],
            deps = [":asr", ":feat_req"],
        )
        ```
    """
    score_requirements_rule(
        name = name,
        srcs = srcs,
        deps = deps,
        req_kind = "component",
        lobster_config = lobster_config,
        spec = spec,
        ref_package = ref_package,
        package = package,
        image_srcs = image_srcs,
        **kwargs
    )
    trlc_requirements_test(
        name = name + "_test",
        reqs = [":" + name],
        **kwargs
    )
