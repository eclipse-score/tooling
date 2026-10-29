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
"""Tests for the test_specification rule and its wiring into unit/component/dependable_element."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@score_tooling//bazel/rules/rules_score:providers.bzl", "TestSpecificationInfo", "UnitInfo")

def _flag_values(argv, flag):
    return [argv[i + 1] for i in range(len(argv) - 1) if argv[i] == flag]

def _test_specification_action_test_impl(ctx):
    """
    Given a test_specification target in the root package,
    When it is analyzed,
    Then one TestSpecificationCheck action validates the RST, the target label
         and the RST file label are passed as references, and the JSON output
         is part of the _validation output group.
    """
    env = analysistest.begin(ctx)
    target = analysistest.target_under_test(env)
    actions = [a for a in analysistest.target_actions(env) if a.mnemonic == "TestSpecificationCheck"]

    asserts.equals(env, 1, len(actions))
    if actions:
        argv = actions[0].argv
        asserts.equals(env, ["//:test_spec_sample"], _flag_values(argv, "--ref"))
        asserts.equals(env, ["//:fixtures/test_specification/sample_spec.rst"], _flag_values(argv, "--alias"))

    asserts.true(env, TestSpecificationInfo in target, "test_specification must provide TestSpecificationInfo")
    spec_json = target[TestSpecificationInfo].spec_json
    asserts.equals(env, "test_spec_sample.test_spec.json", spec_json.basename)
    asserts.true(
        env,
        spec_json in target[OutputGroupInfo]._validation.to_list(),
        "the spec JSON must be a validation output so that building a dependent validates the RST",
    )
    return analysistest.end(env)

test_specification_action_test = analysistest.make(_test_specification_action_test_impl)

def _unit_test_specs_test_impl(ctx):
    """
    Given a unit with the package-relative test_specs entry ":test_spec_sample",
    When it is analyzed,
    Then UnitInfo.test_specs carries the JSON of the test_specification together
         with the reference spelled as an absolute label of the unit's repository.
    """
    env = analysistest.begin(ctx)
    target = analysistest.target_under_test(env)
    specs = target[UnitInfo].test_specs.to_list()
    asserts.equals(env, ["test_spec_sample.test_spec.json"], [s.spec_json.basename for s in specs])
    asserts.equals(env, ["//:test_spec_sample"], [s.reference for s in specs])
    return analysistest.end(env)

unit_test_specs_test = analysistest.make(_unit_test_specs_test_impl)

def _coverage_check_consumes_test_specs_test_impl(ctx):
    """
    Given a dependable_element whose component contains a unit with test_specs,
    When it is analyzed,
    Then the ComponentTestCaseCoverageLockCheck action takes the spec JSON as an
         input and receives a spec manifest via TEST_CASE_COVERAGE_SPEC_MANIFEST.
    """
    env = analysistest.begin(ctx)
    actions = [a for a in analysistest.target_actions(env) if a.mnemonic == "ComponentTestCaseCoverageLockCheck"]
    asserts.equals(env, 1, len(actions))
    if actions:
        inputs = [f.basename for f in actions[0].inputs.to_list()]
        asserts.true(env, "test_spec_sample.test_spec.json" in inputs, "spec JSON missing in inputs: %s" % inputs)
        manifest = actions[0].env.get("TEST_CASE_COVERAGE_SPEC_MANIFEST", "")
        asserts.true(env, manifest.endswith(".txt"), "TEST_CASE_COVERAGE_SPEC_MANIFEST not set: %s" % actions[0].env)
    return analysistest.end(env)

coverage_check_consumes_test_specs_test = analysistest.make(_coverage_check_consumes_test_specs_test_impl)

def test_specification_test_suite(name):
    native.test_suite(
        name = name,
        tests = [
            ":test_specification_action_test",
            ":unit_test_specs_test",
            ":coverage_check_consumes_test_specs_test",
        ],
    )
