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
"""
Loading-phase unit tests for plan_view_layout()
(bazel/rules/rules_score/private/puml_utils.bzl).

`plan_view_layout` is pure Starlark -- it only reads `.short_path` / `.extension`
/ `.owner.workspace_name` off whatever is passed as a "file" -- so plain
`struct(short_path = ..., extension = ..., owner = ...)` fakes stand in for
real File objects, and skylib's `loadingtest` (not `analysistest`) is used: no
target_under_test / analysis phase is needed.
"""

load("@bazel_skylib//lib:unittest.bzl", "loadingtest")
load(
    "@score_tooling//bazel/rules/rules_score/private:puml_utils.bzl",
    "plan_view_layout",
)

def _fake_file(short_path, owner_repo = None):
    """`owner_repo` fakes `File.owner.workspace_name` -- omitted (None) for
    plain same-build-main-repo fakes used by most cases below; set it to
    model a file whose owning label lives in a specific repository, for the
    `own_repo`-aware short_path-marker-stripping tests."""
    basename = short_path.split("/")[-1]
    extension = basename.split(".")[-1] if "." in basename else ""
    owner = struct(workspace_name = owner_repo) if owner_repo != None else None
    return struct(short_path = short_path, extension = extension, path = short_path, owner = owner)

def puml_layout_test_suite(name):
    """Defines the loading-phase test suite for plan_view_layout().

    Args:
        name: Suite name; individual test targets and the aggregating
            `<name>_tests` test_suite are derived from it.
    """
    env = loadingtest.make(name)

    # --- authored index.rst composes with the generated directory index -------

    overview = _fake_file("static/overview.puml")
    authored_index = _fake_file("static/index.rst")
    plan = plan_view_layout([overview, authored_index], "")

    loadingtest.equals(env, "plan_index_composes_no_errors", [], plan.errors)
    loadingtest.equals(
        env,
        "plan_index_composes_staged",
        [(overview, "static/overview.puml"), (authored_index, "static/index.rst.inc")],
        plan.staged,
    )
    loadingtest.equals(
        env,
        "plan_index_composes_wrappers",
        [(overview, "static", "overview")],
        plan.wrappers,
    )
    loadingtest.equals(
        env,
        "plan_index_composes_indexes",
        [
            struct(directory = "", entries = ["static/index"], body_relative_path = None),
            struct(directory = "static", entries = ["overview"], body_relative_path = "static/index.rst.inc"),
        ],
        plan.indexes,
    )

    # --- authored same-stem .rst suppresses the generated diagram wrapper ------
    # The wrapper is only a placeholder for prose that doesn't exist yet; real
    # authored content always wins, but the .puml is still staged as a sibling
    # so the author's own `.. uml::` directive resolves.

    rst_overview_puml = _fake_file("static/overview.puml")
    rst_overview_rst = _fake_file("static/overview.rst")
    rst_plan = plan_view_layout([rst_overview_puml, rst_overview_rst], "")

    loadingtest.equals(env, "plan_rst_override_no_errors", [], rst_plan.errors)
    loadingtest.equals(env, "plan_rst_override_no_wrapper", [], rst_plan.wrappers)
    loadingtest.equals(
        env,
        "plan_rst_override_staged",
        [(rst_overview_puml, "static/overview.puml"), (rst_overview_rst, "static/overview.rst")],
        rst_plan.staged,
    )

    # --- authored same-stem .md also suppresses the generated wrapper ----------

    md_overview_puml = _fake_file("static/overview.puml")
    md_overview_md = _fake_file("static/overview.md")
    md_plan = plan_view_layout([md_overview_puml, md_overview_md], "")

    loadingtest.equals(env, "plan_md_override_no_errors", [], md_plan.errors)
    loadingtest.equals(env, "plan_md_override_no_wrapper", [], md_plan.wrappers)

    # --- nested directories: index entries are generated at every level --------

    nested_overview = _fake_file("static/overview.puml")
    nested_detail = _fake_file("static/sub/detail.puml")
    nested_plan = plan_view_layout([nested_overview, nested_detail], "")

    loadingtest.equals(env, "nested_directories_no_errors", [], nested_plan.errors)
    loadingtest.equals(
        env,
        "nested_directories_indexes",
        [
            struct(directory = "", entries = ["static/index"], body_relative_path = None),
            struct(directory = "static", entries = ["overview", "sub/index"], body_relative_path = None),
            struct(directory = "static/sub", entries = ["detail"], body_relative_path = None),
        ],
        nested_plan.indexes,
    )

    # --- pass-through directories collapse into their only descendant ---------
    # "a" and "a/b" hold nothing of their own and each lead to a single child,
    # so they contribute no navigation page; the root links straight to
    # "a/b/c/index".

    collapse_leaf = _fake_file("a/b/c/leaf.puml")
    collapse_plan = plan_view_layout([collapse_leaf], "")

    loadingtest.equals(env, "collapse_pass_through_no_errors", [], collapse_plan.errors)
    loadingtest.equals(
        env,
        "collapse_pass_through_indexes",
        [
            struct(directory = "", entries = ["a/b/c/index"], body_relative_path = None),
            struct(directory = "a/b/c", entries = ["leaf"], body_relative_path = None),
        ],
        collapse_plan.indexes,
    )

    # --- an authored index body keeps its directory in the navigation ---------

    kept_body = _fake_file("a/b/index.rst")
    kept_leaf = _fake_file("a/b/c/leaf.puml")
    kept_plan = plan_view_layout([kept_body, kept_leaf], "")

    loadingtest.equals(env, "collapse_authored_body_no_errors", [], kept_plan.errors)
    loadingtest.equals(
        env,
        "collapse_authored_body_indexes",
        [
            struct(directory = "", entries = ["a/b/index"], body_relative_path = None),
            struct(directory = "a/b", entries = ["c/index"], body_relative_path = "a/b/index.rst.inc"),
            struct(directory = "a/b/c", entries = ["leaf"], body_relative_path = None),
        ],
        kept_plan.indexes,
    )

    # --- empty view: no navigable files at all means no navigation at all ------

    empty_plan = plan_view_layout([], "")

    loadingtest.equals(env, "empty_view_no_errors", [], empty_plan.errors)
    loadingtest.equals(env, "empty_view_no_staged", [], empty_plan.staged)
    loadingtest.equals(env, "empty_view_no_wrappers", [], empty_plan.wrappers)
    loadingtest.equals(env, "empty_view_no_indexes", [], empty_plan.indexes)

    # --- cross-package, same-basename files never collide on bare basename -----
    # relative_source_path() falls back to the full workspace-relative
    # short_path for files outside `package`; two same-named files in
    # different directories must stay distinct rather than silently colliding
    # on "overview.puml".

    cross_a = _fake_file("pkg_a/overview.puml")
    cross_b = _fake_file("pkg_b/overview.puml")
    cross_plan = plan_view_layout([cross_a, cross_b], "mypkg")

    loadingtest.equals(env, "cross_package_same_basename_no_errors", [], cross_plan.errors)
    loadingtest.equals(
        env,
        "cross_package_same_basename_staged",
        [(cross_a, "pkg_a/overview.puml"), (cross_b, "pkg_b/overview.puml")],
        cross_plan.staged,
    )
    loadingtest.equals(
        env,
        "cross_package_same_basename_wrappers",
        [(cross_a, "pkg_a", "overview"), (cross_b, "pkg_b", "overview")],
        cross_plan.wrappers,
    )

    # --- duplicate-path error: two files that would stage at the same path -----

    dup_a = _fake_file("static/overview.puml")
    dup_b = _fake_file("static/overview.puml")
    dup_plan = plan_view_layout([dup_a, dup_b], "")

    loadingtest.equals(
        env,
        "duplicate_path_error",
        ["two files would both stage as 'static/overview.puml': 'static/overview.puml' and 'static/overview.puml'"],
        dup_plan.errors,
    )
    loadingtest.equals(
        env,
        "duplicate_path_staged_once",
        [(dup_a, "static/overview.puml")],
        dup_plan.staged,
    )

    # --- duplicate-path detection also covers non-navigable assets --------------
    # Assets get no wrapper/index of their own, but they are still staged, so
    # an undetected collision would surface as a raw declare_file() conflict.

    asset_a = _fake_file("static/diagram.svg")
    asset_b = _fake_file("static/diagram.svg")
    asset_plan = plan_view_layout([asset_a, asset_b], "")

    loadingtest.equals(
        env,
        "duplicate_asset_path_error",
        ["two files would both stage as 'static/diagram.svg': 'static/diagram.svg' and 'static/diagram.svg'"],
        asset_plan.errors,
    )

    # --- an authored index.rst collides with a literal index.rst.inc ------------
    # The authored index is staged under the ".inc" name, which a source file
    # may already occupy.

    inc_authored = _fake_file("static/index.rst")
    inc_literal = _fake_file("static/index.rst.inc")
    inc_plan = plan_view_layout([inc_authored, inc_literal], "")

    loadingtest.equals(
        env,
        "plan_index_collides_with_literal_inc",
        ["two files would both stage as 'static/index.rst.inc': 'static/index.rst' and 'static/index.rst.inc'"],
        inc_plan.errors,
    )

    # --- a diagram literally named "index" is rejected --------------------------
    # That stem is reserved for the directory's own generated navigation page.

    reserved_index_puml = _fake_file("static/index.puml")
    reserved_plan = plan_view_layout([reserved_index_puml], "")

    loadingtest.equals(
        env,
        "diagram_named_index_rejected",
        ["'static/index.puml' is named 'index', which is reserved for the generated directory navigation page; rename it"],
        reserved_plan.errors,
    )

    # --- same-stem .puml and .plantuml collide -----------------------------------

    same_stem_puml = _fake_file("static/overview.puml")
    same_stem_plantuml = _fake_file("static/overview.plantuml")
    same_stem_plan = plan_view_layout([same_stem_puml, same_stem_plantuml], "")

    loadingtest.equals(
        env,
        "puml_and_plantuml_same_stem_rejected",
        ["both 'overview.puml' and 'overview.plantuml' exist for 'overview' in directory 'static'; keep only one"],
        same_stem_plan.errors,
    )

    # --- a file from another repository is rejected ------------------------------
    # short_path for an external-repo file starts with "../"; there is no
    # in-tree relative path to stage it at.

    external_repo_file = _fake_file("../other_repo+/pkg/overview.puml")
    external_repo_plan = plan_view_layout([external_repo_file], "")

    loadingtest.equals(
        env,
        "external_repo_file_rejected",
        ["'../other_repo+/pkg/overview.puml' lives outside this repository; architectural_design view files must live in the same repository as the target"],
        external_repo_plan.errors,
    )

    # --- own-repo file is accepted even when it isn't the build's main repo -----
    # Bazel's short_path prefixes every file living outside the build's *main*
    # repository with "../<repo>/", even when that repository is the SAME one
    # the consuming architectural_design target itself lives in (e.g. this
    # target is built as someone else's dependency, not as the build's root
    # module). own_repo lets relative_source_path() tell that apart from a
    # genuinely different repository -- see relative_source_path's docstring.

    own_repo_overview = _fake_file("../some_other_library+/overview.puml", owner_repo = "some_other_library+")
    own_repo_plan = plan_view_layout([own_repo_overview], "", own_repo = "some_other_library+")

    loadingtest.equals(env, "own_repo_marker_stripped_no_errors", [], own_repo_plan.errors)
    loadingtest.equals(
        env,
        "own_repo_marker_stripped_staged",
        [(own_repo_overview, "overview.puml")],
        own_repo_plan.staged,
    )

    # --- a genuinely different repository is still rejected with own_repo set --

    other_repo_overview = _fake_file("../other_repo+/overview.puml", owner_repo = "other_repo+")
    other_repo_plan = plan_view_layout([other_repo_overview], "", own_repo = "some_other_library+")

    loadingtest.equals(
        env,
        "different_repo_still_rejected_with_own_repo_set",
        ["'../other_repo+/overview.puml' lives outside this repository; architectural_design view files must live in the same repository as the target"],
        other_repo_plan.errors,
    )
