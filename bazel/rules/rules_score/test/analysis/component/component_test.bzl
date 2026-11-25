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
"""L2 tests of the component traceability config (rules_testing)."""

load("@rules_testing//lib:analysis_test.bzl", "analysis_test", "test_suite")
load("//lib:analysis.bzl", "template_substitutions")
load(
    "//lib:fixtures.bzl",
    "fixture_asr",
    "fixture_component",
    "fixture_component_requirements",
    "fixture_feature_requirements",
)

def _config(env, target):
    return template_substitutions(env, target, target.label.name + "_traceability_config")

def _asr_via_feature_impl(env, target):
    config = _config(env, target)
    env.expect.that_str(config["{ASSUMED_SYSTEM_REQ_SOURCES}"]).contains("asr.lobster")
    env.expect.that_bool("asr.lobster" in config["{FEAT_REQ_SOURCES}"]).equals(False)

def _asr_via_feature_test(name):
    asr = fixture_asr(name + "_asr")
    feat = fixture_feature_requirements(name + "_feat", deps = [asr])
    subject = fixture_component(name + "_subject", requirements = [feat])
    analysis_test(name = name, impl = _asr_via_feature_impl, target = subject)

def _asr_via_nested_feature_impl(env, target):
    env.expect.that_str(_config(env, target)["{ASSUMED_SYSTEM_REQ_SOURCES}"]).contains("asr.lobster")

def _asr_via_nested_feature_test(name):
    asr = fixture_asr(name + "_asr")
    base = fixture_feature_requirements(name + "_base", deps = [asr])
    top = fixture_feature_requirements(name + "_top", deps = [base])
    subject = fixture_component(name + "_subject", requirements = [top])
    analysis_test(name = name, impl = _asr_via_nested_feature_impl, target = subject)

def _asr_direct_impl(env, target):
    config = _config(env, target)
    env.expect.that_str(config["{ASSUMED_SYSTEM_REQ_SOURCES}"]).contains("asr.lobster")
    env.expect.that_str(config["{FEAT_REQ_SOURCES}"]).equals("")

def _asr_direct_test(name):
    asr = fixture_asr(name + "_asr")
    comp_req = fixture_component_requirements(name + "_comp_req", deps = [asr])
    subject = fixture_component(name + "_subject", requirements = [comp_req, asr])
    analysis_test(name = name, impl = _asr_direct_impl, target = subject)

def component_test_suite(name):
    """Registers the component analysis tests as test suite `name`."""
    test_suite(
        name = name,
        tests = [
            _asr_via_feature_test,
            _asr_via_nested_feature_test,
            _asr_direct_test,
        ],
    )
