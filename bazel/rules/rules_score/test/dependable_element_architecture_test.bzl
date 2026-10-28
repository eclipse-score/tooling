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
"""Regression tests for the DE's architecture label keys.

Controlled contexts exercise the production collector across packages and
repositories; an analysis test verifies the generated architecture JSON.
"""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts", "unittest")
load("@score_tooling//bazel/rules/rules_score/private:architecture_aspect.bzl", "CurrentArchitectureProviderInfo")
load("@score_tooling//bazel/rules/rules_score/private:dependable_element.bzl", "dependable_element_test_helpers")

def _context(label, components):
    return struct(
        label = label,
        attr = struct(module_name = "engine", components = components),
    )

def _label_keys_test_impl(ctx):
    env = unittest.begin(ctx)

    # Cover root/non-root packages and an external repository. Derive strings
    # from Label objects rather than assuming Bazel's repository prefix format.
    for index_label in [
        Label("//:engine_index"),
        Label("//fixtures/architecture:engine_index"),
        Label("@score_tooling//bazel/rules/rules_score:engine_index"),
    ]:
        expected_label = str(index_label.same_package_label("engine"))
        other_label = str(Label("@@{}//other:engine".format(index_label.repo_name)))
        existing_entry = {
            "units": [{"label": str(index_label.same_package_label("worker")), "design_name": "worker"}],
        }
        existing = {other_label: existing_entry}
        component = {CurrentArchitectureProviderInfo: CurrentArchitectureProviderInfo(components = existing)}
        actual = dependable_element_test_helpers.collect_architecture_components(_context(index_label, [component]))

        asserts.equals(env, {other_label: existing_entry, expected_label: {}}, actual)
        asserts.false(env, "engine" in actual)
        asserts.false(env, str(index_label) in actual)
        asserts.equals(env, {other_label: existing_entry}, existing)

    return unittest.end(env)

label_keys_test = unittest.make(_label_keys_test_impl)

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
            "components": [{"label": parent_label, "design_name": "test_parent_component"}],
        }, architecture.get(expected_label))
        asserts.equals(env, {
            "components": [{"label": nested_label, "design_name": "test_nested_component"}],
            "units": [{"label": str(target.label.same_package_label("test_unit")), "design_name": "test_unit"}],
        }, architecture.get(parent_label))
        asserts.equals(env, {
            "units": [{"label": str(target.label.same_package_label("test_unit2")), "design_name": "test_unit2"}],
        }, architecture.get(nested_label))
    return analysistest.end(env)

architecture_json_test = analysistest.make(
    _architecture_json_test_impl,
    attrs = {"module_name": attr.string(mandatory = True)},
)

def architecture_collection_test_suite(name):
    label_keys_test(name = name + "_label_keys_test")
    native.test_suite(
        name = name,
        tests = [
            ":" + name + "_label_keys_test",
        ],
    )
