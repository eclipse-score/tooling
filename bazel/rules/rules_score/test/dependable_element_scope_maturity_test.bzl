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
"""Analysis tests: a unit's maturity controls its certified scope violations."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _scope_violation_fails_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(env, "Not in certified scope")
    return analysistest.end(env)

scope_violation_fails_test = analysistest.make(
    _scope_violation_fails_test_impl,
    expect_failure = True,
)

def _scope_violation_passes_test_impl(ctx):
    env = analysistest.begin(ctx)
    return analysistest.end(env)

scope_violation_passes_test = analysistest.make(_scope_violation_passes_test_impl)
