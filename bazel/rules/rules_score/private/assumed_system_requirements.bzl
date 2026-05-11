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
Assumed System Requirements build rules for S-CORE projects.

Assumed System Requirements (ASR) are system-level requirements that represent
the assumptions a Safety Element out of Context (SEooC) makes about the system
it will be integrated into. Feature requirements are derived from them.
"""

load("@trlc//:trlc.bzl", "trlc_requirements_test")
load("//bazel/rules/rules_score/private:requirements.bzl", "score_requirements_rule")

# ============================================================================
# Public Macro
# ============================================================================

def assumed_system_requirements(
        name,
        srcs,
        deps = [],
        spec = Label("//bazel/rules/rules_score/trlc/config:score_requirements_model"),
        ref_package = "",
        **kwargs):
    """Define Assumed System Requirements following S-CORE process guidelines.

    Creates a target providing AssumedSystemRequirementsInfo, TrlcProviderInfo,
    and SphinxSourcesInfo, plus a validation test target ``<name>_test``.

    Because this target emits TrlcProviderInfo, downstream requirement targets
    (e.g. feature_requirements) can reference it directly in their ``deps``
    without any intermediate trlc_requirements wrapper.

    Args:
        name: The name of the target.
        srcs: List of .trlc source files containing AssumedSystemReq records as
            defined in the S-CORE requirements model.
        deps: Optional list of requirement targets whose TRLC records are needed
            for cross-reference parsing.  These targets must provide
            TrlcProviderInfo.  Typically empty for top-level system requirements.
        spec: Optional TRLC specification target providing RSL type definitions.
            Defaults to the S-CORE requirements model
            (``@score_tooling//bazel/rules/rules_score/trlc/config:score_requirements_model``).
            Override this when using a custom requirements model.
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>:      Main target providing AssumedSystemRequirementsInfo,
                     TrlcProviderInfo, and SphinxSourcesInfo.
        <name>_test: TRLC validation test (runs ``trlc --verify``).

    Example:
        ```starlark
        assumed_system_requirements(
            name = "asr",
            srcs = ["assumed_system_requirements.trlc"],
        )

        feature_requirements(
            name = "feat_req",
            srcs = ["feature_requirements.trlc"],
            deps = [":asr"],
        )
        ```
    """
    score_requirements_rule(
        name = name,
        srcs = srcs,
        deps = deps,
        req_kind = "assumed_system",
        lobster_config = Label("//bazel/rules/rules_score/lobster/config:assumed_system_requirement"),
        spec = spec,
        ref_package = ref_package,
        **kwargs
    )
    trlc_requirements_test(
        name = name + "_test",
        reqs = [":" + name],
        **kwargs
    )
