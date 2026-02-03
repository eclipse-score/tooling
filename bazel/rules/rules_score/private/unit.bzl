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
Unit build rules for S-CORE projects.

This module provides macros and rules for defining software units
following S-CORE process guidelines. A unit is the smallest testable
software element with associated design, implementation, and tests.
"""

load("//bazel/rules/rules_score:providers.bzl", "SphinxSourcesInfo", "UnitInfo")

# ============================================================================
# Private Rule Implementation
# ============================================================================

def _unit_impl(ctx):
    """Implementation for unit rule.

    Collects unit design artifacts, implementation targets, and tests
    and provides them through the UnitInfo provider.

    Args:
        ctx: Rule context

    Returns:
        List of providers including DefaultInfo and UnitInfo
    """

    # Collect design files from unit_design targets
    design_files = []
    for design_target in ctx.attr.unit_design:
        if SphinxSourcesInfo in design_target:
            design_files.append(design_target[SphinxSourcesInfo].srcs)

    design_depset = depset(transitive = design_files)

    # Collect implementation and test targets
    # Include scope targets in the implementation depset
    implementation_depset = depset(ctx.attr.implementation + ctx.attr.scope)
    tests_depset = depset(ctx.attr.tests)

    # Combine all files for DefaultInfo
    all_files = depset(
        transitive = [design_depset],
    )

    return [
        DefaultInfo(files = all_files),
        UnitInfo(
            name = ctx.label.name,
            unit_design = design_depset,
            implementation = implementation_depset,
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

_unit = rule(
    implementation = _unit_impl,
    doc = "Defines a software unit with design, implementation, and tests for S-CORE process compliance",
    attrs = {
        "unit_design": attr.label_list(
            mandatory = True,
            doc = "Unit design artifacts (typically architectural_design targets)",
        ),
        "implementation": attr.label_list(
            mandatory = True,
            doc = "Implementation targets (cc_library, py_library, rust_library, etc.)",
        ),
        "scope": attr.label_list(
            default = [],
            doc = "Additional not explicitly named targets which are needed for the unit implementation",
        ),
        "tests": attr.label_list(
            mandatory = True,
            doc = "Test targets that verify the unit (cc_test, py_test, rust_test, etc.)",
        ),
    },
)

# ============================================================================
# Public Macro
# ============================================================================

def unit(
        name,
        unit_design,
        implementation,
        tests,
        scope = [],
        testonly = True,
        visibility = None):
    """Define a software unit following S-CORE process guidelines.

    A unit is the smallest testable software element in the S-CORE process.
    It consists of:
    - Unit design: Design documentation and diagrams
    - Implementation: Source code that realizes the design
    - Tests: Test cases that verify the implementation

    Args:
        name: The name of the unit. Used as the target name.
        unit_design: List of labels to architectural_design targets or design
            documentation that describes the unit's internal structure and behavior.
        implementation: List of labels to Bazel targets representing the actual
            implementation (cc_library, py_library, rust_library, etc.).
        scope: Optional list of additional targets needed for the unit implementation
            but not explicitly named in the implementation list. Default is empty list.
        tests: List of labels to Bazel test targets (cc_test, py_test, rust_test, etc.)
            that verify the unit implementation.
        testonly: If true, only testonly targets can depend on this unit. Set to true
            when the unit depends on testonly targets like tests.
        visibility: Bazel visibility specification for the unit target.

    Example:
        ```python
        unit(
            name = "kvs_unit1",
            unit_design = [":kvs_architectural_design"],
            implementation = [
                "//persistency/kvs:lib1",
                "//persistency/kvs:lib2",
                "//persistency/kvs:lib3",
            ],
            tests = ["//persistency/kvs/tests:score_kvs_component_tests"],
            visibility = ["//visibility:public"],
        )
        ```
    """
    _unit(
        name = name,
        unit_design = unit_design,
        implementation = implementation,
        scope = scope,
        tests = tests,
        testonly = testonly,
        visibility = visibility,
    )
