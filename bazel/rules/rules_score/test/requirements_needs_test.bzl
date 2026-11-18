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
"""Tests for requirement macros fed with Sphinx-Needs needs.json files."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load(
    "@score_tooling//bazel/rules/rules_score:providers.bzl",
    "AssumedSystemRequirementsInfo",
    "AssumptionsOfUseInfo",
    "ComponentRequirementsInfo",
    "FeatureRequirementsInfo",
    "SphinxSourcesInfo",
)

def _basenames(files):
    return sorted([f.basename for f in files])

def _mnemonics(env):
    return [a.mnemonic for a in analysistest.target_actions(env)]

def _needs_only_test_impl(ctx):
    """needs without srcs: only lobster_sphinx_needs runs, no TRLC rendering."""
    env = analysistest.begin(ctx)
    target = analysistest.target_under_test(env)
    name = target.label.name

    asserts.equals(
        env,
        [name + "_needs.lobster"],
        _basenames(target[ComponentRequirementsInfo].srcs.to_list()),
    )
    asserts.equals(env, [name + ".rst"], _basenames(target[SphinxSourcesInfo].srcs.to_list()))
    asserts.true(env, "LobsterSphinxNeeds" in _mnemonics(env), "expected a LobsterSphinxNeeds action")
    asserts.false(env, "TrlcToRst" in _mnemonics(env), "a needs-only target must not render TRLC")
    return analysistest.end(env)

needs_only_test = analysistest.make(_needs_only_test_impl)

def _needs_and_trlc_test_impl(ctx):
    """srcs and needs: both LOBSTER files and both RST pages are provided."""
    env = analysistest.begin(ctx)
    target = analysistest.target_under_test(env)
    name = target.label.name

    lobster_files = _basenames(target[ComponentRequirementsInfo].srcs.to_list())
    asserts.equals(env, 2, len(lobster_files), "expected the lobster-trlc and the lobster_sphinx_needs output")
    asserts.true(env, name + "_needs.lobster" in lobster_files)
    asserts.equals(
        env,
        [name + ".rst", name + "_needs.rst"],
        _basenames(target[SphinxSourcesInfo].srcs.to_list()),
    )
    return analysistest.end(env)

needs_and_trlc_test = analysistest.make(_needs_and_trlc_test_impl)

def _needs_provider_test_impl(ctx):
    """Every requirement kind passes the needs LOBSTER file in its own provider."""
    env = analysistest.begin(ctx)
    target = analysistest.target_under_test(env)
    expected = target.label.name + "_needs.lobster"

    if FeatureRequirementsInfo in target:
        files = target[FeatureRequirementsInfo].srcs
    elif AssumedSystemRequirementsInfo in target:
        files = target[AssumedSystemRequirementsInfo].srcs
    else:
        files = target[AssumptionsOfUseInfo].aou_lobster
    asserts.equals(env, [expected], _basenames(files.to_list()))
    return analysistest.end(env)

needs_provider_test = analysistest.make(_needs_provider_test_impl)

def _requirement_lobster_files_impl(ctx):
    files = []
    for target in ctx.attr.requirements:
        for provider, attr in [
            (ComponentRequirementsInfo, "srcs"),
            (FeatureRequirementsInfo, "srcs"),
            (AssumedSystemRequirementsInfo, "srcs"),
            (AssumptionsOfUseInfo, "aou_lobster"),
        ]:
            if provider in target:
                files.append(getattr(target[provider], attr))
    return [DefaultInfo(files = depset(transitive = files))]

requirement_lobster_files = rule(
    implementation = _requirement_lobster_files_impl,
    attrs = {"requirements": attr.label_list()},
    doc = "Exposes the LOBSTER files of requirement targets for content tests.",
)
