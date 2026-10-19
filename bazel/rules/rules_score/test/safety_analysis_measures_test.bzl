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
"""Analysis tests for the fault_trees / safety_analysis rules and their measures."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@score_tooling//bazel/rules/rules_score:providers.bzl", "FaultTreesInfo")

def _action_with_output(env, basename):
    for action in analysistest.target_actions(env):
        for out in action.outputs.to_list():
            if out.basename == basename:
                return action
    return None

def _flag_values(argv, flag):
    """Values following *flag* up to the next ``--`` option."""
    if flag not in argv:
        return []
    values = []
    for arg in argv[argv.index(flag) + 1:]:
        if arg.startswith("--"):
            break
        values.append(arg)
    return values

def _basenames(paths):
    return [p.split("/")[-1] for p in paths]

def _measure_deps_are_context_only_test_impl(ctx):
    env = analysistest.begin(ctx)

    # A component requirement measure derives from an AoU held in its deps.
    # That AoU is parsed for reference resolution only: it is not rendered and
    # lobster-trlc, which only extracts Mitigation / AoU records, never sees it.
    assembler = _action_with_output(env, "safety_analysis.rst")
    if assembler == None:
        asserts.true(env, False, "assembler action not found")
        return analysistest.end(env)
    argv = assembler.argv
    asserts.equals(
        env,
        ["comp_req.trlc"],
        _basenames(_flag_values(argv, "--safetymeasures")),
    )
    asserts.true(env, "aou.trlc" in _basenames(_flag_values(argv, "--dep-files")))
    asserts.equals(env, None, _action_with_output(env, "safetymeasures.lobster"))
    return analysistest.end(env)

measure_deps_are_context_only_test = analysistest.make(_measure_deps_are_context_only_test_impl)

def _measure_spec_is_merged_test_impl(ctx):
    env = analysistest.begin(ctx)

    # The RSL types of a measure target are needed to parse its records.
    assembler = _action_with_output(env, "safety_analysis.rst")
    if assembler == None:
        asserts.true(env, False, "assembler action not found")
        return analysistest.end(env)
    asserts.true(env, "extra_model.rsl" in _basenames(_flag_values(assembler.argv, "--dep-files")))
    return analysistest.end(env)

measure_spec_is_merged_test = analysistest.make(_measure_spec_is_merged_test_impl)

def _fta_default_package_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.equals(
        env,
        ctx.attr.expected_package,
        analysistest.target_under_test(env)[FaultTreesInfo].fta_package,
    )
    return analysistest.end(env)

fta_default_package_test = analysistest.make(
    _fta_default_package_test_impl,
    attrs = {"expected_package": attr.string(mandatory = True)},
)

def _aou_in_aou_deps_fails_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(env, "deps must not hold the assumptions_of_use target")
    return analysistest.end(env)

aou_in_aou_deps_fails_test = analysistest.make(
    _aou_in_aou_deps_fails_test_impl,
    expect_failure = True,
)
