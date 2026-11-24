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
"""Analysis test of the generated architecture JSON of a dependable element."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _architecture_json_test_impl(ctx):
    env = analysistest.begin(ctx)
    actions = [
        action
        for action in analysistest.target_actions(env)
        if action.mnemonic == "FileWrite" and action.outputs.to_list()[0].basename == "architecture.json"
    ]
    asserts.equals(env, 1, len(actions))
    if len(actions) == 1:
        architecture = json.decode(actions[0].content)["components"]
        target = analysistest.target_under_test(env)
        expected_label = str(target.label.same_package_label(ctx.attr.module_name))
        asserts.true(env, expected_label in architecture)
        asserts.false(env, ctx.attr.module_name in architecture)
        asserts.false(env, str(target.label) in architecture)
        parent_label = str(target.label.same_package_label("test_parent_component"))
        nested_label = str(target.label.same_package_label("test_nested_component"))
        asserts.equals(env, {
            "design_name": ctx.attr.module_name,
            "components": [{"label": parent_label, "design_name": "test_parent_component"}],
        }, architecture.get(expected_label))
        asserts.equals(env, {
            "design_name": "test_parent_component",
            "components": [{"label": nested_label, "design_name": "test_nested_component"}],
            "units": [{"label": str(target.label.same_package_label("test_unit")), "design_name": "test_unit"}],
        }, architecture.get(parent_label))
        asserts.equals(env, {
            "design_name": "test_nested_component",
            "units": [{"label": str(target.label.same_package_label("test_unit2")), "design_name": "test_unit2"}],
        }, architecture.get(nested_label))
    return analysistest.end(env)

architecture_json_test = analysistest.make(
    _architecture_json_test_impl,
    attrs = {"module_name": attr.string(mandatory = True)},
)
