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
Component build rules for S-CORE projects.

This module provides macros and rules for defining software components
following S-CORE process guidelines. A component consists of multiple units
with associated requirements and tests.
"""

load("//bazel/rules/rules_score:providers.bzl", "ComponentInfo", "SphinxSourcesInfo")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _component_impl(ctx):
    """Implementation for component rule.

    Collects component requirements, units, and tests and provides them
    through the ComponentInfo provider.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo and ComponentInfo
    """

    # Collect requirements files from component_requirements targets
    requirements_files = []
    for req_target in ctx.attr.requirements:
        if SphinxSourcesInfo in req_target:
            requirements_files.append(req_target[SphinxSourcesInfo].srcs)

    requirements_depset = depset(transitive = requirements_files)

    # Collect components and tests
    components_depset = depset(ctx.attr.components)
    tests_depset = depset(ctx.attr.tests)

    # Combine all files for DefaultInfo
    all_files = depset(
        transitive = [requirements_depset],
    )

    return [
        DefaultInfo(files = all_files),
        ComponentInfo(
            name = ctx.label.name,
            requirements = requirements_depset,
            components = components_depset,
            tests = tests_depset,
        ),
        SphinxSourcesInfo(
            srcs = all_files,
            transitive_srcs = all_files,
        ),
    ]

# ============================================================================
# Rule Definition
# ============================================================================

_component = rule(
    implementation = _component_impl,
    doc = "Defines a software component composed of multiple units for S-CORE process compliance",
    attrs = {
        "requirements": attr.label_list(
            mandatory = True,
            doc = "Component requirements artifacts (typically component_requirements targets)",
        ),
        "components": attr.label_list(
            mandatory = True,
            doc = "Unit targets that comprise this component",
        ),
        "tests": attr.label_list(
            mandatory = True,
            doc = "Component-level integration test targets",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def component(
        name,
        units = None,
        tests = [],
        requirements = None,
        components = None,
        testonly = True,
        visibility = None):
    """Define a software component following S-CORE process guidelines.

    A component is a collection of related units that together provide
    a specific functionality. It consists of:
    - Component requirements: Requirements specification for the component
    - Implementation: Concrete libraries/binaries that realize the component
    - Units: Individual software units that implement the requirements
    - Tests: Integration tests that verify the component as a whole

    Args:
        name: The name of the component. Used as the target name.
        component_requirements: List of labels to component_requirements targets
            that define the requirements for this component.
        requirements: Alias for component_requirements (use one or the other).
        implementation: List of labels to implementation targets (cc_library,
            cc_binary, etc.) that realize this component.
        units: List of labels to unit targets that comprise this component.
        components: Alias for units (use one or the other).
        tests: List of labels to Bazel test targets that verify the component
            integration.
        testonly: If true, only testonly targets can depend on this component.
        visibility: Bazel visibility specification for the component target.

    Example:
        ```python
        component(
            name = "kvs_component",
            requirements = [":kvs_component_requirements"],
            implementation = [":kvs_lib", ":kvs_tool"],
            units = [":kvs_unit1", ":kvs_unit2"],
            tests = ["//persistency/kvs/tests:score_kvs_component_integration_tests"],
            visibility = ["//visibility:public"],
        )
        ```
    """

    _component(
        name = name,
        requirements = requirements,
        components = components,
        tests = tests,
        testonly = testonly,
        visibility = visibility,
    )
