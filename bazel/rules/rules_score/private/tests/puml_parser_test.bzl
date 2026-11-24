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
"""Loading-phase tests of disambiguated_stems() in puml_parser.bzl.

Files are faked with `struct(path, short_path, basename, extension, owner)`; the
residual-collision `fail()` path is covered at L2 (puml_stem_collision_test).
"""

load("@bazel_skylib//lib:unittest.bzl", "loadingtest")
load("//bazel/rules/rules_score/private:puml_parser.bzl", "disambiguated_stems")

def _fake_file(short_path):
    basename = short_path.split("/")[-1]
    return struct(
        path = short_path,
        short_path = short_path,
        basename = basename,
        extension = basename.split(".")[-1] if "." in basename else "",
        owner = None,
    )

def puml_parser_test_suite(name):
    """Defines loading-phase tests for disambiguated_stems().

    Args:
        name: Suite name; test targets and the `<name>_tests` test_suite derive from it.
    """
    env = loadingtest.make(name)
    ctx = struct(label = struct(package = "pkg", workspace_name = ""))

    loadingtest.equals(
        env,
        "unique_basenames_keep_plain_stem",
        {"pkg/a.puml": "a", "pkg/sub/b.puml": "b"},
        disambiguated_stems(ctx, [_fake_file("pkg/a.puml"), _fake_file("pkg/sub/b.puml")]),
    )

    loadingtest.equals(
        env,
        "colliding_basenames_are_prefixed_with_directory",
        {"pkg/x/view.puml": "x__view", "pkg/y/z/view.puml": "y_z__view"},
        disambiguated_stems(ctx, [_fake_file("pkg/x/view.puml"), _fake_file("pkg/y/z/view.puml")]),
    )

    loadingtest.equals(
        env,
        "colliding_basename_in_package_root_keeps_plain_stem",
        {"pkg/view.puml": "view", "pkg/x/view.puml": "x__view"},
        disambiguated_stems(ctx, [_fake_file("pkg/view.puml"), _fake_file("pkg/x/view.puml")]),
    )

    loadingtest.equals(
        env,
        "non_puml_files_are_ignored",
        {"pkg/a.puml": "a"},
        disambiguated_stems(ctx, [_fake_file("pkg/a.puml"), _fake_file("pkg/notes.md")]),
    )

    loadingtest.equals(env, "no_files", {}, disambiguated_stems(ctx, []))
