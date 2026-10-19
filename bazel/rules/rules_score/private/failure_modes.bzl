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
Failure Modes build rule for S-CORE projects.

Holds the ``ScoreReq.FailureMode`` records of a safety analysis. The fault-tree
diagrams reference them as the top events of their trees (``fault_trees`` lists
the target in ``deps``); ``safety_analysis`` renders and traces them.
"""

load("@trlc//:trlc.bzl", "trlc_requirements_test")
load("//bazel/rules/rules_score/private:requirements.bzl", "score_requirements_rule")

def failure_modes(
        name,
        srcs,
        deps = [],
        lobster_config = Label("//bazel/rules/rules_score/lobster/config:failuremodes_config"),
        **kwargs):
    """Define the failure modes of a safety analysis.

    Creates a target providing FailureModesInfo, TrlcProviderInfo and
    SphinxSourcesInfo, plus a validation test target ``<name>_test``.

    Args:
        name: The name of the target.
        srcs: List of ``.trlc`` files containing ``ScoreReq.FailureMode`` records.
        deps: Optional targets (providing TrlcProviderInfo) needed to parse the
            records.
        lobster_config: Lobster YAML configuration for FailureMode traceability
            extraction. Defaults to the standard S-CORE failure mode config.
        **kwargs: Additional arguments (e.g. ``visibility``, ``tags``).

    Generated Targets:
        <name>:      Main target providing FailureModesInfo and TrlcProviderInfo.
        <name>_test: TRLC validation test (runs ``trlc --verify``).

    Example:
        ```starlark
        failure_modes(
            name = "my_failure_modes",
            srcs = ["failure_modes.trlc"],
        )
        ```
    """
    score_requirements_rule(
        name = name,
        srcs = srcs,
        deps = deps,
        req_kind = "failure_mode",
        lobster_config = lobster_config,
        **kwargs
    )
    trlc_requirements_test(
        name = name + "_test",
        reqs = [":" + name],
        **kwargs
    )
