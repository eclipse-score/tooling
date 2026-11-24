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

"""L1 tests of the documentation path helpers in label_path.bzl."""

load("@bazel_skylib//lib:unittest.bzl", "loadingtest")
load("//bazel/rules/rules_score/private:label_path.bzl", "label_path", "repo_segment")

def label_path_test_suite(name):
    """Defines loading-phase tests for repo_segment and label_path."""
    env = loadingtest.make(name)

    for case, repo_name, expected in [
        ("main_repo", "", "_main"),
        ("module_without_version", "mod+", "mod"),
        ("module_with_plus_version", "mod+1.2.3", "mod"),
        ("module_with_tilde_version", "mod~1.2.3", "mod"),
        ("extension_repo_keeps_canonical_name", "mod++ext+repo", "mod++ext+repo"),
    ]:
        loadingtest.equals(
            env,
            "repo_segment_" + case,
            expected,
            repo_segment(struct(repo_name = repo_name)),
        )

    for case, label, expected in [
        ("root_package", struct(repo_name = "", package = "", name = "lib"), "_main/lib"),
        ("nested_package", struct(repo_name = "mod+", package = "a/b", name = "lib"), "mod/a/b/lib"),
        ("name_with_slash", struct(repo_name = "", package = "a", name = "b/c"), "_main/a/b/c"),
    ]:
        loadingtest.equals(
            env,
            "label_path_" + case,
            expected,
            label_path(label),
        )
