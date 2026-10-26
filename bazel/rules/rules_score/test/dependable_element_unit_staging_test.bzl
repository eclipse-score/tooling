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
"""Analysis tests: dependable_element fails on staged-path clashes between units."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _unit_design_collision_fails_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(
        env,
        "'units/_main/same_puml_clash_unit/class_diagram.puml' would be staged by both",
    )
    return analysistest.end(env)

unit_design_collision_fails_test = analysistest.make(
    _unit_design_collision_fails_test_impl,
    expect_failure = True,
)
