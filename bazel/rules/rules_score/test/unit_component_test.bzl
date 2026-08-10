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
Test suite for unit, component, and dependable_element rules.

Tests the new hierarchical structure for S-CORE process compliance:
- Unit: smallest testable element
- Component: collection of units
- Dependable Element: complete SEooC with full documentation
"""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@score_tooling//bazel/rules/rules_score:providers.bzl", "ComponentInfo", "SphinxSourcesInfo", "UnitInfo")

# ============================================================================
# Unit Tests
# ============================================================================

def _unit_provider_test_impl(ctx):
    """Test that unit rule provides UnitInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check UnitInfo provider exists
    asserts.true(
        env,
        UnitInfo in target_under_test,
        "Unit should provide UnitInfo",
    )

    unit_info = target_under_test[UnitInfo]

    # Verify fields are populated
    asserts.true(
        env,
        unit_info.name != None,
        "UnitInfo should have name field",
    )

    asserts.true(
        env,
        unit_info.unit_design != None,
        "UnitInfo should have unit_design field",
    )

    asserts.true(
        env,
        unit_info.implementation != None,
        "UnitInfo should have implementation field",
    )

    asserts.true(
        env,
        unit_info.tests != None,
        "UnitInfo should have tests field",
    )

    return analysistest.end(env)

unit_provider_test = analysistest.make(_unit_provider_test_impl)

def _unit_sphinx_sources_test_impl(ctx):
    """Test that unit rule provides SphinxSourcesInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check SphinxSourcesInfo provider exists
    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "Unit should provide SphinxSourcesInfo",
    )

    return analysistest.end(env)

unit_sphinx_sources_test = analysistest.make(_unit_sphinx_sources_test_impl)

# ============================================================================
# Component Tests
# ============================================================================

def _component_provider_test_impl(ctx):
    """Test that component rule provides ComponentInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check ComponentInfo provider exists
    asserts.true(
        env,
        ComponentInfo in target_under_test,
        "Component should provide ComponentInfo",
    )

    comp_info = target_under_test[ComponentInfo]

    # Verify fields are populated
    asserts.true(
        env,
        comp_info.name != None,
        "ComponentInfo should have name field",
    )

    asserts.true(
        env,
        comp_info.requirements != None,
        "ComponentInfo should have component_requirements field",
    )

    asserts.true(
        env,
        comp_info.components != None,
        "ComponentInfo should have components field",
    )

    asserts.true(
        env,
        comp_info.tests != None,
        "ComponentInfo should have tests field",
    )

    return analysistest.end(env)

component_provider_test = analysistest.make(_component_provider_test_impl)

def _component_requirements_transitive_test_impl(ctx):
    """Test that ComponentInfo.requirements_transitive rolls up nested components.

    :test_parent_component nests :test_nested_component, and the two use
    DIFFERENT component_requirements targets (:comp_req vs. :comp_req_rst,
    see test/BUILD). Both must be present in the parent's
    requirements_transitive, or the nested component's own requirements
    silently stopped being rolled up (and thus stopped being covered by any
    CI-visible traceability check at the dependable_element level).
    """
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    comp_info = target_under_test[ComponentInfo]
    basenames = [f.basename for f in comp_info.requirements_transitive.to_list()]

    asserts.true(
        env,
        "comp_req.lobster" in basenames,
        "Parent component's own requirements (:comp_req) missing from " +
        "requirements_transitive; got: %s" % basenames,
    )
    asserts.true(
        env,
        "comp_req_rst.lobster" in basenames,
        "Nested component's requirements (:comp_req_rst) did not roll up into " +
        "the parent's requirements_transitive; got: %s" % basenames,
    )

    return analysistest.end(env)

component_requirements_transitive_test = analysistest.make(_component_requirements_transitive_test_impl)

def _component_sphinx_sources_test_impl(ctx):
    """Test that component rule provides SphinxSourcesInfo."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Check SphinxSourcesInfo provider exists
    asserts.true(
        env,
        SphinxSourcesInfo in target_under_test,
        "Component should provide SphinxSourcesInfo",
    )

    return analysistest.end(env)

component_sphinx_sources_test = analysistest.make(_component_sphinx_sources_test_impl)

def _component_excludes_feature_req_docs_test_impl(ctx):
    """Test that a feature_requirements target listed in `requirements`
    alongside a component_requirements target is not re-rendered into the
    component's own Sphinx docs (SphinxSourcesInfo.srcs), avoiding duplicate
    feature requirements pages across every component. It is still expected
    in ComponentInfo/tests via feature_requirements' own mechanisms elsewhere
    (e.g. the dependable_element level)."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    sphinx_srcs = target_under_test[SphinxSourcesInfo].srcs.to_list()
    basenames = [f.basename for f in sphinx_srcs]

    asserts.true(
        env,
        "feat_req.rst" not in basenames,
        "Component SphinxSourcesInfo.srcs should not include the rendered " +
        "feature_requirements RST (found: %s)" % basenames,
    )
    asserts.true(
        env,
        "comp_req.rst" in basenames,
        "Component SphinxSourcesInfo.srcs should include the rendered " +
        "component_requirements RST (found: %s)" % basenames,
    )

    return analysistest.end(env)

