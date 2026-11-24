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

"""L2 tests of the rst_to_trlc action contract (rules_testing)."""

load("@rules_testing//lib:analysis_test.bzl", "analysis_test", "test_suite")
load("@rules_testing//lib:util.bzl", "util")
load("@score_tooling//bazel/rules/rules_score:rules_score.bzl", "rst_to_trlc")

def _converter_action(env, target):
    return env.expect.that_target(target).action_named("RstToTrlc")

def _package_forwarded_impl(env, target):
    _converter_action(env, target).contains_at_least_args(["--package", "CustomPackage"])

def _package_forwarded_test(name):
    util.helper_target(
        rst_to_trlc,
        name = name + "_subject",
        srcs = ["aou.rst"],
        package = "CustomPackage",
    )
    analysis_test(name = name, impl = _package_forwarded_impl, target = name + "_subject")

def _ref_package_forwarded_impl(env, target):
    _converter_action(env, target).contains_at_least_args(["--ref-package", "FeatureRequirements"])

def _ref_package_forwarded_test(name):
    util.helper_target(
        rst_to_trlc,
        name = name + "_subject",
        srcs = ["aou.rst"],
        ref_package = "FeatureRequirements",
    )
    analysis_test(name = name, impl = _ref_package_forwarded_impl, target = name + "_subject")

def _default_packages_not_forwarded_impl(env, target):
    # ActionSubject.argv() formats values as templates, which breaks contains_none_of.
    argv = _converter_action(env, target).actual.argv
    env.expect.that_collection(argv).contains_none_of(["--package", "--ref-package"])

def _default_packages_not_forwarded_test(name):
    util.helper_target(
        rst_to_trlc,
        name = name + "_subject",
        srcs = ["aou.rst"],
    )
    analysis_test(name = name, impl = _default_packages_not_forwarded_impl, target = name + "_subject")

def rst_to_trlc_test_suite(name):
    """Registers the rst_to_trlc analysis tests as test suite `name`."""
    test_suite(
        name = name,
        tests = [
            _package_forwarded_test,
            _ref_package_forwarded_test,
            _default_packages_not_forwarded_test,
        ],
    )
