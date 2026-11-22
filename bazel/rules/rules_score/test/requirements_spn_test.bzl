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
"""Analysis tests for the *_spn requirement macros (requirements from Sphinx-Needs)."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load(
    "@score_tooling//bazel/rules/rules_score:providers.bzl",
    "AnalysisInfo",
    "AssumedSystemRequirementsInfo",
    "AssumptionsOfUseInfo",
    "ComponentRequirementsInfo",
    "FeatureRequirementsInfo",
    "SafetyAnalysisProviderInfo",
    "SphinxSourcesInfo",
)

# (provider, field holding the .lobster files) per requirement kind
_REQ_PROVIDERS = [
    (FeatureRequirementsInfo, "srcs"),
    (ComponentRequirementsInfo, "srcs"),
    (AssumedSystemRequirementsInfo, "srcs"),
    (AssumptionsOfUseInfo, "aou_lobster"),
]

def _lobster_files(target):
    files = []
    for provider, field in _REQ_PROVIDERS:
        if provider in target:
            files.append(getattr(target[provider], field))
    return depset(transitive = files)

def _basenames(files):
    return sorted([f.basename for f in files])

def _spn_provider_test_impl(ctx):
    """One requirement provider with the lobster_sphinx_needs output, an RST page, no TRLC."""
    env = analysistest.begin(ctx)
    target = analysistest.target_under_test(env)
    name = target.label.name

    asserts.equals(
        env,
        1,
        len([p for p, _ in _REQ_PROVIDERS if p in target]),
        "expected exactly one requirement provider",
    )
    asserts.equals(env, [name + "_needs.lobster"], _basenames(_lobster_files(target).to_list()))
    asserts.equals(env, [name + ".rst"], _basenames(target[SphinxSourcesInfo].srcs.to_list()))

    mnemonics = [a.mnemonic for a in analysistest.target_actions(env)]
    asserts.true(env, "LobsterSphinxNeeds" in mnemonics, "expected a LobsterSphinxNeeds action")
    asserts.false(env, "TrlcToRst" in mnemonics, "a *_spn target must not render TRLC")
    return analysistest.end(env)

spn_provider_test = analysistest.make(_spn_provider_test_impl)

def _safety_analysis_spn_provider_test_impl(ctx):
    """The LOBSTER files of the TRLC safety_analysis, the AoU measures, no TRLC tool."""
    env = analysistest.begin(ctx)
    target = analysistest.target_under_test(env)

    asserts.equals(
        env,
        ["assessments.lobster", "failuremodes.lobster", "fta_root_causes.lobster"],
        sorted(target[AnalysisInfo].lobster_files.keys()),
    )
    asserts.equals(env, ["aous_spn"], [t.label.name for t in target[SafetyAnalysisProviderInfo].aou_targets])
    asserts.equals(
        env,
        ["fta_no_return.puml", "fta_wrong_bit.puml"],
        _basenames(target[SphinxSourcesInfo].aux_srcs.to_list()),
    )

    mnemonics = [a.mnemonic for a in analysistest.target_actions(env)]
    asserts.true(env, "SafetyAnalysisSpn" in mnemonics, "expected a SafetyAnalysisSpn action")
    asserts.equals(env, [], [m for m in mnemonics if "trlc" in m.lower()], "safety_analysis_spn must not run TRLC tools")
    return analysistest.end(env)

safety_analysis_spn_provider_test = analysistest.make(_safety_analysis_spn_provider_test_impl)

def _requirement_lobster_files_impl(ctx):
    return [DefaultInfo(files = depset(transitive = [_lobster_files(t) for t in ctx.attr.requirements]))]

requirement_lobster_files = rule(
    implementation = _requirement_lobster_files_impl,
    attrs = {"requirements": attr.label_list()},
    doc = "Exposes the LOBSTER files of requirement targets for content tests.",
)
