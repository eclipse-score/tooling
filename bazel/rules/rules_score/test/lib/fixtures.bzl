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
"""Minimal rules_score fixtures for L2 analysis tests.

All fixtures are `manual` helper targets created with `util.helper_target`, so
they are only analysed through the test that uses them. They deliberately are
not release-conformant beyond what the tested contract needs.
"""

load("@rules_testing//lib:util.bzl", "util")
load(
    "@score_tooling//bazel/rules/rules_score:rules_score.bzl",
    "architectural_design",
    "assumed_system_requirements",
    "component",
    "dependable_element",
    "unit",
    "unit_design",
)

_ASR_SRC = Label("//lib:fixtures/asr.trlc")
_COVERAGE_LOCK = Label("//lib:fixtures/test_case_coverage.lock.yaml")
_DESIGN_SRC = Label("//lib:fixtures/design.puml")

def fixture_architectural_design(name):
    """architectural_design with a single static component diagram."""
    util.helper_target(
        architectural_design,
        name = name,
        static = [_DESIGN_SRC],
    )
    return ":" + name

def fixture_asr(name):
    """Minimal assumed_system_requirements with a single TRLC file."""
    util.helper_target(
        assumed_system_requirements,
        name = name,
        srcs = [_ASR_SRC],
    )
    return ":" + name

def fixture_unit(name):
    """Empty unit with an empty unit_design."""
    util.helper_target(unit_design, name = name + "_design")
    util.helper_target(
        unit,
        name = name,
        unit_design = [":" + name + "_design"],
        implementation = [],
        tests = [],
    )
    return ":" + name

def fixture_component(name, components = [], with_coverage_lock = False):
    """Component of `components`, optionally with a test case coverage lock."""
    util.helper_target(
        component,
        name = name,
        components = components,
        tests = [],
        test_case_coverage_lock = _COVERAGE_LOCK if with_coverage_lock else None,
    )
    return ":" + name

def fixture_dependable_element(
        name,
        maturity = "release",
        integrity_level = "B",
        components = [],
        requirements = [],
        designs = [],
        deps = [],
        **kwargs):
    """Dependable element; `designs` are architectural_design labels, other inputs are empty.

    Returns the label of the `<name>_index` target, which owns the actions.
    """
    util.helper_target(
        dependable_element,
        name = name,
        architectural_design = designs,
        dependability_analysis = [],
        components = components,
        requirements = requirements,
        tests = [],
        deps = deps,
        integrity_level = integrity_level,
        maturity = maturity,
        **kwargs
    )
    return ":" + name + "_index"
