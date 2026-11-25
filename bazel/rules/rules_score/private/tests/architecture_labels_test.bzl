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
"""Loading-phase tests of the label keys built by the dependable element's architecture collector.

The collector runs on a fake rule context for packages of the main repository and
for an external repository.
"""

load("@bazel_skylib//lib:unittest.bzl", "loadingtest")
load("//bazel/rules/rules_score/private:architecture_aspect.bzl", "CurrentArchitectureProviderInfo")
load("//bazel/rules/rules_score/private:dependable_element.bzl", "dependable_element_test_helpers")

def architecture_labels_test_suite(name):
    """Defines loading-phase tests for the architecture label keys.

    Args:
        name: Suite name; test targets and the `<name>_tests` test_suite derive from it.
    """
    env = loadingtest.make(name)

    for case, index_label in [
        ("root_package", Label("//:engine_index")),
        ("nested_package", Label("//fixtures/architecture:engine_index")),
        ("external_repository", Label("@bazel_skylib//lib:engine_index")),
    ]:
        # Derive strings from Label objects rather than assuming the repository prefix format.
        expected_label = str(index_label.same_package_label("engine"))
        other_label = str(Label("@@{}//other:engine".format(index_label.repo_name)))
        existing_entry = {
            "units": [{"label": str(index_label.same_package_label("worker")), "design_name": "worker"}],
        }
        existing = {other_label: existing_entry}
        component = {CurrentArchitectureProviderInfo: CurrentArchitectureProviderInfo(components = existing)}
        ctx = struct(
            label = index_label,
            attr = struct(module_name = "engine", components = [component]),
        )
        actual = dependable_element_test_helpers.collect_architecture_components(ctx)

        loadingtest.equals(
            env,
            case + "_entries",
            {other_label: existing_entry, expected_label: {"design_name": "engine"}},
            actual,
        )
        loadingtest.equals(env, case + "_no_bare_module_name_key", False, "engine" in actual)
        loadingtest.equals(env, case + "_no_index_label_key", False, str(index_label) in actual)
        loadingtest.equals(env, case + "_input_not_mutated", {other_label: existing_entry}, existing)
