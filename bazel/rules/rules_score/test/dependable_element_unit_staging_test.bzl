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
"""Analysis tests: dependable_element stages each unit into its own directory."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _unit_same_puml_names_test_impl(ctx):
    env = analysistest.begin(ctx)
    short_paths = [
        f.short_path
        for f in analysistest.target_under_test(env)[DefaultInfo].files.to_list()
    ]
    for unit_name in ctx.attr.unit_names:
        for suffix in [
            "/units/{}/class_diagram.puml".format(unit_name),
            "/units/{}/design/class_diagram.puml".format(unit_name),
            "/units/{}/design/design.rst".format(unit_name),
        ]:
            asserts.true(
                env,
                any([p.endswith(suffix) for p in short_paths]),
                "missing staged '{}' in: {}".format(suffix, short_paths),
            )
    return analysistest.end(env)

unit_same_puml_names_test = analysistest.make(
    _unit_same_puml_names_test_impl,
    attrs = {"unit_names": attr.string_list(mandatory = True)},
)

def _unit_design_collision_fails_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(
        env,
        "'units/same_puml_clash_unit/class_diagram.puml' would be staged by both",
    )
    return analysistest.end(env)

unit_design_collision_fails_test = analysistest.make(
    _unit_design_collision_fails_test_impl,
    expect_failure = True,
)

def _same_name_units_fail_test_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(env, "share the name 'dup_unit'")
    return analysistest.end(env)

same_name_units_fail_test = analysistest.make(
    _same_name_units_fail_test_impl,
    expect_failure = True,
)