component_excludes_feature_req_docs_test = analysistest.make(_component_excludes_feature_req_docs_test_impl)

# ============================================================================
# Test Case Coverage Lock Tests
# ============================================================================

def _component_test_case_coverage_lock_test_impl(ctx):
    """Test that a component with test_case_coverage_lock still provides ComponentInfo and
    DefaultInfo with output files (analysis phase must succeed)."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        ComponentInfo in target_under_test,
        "Component with test_case_coverage_lock should provide ComponentInfo",
    )

    # DefaultInfo.files must contain the lobster report (non-empty).
    output_files = target_under_test[DefaultInfo].files.to_list()
    asserts.true(
        env,
        len(output_files) > 0,
        "Component with test_case_coverage_lock should declare output files (expected lobster report); got: %s" % output_files,
    )

    # At least one output file should be named like the lobster report.
    lobster_report_present = any([f.basename.endswith(".lobster_report") or f.extension == "html" or f.basename.endswith(".lobster") for f in output_files])
    asserts.true(
        env,
        lobster_report_present,
        "Component with test_case_coverage_lock should declare a lobster report output; got: %s" % [f.basename for f in output_files],
    )

    return analysistest.end(env)

component_test_case_coverage_lock_test = analysistest.make(_component_test_case_coverage_lock_test_impl)

def _dependable_element_test_case_coverage_lock_check_action_test_impl(ctx):
    """
    Given a dependable_element wrapping a component with test_case_coverage_lock set,
    When the target is analyzed,
    Then a ComponentTestCaseCoverageLockCheck action must be registered that consumes
         the committed test_case_coverage.lock.yaml as an input, and it must carry
         ``--allow-check-failures`` in its argv only when the
         dependable_element's maturity is "development" (never for "release").

    This exercises the actual component -> ComponentTestCaseCoverageInfo ->
    dependable_element wiring (unlike component_test_case_coverage_lock_test, which only
    inspects the component() rule in isolation).
    """
    env = analysistest.begin(ctx)

    actions = analysistest.target_actions(env)
    check_action = None
    for action in actions:
        if action.mnemonic == "ComponentTestCaseCoverageLockCheck":
            check_action = action
            break

    asserts.true(
        env,
        check_action != None,
        "Expected a ComponentTestCaseCoverageLockCheck action to be registered for a " +
        "dependable_element wrapping a component with test_case_coverage_lock set",
    )

    if check_action == None:
        return analysistest.end(env)

    input_names = [f.basename for f in check_action.inputs.to_list()]
    asserts.true(
        env,
        "test_case_coverage.lock.yaml" in input_names,
        "ComponentTestCaseCoverageLockCheck action must take the committed " +
        "test_case_coverage.lock.yaml as an input; got inputs: %s" % input_names,
    )

    has_allow_flag = "--allow-check-failures" in check_action.argv
    asserts.equals(
        env,
        ctx.attr.expect_allow_check_failures,
        has_allow_flag,
        "expected --allow-check-failures presence=%s (maturity=%s) but argv was: %s" % (
            ctx.attr.expect_allow_check_failures,
            ctx.attr.maturity_under_test,
            check_action.argv,
        ),
    )

    return analysistest.end(env)

dependable_element_test_case_coverage_lock_check_action_test = analysistest.make(
    impl = _dependable_element_test_case_coverage_lock_check_action_test_impl,
    attrs = {
        "expect_allow_check_failures": attr.bool(
            mandatory = True,
            doc = "Whether --allow-check-failures is expected in the check action's argv.",
        ),
        "maturity_under_test": attr.string(
            doc = "The maturity value of the target under test (for assertion messages only).",
        ),
    },
)

# ============================================================================
# Dependable Element Tests
# ============================================================================
# Note: Provider tests removed as dependable_element no longer creates a
# separate provider target. The main target is now a sphinx_module.

# ============================================================================
# Test Suite Definition
# ============================================================================

def unit_component_test_suite(name):
    """Create test suite for unit, component, and dependable_element rules.

    Args:
        name: Name of the test suite
    """
    native.test_suite(
        name = name,
        tests = [
            ":unit_provider_test",
            ":unit_sphinx_sources_test",
            ":component_provider_test",
            ":component_sphinx_sources_test",
            ":component_excludes_feature_req_docs_test",
            ":component_test_case_coverage_lock_test",
            ":component_requirements_transitive_test",
            ":test_case_coverage_lock_check_action_release_test",
            ":test_case_coverage_lock_check_action_development_test",
        ],
    )
