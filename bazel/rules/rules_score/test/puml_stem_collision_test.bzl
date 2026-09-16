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
"""Analysis test for architectural_design's `_disambiguated_stems()` residual
collision `fail()` path (bazel/rules/rules_score/private/architectural_design.bzl).

Two same-basename diagrams already need directory-based disambiguation
(`a/b/foo.puml` and `a_b/foo.puml` both have basename `foo.puml`), and that
disambiguation itself collides once `/` is replaced with `_` in both
directory parts (`a/b` -> `a_b`, `a_b` -> `a_b`) -- both end up wanting the
same output stem `a_b__foo`. This must fail analysis with a message naming
both source files, rather than silently letting one diagram's output
overwrite the other's.
"""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _stem_collision_fails_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(
        env,
        "both disambiguate to the output stem 'fixtures_stem_collision_a_b__foo'",
    )
    return analysistest.end(env)

stem_collision_fails_test = analysistest.make(
    _stem_collision_fails_test_impl,
    expect_failure = True,
)
