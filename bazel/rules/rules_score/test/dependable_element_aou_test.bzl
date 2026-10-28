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
"""Analysis tests for the AoUs and root cause assessments a dependable_element derives from its safety analyses."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@score_tooling//bazel/rules/rules_score:providers.bzl", "ForwardedAoUInfo")

def _staged_aou_paths(target):
    return [
        f.short_path
        for f in target[DefaultInfo].files.to_list()
        if "/assumptions_of_use/" in f.short_path
    ]

def _aou_derived_test_impl(ctx):
    env = analysistest.begin(ctx)
    target = analysistest.target_under_test(env)

    asserts.equals(
        env,
        1,
        len(target[ForwardedAoUInfo].own_aou_lobster.to_list()),
    )
    staged = [p for p in _staged_aou_paths(target) if "/assumptions_of_use/_main/aou_root_cause_aous/" in p]
    asserts.true(env, staged, "AoU measure of the safety analysis must be staged for Sphinx")
    return analysistest.end(env)

aou_derived_test = analysistest.make(_aou_derived_test_impl)

def _aou_none_test_impl(ctx):
    env = analysistest.begin(ctx)
    target = analysistest.target_under_test(env)

    asserts.equals(
        env,
        [],
        target[ForwardedAoUInfo].own_aou_lobster.to_list(),
    )
    asserts.equals(env, [], _staged_aou_paths(target))
    return analysistest.end(env)

aou_none_test = analysistest.make(_aou_none_test_impl)

def _aou_merged_test_impl(ctx):
    env = analysistest.begin(ctx)

    merge_actions = [
        a
        for a in analysistest.target_actions(env)
        if a.mnemonic == "MergeLobsterItems" and a.outputs.to_list()[0].basename == "received_aous.lobster"
    ]
    asserts.equals(env, 1, len(merge_actions))
    if len(merge_actions) != 1:
        return analysistest.end(env)

    # The AoU arrives directly and chain-forwarded: one file per path.
    inputs = [
        f.basename
        for f in merge_actions[0].inputs.to_list()
        if f.extension == "lobster"
    ]
    asserts.equals(
        env,
        ["aou_root_cause_aous.lobster", "chain_forwarded_aous.lobster"],
        sorted(inputs),
    )
    return analysistest.end(env)

aou_merged_test = analysistest.make(_aou_merged_test_impl)

def _assessment_config_test_impl(ctx):
    env = analysistest.begin(ctx)

    expand_actions = [
        a
        for a in analysistest.target_actions(env)
        if a.mnemonic == "TemplateExpand" and a.outputs.to_list()[0].basename == "de_traceability_config"
    ]
    asserts.equals(env, 1, len(expand_actions))
    if len(expand_actions) != 1:
        return analysistest.end(env)
    blocks = expand_actions[0].substitutions

    # Release emits the checking level even when it holds no assessments.
    assessment_block = blocks["{ASSESSMENT_BLOCK}"]
    asserts.true(env, 'requirements "Root Cause Assessments"' in assessment_block, assessment_block)
    asserts.true(env, 'trace to: "Root Causes"' in assessment_block, assessment_block)
    asserts.equals(
        env,
        ctx.attr.expect_own_aou,
        'trace to: "Assumptions of Use"' in assessment_block,
        assessment_block,
    )
    asserts.equals(
        env,
        ctx.attr.expect_own_aou,
        'requirements "Assumptions of Use"' in blocks["{OWN_AOU_BLOCK}"],
        blocks["{OWN_AOU_BLOCK}"],
    )

    # Measures are referenced by tag only: neither Root Causes nor Component
    # Requirements are restricted or traced by them.
    asserts.false(env, "requires" in blocks["{RC_BLOCK}"], blocks["{RC_BLOCK}"])
    asserts.false(env, "Root Causes" in blocks["{COMP_REQ_BLOCK}"], blocks["{COMP_REQ_BLOCK}"])
    return analysistest.end(env)

assessment_config_test = analysistest.make(
    _assessment_config_test_impl,
    attrs = {"expect_own_aou": attr.bool(mandatory = True)},
)
