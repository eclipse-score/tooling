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
Assumptions of Use build rules for S-CORE projects.

This module provides a macro for defining Assumptions of Use (AoU) following
S-CORE process guidelines. Assumptions of Use define the safety-relevant
operating conditions and constraints for a Safety Element out of Context (SEooC).

Implemented as the "aou" kind of the shared score_requirements_rule (see
requirements.bzl), so raw .trlc files, .rst files, and existing
TrlcProviderInfo-emitting targets (e.g. trlc_requirements) may all be passed
as srcs, matching the conventions of feature_requirements,
component_requirements, and assumed_system_requirements.

Traceability to feature/assumed-system requirements is established at the
dependable_element level (via its own `requirements` attribute), not here.
"""

load("@trlc//:trlc.bzl", "trlc_requirements_test")
load("//bazel/rules/rules_score/private:requirements.bzl", "score_requirements_rule")

# ============================================================================
# Public Macro
# ============================================================================

def assumptions_of_use(
        name,
        srcs,
        deps = [],
        ref_package = None,
        lobster_config = Label("//bazel/rules/rules_score/lobster/config:aou_config"),
        **kwargs):
    """Define Assumptions of Use following S-CORE process guidelines.

    Assumptions of Use (AoU) define the safety-relevant operating conditions
    and constraints for a Safety Element out of Context (SEooC). They specify
    the conditions under which the component is expected to operate safely
    and the responsibilities of the integrator.

    Args:
        name: The name of the assumptions of use target. Used as the base
            name for all generated targets.
        srcs: List of TRLC/RST/label sources defining the Assumptions of Use.
            Accepts raw ``.trlc`` files, ``.rst`` files containing ``aou_req``
            directives (converted to TRLC automatically), and/or labels to
            existing targets that already provide TrlcProviderInfo (e.g.
            ``trlc_requirements`` targets).
        deps: Optional list of other requirement targets (providing
            TrlcProviderInfo) needed for cross-reference parsing.
        ref_package: Optional TRLC package prefix used for ``derived_from``
            cross-references when converting RST sources.
        lobster_config: Lobster YAML configuration for AoU traceability
            extraction. Defaults to the standard S-CORE AoU config.
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>: Main assumptions of use target providing AssumptionsOfUseInfo
        <name>_test: TRLC validation test for the assumptions of use sources

    Example using raw TRLC sources directly:
        ```starlark
        assumptions_of_use(
            name = "my_assumptions_of_use",
            srcs = ["assumptions_of_use.trlc"],
        )
        ```

    Example using RST sources directly:
        ```starlark
        assumptions_of_use(
            name = "my_assumptions_of_use",
            srcs = ["docs/assumptions_of_use.rst"],
        )
        ```
    """
    score_requirements_rule(
        name = name,
        srcs = srcs,
        deps = deps,
        req_kind = "aou",
        lobster_config = lobster_config,
        ref_package = ref_package or "",
        **kwargs
    )
    trlc_requirements_test(
        name = name + "_test",
        reqs = [":" + name],
        **kwargs
    )
