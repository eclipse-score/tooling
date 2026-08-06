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

"""Analysis tests for spec_extras support in requirements macros."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@trlc//:trlc.bzl", "TrlcProviderInfo")

def _spec_extras_in_provider_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    spec_basenames = [f.basename for f in target_under_test[TrlcProviderInfo].spec.to_list()]

    asserts.true(
        env,
        "score_requirements_model.rsl" in spec_basenames,
        "Base score spec should still be present in TrlcProviderInfo.spec",
    )
    asserts.true(
        env,
        "extra_model.rsl" in spec_basenames,
        "spec_extras RSL should be merged into TrlcProviderInfo.spec",
    )

    return analysistest.end(env)

spec_extras_in_provider_test = analysistest.make(_spec_extras_in_provider_test_impl)

def requirements_spec_extras_test_suite(name):
    native.test_suite(
        name = name,
        tests = [
            ":spec_extras_in_provider_test",
        ],
    )
