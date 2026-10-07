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
Test Specification build rules for S-CORE projects.

Test specifications describe verification test cases (preconditions, test
steps, expected results) that trace to feature requirements via the
TestSpec.verifies field, enabling LOBSTER traceability from requirements
down to test design.
"""

load("@trlc//:trlc.bzl", "trlc_requirements_test")
load("//bazel/rules/rules_score/private:requirements.bzl", "score_requirements_rule")

# ============================================================================
# Public Macro
# ============================================================================

def test_specifications(
        name,
        srcs,
        lobster_config,
        deps = [],
        spec = Label("//bazel/rules/rules_score/trlc/config:score_requirements_model"),
        ref_package = "",
        image_srcs = [],
        **kwargs):
    """Define test specifications following S-CORE process guidelines.

    Creates a target providing TestSpecificationsInfo, TrlcProviderInfo, and
    SphinxSourcesInfo, plus a validation test target ``<name>_test``.

    Because this target emits TrlcProviderInfo, it can reference
    feature_requirements targets directly in its ``deps`` without any
    intermediate trlc_requirements wrapper.

    Args:
        name: The name of the target.
        srcs: List of .trlc source files containing TestSpec records as
            defined in the requirements model in use (the base S-CORE
            requirements model does not define TestSpec; it is provided by
            extension models such as SweAsCodeReq).
        lobster_config: Lobster extraction config label. There is no default
            because the TestSpec record type is not part of the base S-CORE
            requirements model (e.g. use
            ``@swe_as_code//tools/trlc/config:lobster_test_specification``).
        deps: Optional list of requirement targets (e.g. feature_requirements)
            whose TRLC records are needed for cross-reference parsing since
            TestSpec.verifies references FeatReqId. These targets must
            provide TrlcProviderInfo.
        spec: Optional TRLC specification target providing RSL type definitions.
            Defaults to the S-CORE requirements model
            (``@score_tooling//bazel/rules/rules_score/trlc/config:score_requirements_model``).
            Override this when using a custom requirements model that defines TestSpec.
        ref_package: TRLC package prefix used for verifies cross-references
            when converting RST sources.
        image_srcs: Image and diagram files (.svg, .png, or .puml) to stage
            next to the rendered RST.
        visibility: Bazel visibility specification for the generated targets.

    Generated Targets:
        <name>:      Main target providing TestSpecificationsInfo, TrlcProviderInfo,
                     and SphinxSourcesInfo.
        <name>_test: TRLC validation test (runs ``trlc --verify``).

    Example:
        ```starlark
        feature_requirements(
            name = "feat_req",
            srcs = ["feature_requirements.trlc"],
        )

        test_specifications(
            name = "test_spec",
            srcs = ["test_specifications.trlc"],
            deps = [":feat_req"],
            lobster_config = "@swe_as_code//tools/trlc/config:lobster_test_specification",
        )
        ```
    """
    score_requirements_rule(
        name = name,
        srcs = srcs,
        deps = deps,
        req_kind = "test_spec",
        lobster_config = lobster_config,
        spec = spec,
        ref_package = ref_package,
        image_srcs = image_srcs,
        **kwargs
    )
    trlc_requirements_test(
        name = name + "_test",
        reqs = [":" + name],
        **kwargs
    )
