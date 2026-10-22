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
"""Loading-phase tests for certified-scope matching helpers."""

load("@bazel_skylib//lib:unittest.bzl", "loadingtest")
load(
    "@score_tooling//bazel/rules/rules_score/private:certified_scope.bzl",
    "build_scope_index",
    "collect_scope_violations",
    "find_scope_violation",
)

def certified_scope_matching_test_suite(name):
    """Defines loading-phase tests for package scope matching semantics."""
    env = loadingtest.make(name)
    scope_index = build_scope_index([Label("//:__pkg__")])

    loadingtest.equals(
        env,
        "pkg_scope_covers_target_in_its_package",
        None,
        find_scope_violation(scope_index, Label("//:library")),
    )

    loadingtest.equals(
        env,
        "pkg_scope_does_not_cover_child_package",
        "package '//child' is not covered",
        find_scope_violation(scope_index, Label("//child:library")),
    )

    loadingtest.equals(
        env,
        "pkg_scope_does_not_cover_nested_child_package",
        "package '//child/nested' is not covered",
        find_scope_violation(scope_index, Label("//child/nested:library")),
    )

    loadingtest.equals(
        env,
        "aggregated_scope_violation_reports_originating_unit",
        ["Unit @@//units:my_unit depends on @@//child:library, but the dependency is not in any certified scope (package '//child' is not covered)."],
        collect_scope_violations(
            scope_index,
            [struct(
                unit_label = Label("//units:my_unit"),
                dependencies = depset([Label("//child:library")]),
            )],
        ),
    )

    loadingtest.equals(
        env,
        "shared_uncovered_dependency_reports_every_originating_unit",
        [
            "Unit @@//units:first depends on @@//child:library, but the dependency is not in any certified scope (package '//child' is not covered).",
            "Unit @@//units:second depends on @@//child:library, but the dependency is not in any certified scope (package '//child' is not covered).",
        ],
        collect_scope_violations(
            scope_index,
            [
                struct(
                    unit_label = Label("//units:first"),
                    dependencies = depset([Label("//child:library")]),
                ),
                struct(
                    unit_label = Label("//units:second"),
                    dependencies = depset([Label("//child:library")]),
                ),
            ],
        ),
    )

    subpackages_scope_index = build_scope_index([Label("//:__subpackages__")])

    loadingtest.equals(
        env,
        "subpackages_scope_covers_arbitrary_nested_target",
        None,
        find_scope_violation(subpackages_scope_index, Label("//child/nested:any_target")),
    )

    root_and_child_scope_index = build_scope_index([
        Label("//:root_lib"),
        Label("//a:child_lib"),
    ])

    loadingtest.equals(
        env,
        "root_scope_does_not_hide_child_package_scope",
        None,
        find_scope_violation(root_and_child_scope_index, Label("//a:child_lib")),
    )

    external_scope_index = build_scope_index([
        Label("@bazel_skylib//lib:unittest.bzl"),
    ])

    loadingtest.equals(
        env,
        "external_scope_covers_same_repository_target",
        None,
        find_scope_violation(external_scope_index, Label("@bazel_skylib//lib:unittest.bzl")),
    )

    loadingtest.equals(
        env,
        "external_scope_does_not_cover_same_named_main_repository_target",
        "repository <main> is not covered",
        find_scope_violation(external_scope_index, Label("//lib:unittest.bzl")),
    )

    exact_target_scope_index = build_scope_index([Label("//a/b:lib")])

    loadingtest.equals(
        env,
        "exact_target_scope_rejects_other_target_in_same_package",
        "dependency 'other' is not certified in package 'a/b'; declared certified targets: lib",
        find_scope_violation(exact_target_scope_index, Label("//a/b:other")),
    )

    loadingtest.equals(
        env,
        "exact_target_scope_rejects_target_in_shallower_package",
        "package '//a' is not covered",
        find_scope_violation(exact_target_scope_index, Label("//a:other")),
    )

    nested_scope_index = build_scope_index([Label("//a/b:__subpackages__")])
    for package_path in ["a/b", "a/b/c", "a/b/c/d"]:
        loadingtest.equals(
            env,
            "subpackage_scope_covers_" + package_path.replace("/", "_"),
            None,
            find_scope_violation(nested_scope_index, Label("//{}:lib".format(package_path))),
        )
    for package_path in ["", "a", "a/bc"]:
        loadingtest.equals(
            env,
            "subpackage_scope_rejects_" + (package_path.replace("/", "_") or "root"),
            "package '//{}' is not covered".format(package_path),
            find_scope_violation(nested_scope_index, Label("//{}:lib".format(package_path))),
        )

    for wildcard in ["__pkg__", "__subpackages__"]:
        external_wildcard_index = build_scope_index([
            Label("@bazel_skylib//lib:" + wildcard),
        ])
        loadingtest.equals(
            env,
            "external_{}_covers_its_package".format(wildcard),
            None,
            find_scope_violation(external_wildcard_index, Label("@bazel_skylib//lib:library")),
        )
        loadingtest.equals(
            env,
            "external_{}_does_not_cover_main_repository".format(wildcard),
            "repository <main> is not covered",
            find_scope_violation(external_wildcard_index, Label("//lib:library")),
        )

def certified_scope_index_test_suite(name):
    """Defines loading-phase tests for certified-scope index construction."""
    env = loadingtest.make(name)
    mixed_depth_scope_index = build_scope_index([
        Label("//a/b:lib"),
        Label("//a/b/c:lib2"),
    ])

    loadingtest.equals(
        env,
        "mixed_depth_scopes_use_full_package_keys",
        {
            "": {
                "a/b": {
                    "targets": {"lib": True},
                    "covers_package": False,
                    "covers_subpackages": False,
                },
                "a/b/c": {
                    "targets": {"lib2": True},
                    "covers_package": False,
                    "covers_subpackages": False,
                },
            },
        },
        mixed_depth_scope_index,
    )

    external_scope = Label("@score_tooling//bazel/rules/rules_score:__pkg__")
    external_scope_index = build_scope_index([external_scope])

    loadingtest.equals(
        env,
        "external_scope_uses_repository_name_as_index_key",
        {
            external_scope.workspace_name: {
                "bazel/rules/rules_score": {
                    "targets": {},
                    "covers_package": True,
                    "covers_subpackages": False,
                },
            },
        },
        external_scope_index,
    )

    declarations = [Label("//:lib"), Label("//:__pkg__"), Label("//:__subpackages__")]
    combined_index = build_scope_index(declarations)
    loadingtest.equals(
        env,
        "root_package_groups_targets_and_wildcards",
        {"": {"": {
            "targets": {"lib": True},
            "covers_package": True,
            "covers_subpackages": True,
        }}},
        combined_index,
    )
    loadingtest.equals(
        env,
        "repeated_targets_and_wildcards_produce_the_same_index",
        combined_index,
        build_scope_index(declarations + declarations),
    )
