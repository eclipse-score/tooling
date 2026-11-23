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
"""Analysis test for architectural_design's `.iuml` fragment `fail()` paths
(bazel/rules/rules_score/private/architectural_design.bzl): a fragment cannot
be a generated file or a file outside the package."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _includes_fails_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(env, ctx.attr.expected)
    return analysistest.end(env)

includes_fails_test = analysistest.make(
    _includes_fails_test_impl,
    expect_failure = True,
    attrs = {"expected": attr.string(mandatory = True)},
)
