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
        ref_package = "",
        visibility = None):
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
            parsing.  These targets must provide TrlcProviderInfo.
        spec: Optional TRLC specification target providing RSL type definitions.
            Defaults to the S-CORE requirements model
            (``@score_tooling//bazel/rules/rules_score/trlc/config:score_requirements_model``).
            Override this when using a custom requirements model.
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
        lobster_config = Label("//bazel/rules/rules_score/lobster/config:component_requirement"),
        spec = spec,
        ref_package = ref_package,
        visibility = visibility,
    )
    trlc_requirements_test(
        name = name + "_test",
        reqs = [":" + name],
        visibility = visibility,
    )
