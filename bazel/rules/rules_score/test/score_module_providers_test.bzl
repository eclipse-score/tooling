# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
"""Tests for sphinx_module providers and two-phase build system."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load("@score_tooling//bazel/rules/rules_score:providers.bzl", "SphinxModuleInfo", "SphinxNeedsInfo")

# ============================================================================
# SphinxModuleInfo Provider Tests
# ============================================================================

def _sphinx_module_info_fields_test_impl(ctx):
    """Test that SphinxModuleInfo provides all required fields."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        SphinxModuleInfo in target_under_test,
        "Target should provide SphinxModuleInfo",
    )

    score_info = target_under_test[SphinxModuleInfo]

    # Verify html_dir field
    asserts.true(
        env,
        hasattr(score_info, "html_dir"),
        "SphinxModuleInfo should have html_dir field",
    )

    asserts.true(
        env,
        score_info.html_dir != None,
        "html_dir should not be None",
    )

    return analysistest.end(env)

sphinx_module_info_fields_test = analysistest.make(_sphinx_module_info_fields_test_impl)

def _transitive_modules_dedup_test_impl(ctx):
    """Regression test: a module reachable via multiple dependency paths (a
    diamond — module_d_lib is a dep of both module_b_lib and module_c_lib,
    never directly of module_a_lib) must appear exactly once in
    transitive_modules, not once per path that reaches it.
    """
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    score_info = target_under_test[SphinxModuleInfo]
    transitive_modules = score_info.transitive_modules.to_list()

    matches = [m for m in transitive_modules if m.name == "module_d_lib"]
    asserts.equals(
        env,
        1,
        len(matches),
        "module_d_lib is reachable via two paths (module_b_lib and " +
        "module_c_lib) but must appear exactly once in transitive_modules, " +
        "got: %s" % matches,
    )

    return analysistest.end(env)

transitive_modules_dedup_test = analysistest.make(_transitive_modules_dedup_test_impl)

# ============================================================================
# SphinxNeedsInfo Provider Tests
# ============================================================================

def _score_needs_info_fields_test_impl(ctx):
    """Test that SphinxNeedsInfo provides all required fields."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        SphinxNeedsInfo in target_under_test,
        "Needs target should provide SphinxNeedsInfo",
    )

    needs_info = target_under_test[SphinxNeedsInfo]

    # Verify needs_json_file field (direct file)
    asserts.true(
        env,
        hasattr(needs_info, "needs_json_file"),
        "SphinxNeedsInfo should have needs_json_file field",
    )

    asserts.true(
        env,
        needs_info.needs_json_file != None,
        "needs_json_file should not be None",
    )

    # Verify needs_json_files field (transitive depset)
    asserts.true(
        env,
        hasattr(needs_info, "needs_json_files"),
        "SphinxNeedsInfo should have needs_json_files field",
    )

    asserts.true(
        env,
        needs_info.needs_json_files != None,
        "needs_json_files should not be None",
    )

    # Verify it's a depset
    asserts.true(
        env,
        type(needs_info.needs_json_files) == type(depset([])),
        "needs_json_files should be a depset",
    )

    return analysistest.end(env)

score_needs_info_fields_test = analysistest.make(_score_needs_info_fields_test_impl)

def _score_needs_transitive_collection_test_impl(ctx):
    """Test that needs.json files are collected transitively."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    needs_info = target_under_test[SphinxNeedsInfo]

    # Get the list of transitive needs files
    transitive_needs = needs_info.needs_json_files.to_list()

    # Should have at least the direct needs file
    asserts.true(
        env,
        len(transitive_needs) >= 1,
        "Should have at least the direct needs.json file",
    )

    # Direct file should be in the transitive set
    direct_file = needs_info.needs_json_file
    asserts.true(
        env,
        direct_file in transitive_needs,
        "Direct needs.json file should be in transitive collection",
    )

    return analysistest.end(env)

score_needs_transitive_collection_test = analysistest.make(_score_needs_transitive_collection_test_impl)

def _score_needs_with_deps_test_impl(ctx):
    """Test that needs.json files include dependencies."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    needs_info = target_under_test[SphinxNeedsInfo]
    transitive_needs = needs_info.needs_json_files.to_list()

    # Module with dependencies should have multiple needs files
    # (its own + dependencies)
    asserts.true(
        env,
        len(transitive_needs) >= 1,
        "Module with dependencies should collect transitive needs.json files",
    )

    return analysistest.end(env)

score_needs_with_deps_test = analysistest.make(_score_needs_with_deps_test_impl)

# ============================================================================
# Two-Phase Build Tests
# ============================================================================

def _two_phase_needs_first_test_impl(ctx):
    """Test that Phase 1 (needs generation) works independently."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Verify SphinxNeedsInfo provider
    asserts.true(
        env,
        SphinxNeedsInfo in target_under_test,
        "Phase 1 should provide SphinxNeedsInfo",
    )

    # Verify DefaultInfo with needs.json output
    asserts.true(
        env,
        DefaultInfo in target_under_test,
        "Phase 1 should provide DefaultInfo",
    )

    default_info = target_under_test[DefaultInfo]
    files = default_info.files.to_list()

    # Should have at least one file (needs.json)
    asserts.true(
        env,
        len(files) >= 1,
        "Phase 1 should output needs.json file",
    )

    return analysistest.end(env)

two_phase_needs_first_test = analysistest.make(_two_phase_needs_first_test_impl)

def _two_phase_html_second_test_impl(ctx):
    """Test that Phase 2 (HTML generation) works with needs from Phase 1."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    # Verify SphinxModuleInfo provider
    asserts.true(
        env,
        SphinxModuleInfo in target_under_test,
        "Phase 2 should provide SphinxModuleInfo",
    )

    score_info = target_under_test[SphinxModuleInfo]

    # Verify HTML output
    asserts.true(
        env,
        score_info.html_dir != None,
        "Phase 2 should generate HTML directory",
    )

    return analysistest.end(env)

two_phase_html_second_test = analysistest.make(_two_phase_html_second_test_impl)

# ============================================================================
# Config Generation Tests
# ============================================================================

def _config_auto_generation_test_impl(ctx):
    """Test that conf.py is auto-generated when not provided."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    score_info = target_under_test[SphinxModuleInfo]

    # Module without explicit config should still build
    asserts.true(
        env,
        score_info.html_dir != None,
        "Auto-generated config should allow HTML generation",
    )

    return analysistest.end(env)

config_auto_generation_test = analysistest.make(_config_auto_generation_test_impl)

def _config_explicit_usage_test_impl(ctx):
    """Test that explicit conf.py is used when provided."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    score_info = target_under_test[SphinxModuleInfo]

    # Module with explicit config should build
    asserts.true(
        env,
        score_info.html_dir != None,
        "Explicit config should allow HTML generation",
    )

    return analysistest.end(env)

config_explicit_usage_test = analysistest.make(_config_explicit_usage_test_impl)

# ============================================================================
# Dependency Handling Tests
# ============================================================================

def _deps_html_merging_test_impl(ctx):
    """Test that HTML from dependencies is merged into output."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    score_info = target_under_test[SphinxModuleInfo]

    # Module with dependencies should generate merged HTML
    asserts.true(
        env,
        score_info.html_dir != None,
        "Module with dependencies should generate merged HTML",
    )

    return analysistest.end(env)

deps_html_merging_test = analysistest.make(_deps_html_merging_test_impl)

def _deps_needs_collection_test_impl(ctx):
    """Test that needs from dependencies are collected."""
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    needs_info = target_under_test[SphinxNeedsInfo]
    transitive_needs = needs_info.needs_json_files.to_list()

    # Should collect needs from dependencies
    asserts.true(
        env,
        len(transitive_needs) >= 1,
        "Should collect needs.json from dependencies",
    )

    return analysistest.end(env)

deps_needs_collection_test = analysistest.make(_deps_needs_collection_test_impl)

# ============================================================================
# Toolchain Resolution Tests
# ============================================================================

def _score_toolchain_override_test_impl(ctx):
    """Test that this module's own registered toolchain wins over the default.

    This module (rules_score/test/MODULE.bazel) registers `//:score_toolchain`
    as a root-module toolchain, which must take precedence over score_tooling's
    default registration. Verified by inspecting the executable of the
    SphinxHtmlBuild action rather than a provider field, since which toolchain
    won is not itself exposed as a provider.
    """
    env = analysistest.begin(ctx)

    actions = analysistest.target_actions(env)
    html_build_actions = [a for a in actions if a.mnemonic == "SphinxHtmlBuild"]
    asserts.equals(env, 1, len(html_build_actions), "Expected exactly one SphinxHtmlBuild action")

    executable_path = html_build_actions[0].argv[0]
    asserts.true(
        env,
        "score_toolchain_binary" in executable_path,
        "Expected the locally-registered //:score_toolchain to be selected " +
        "(executable path should contain 'score_toolchain_binary'), got: %s" % executable_path,
    )
    asserts.false(
        env,
        "raw_build" in executable_path,
        "score_tooling's default toolchain binary ('raw_build') should not " +
        "have been selected when this module registers its own " +
        "//:score_toolchain, got: %s" % executable_path,
    )

    return analysistest.end(env)

score_toolchain_override_test = analysistest.make(_score_toolchain_override_test_impl)

# ============================================================================
# Persistent Worker Gating Tests
# ============================================================================
#
# Bazel's Starlark Action object has no execution_requirements/execution_info
# field (verified empirically: accessing it raises "'Action' value has no
# field or method 'execution_requirements'"), so these can't assert on the
# worker exec-requirements dict directly. They instead assert on the argv
# side effect of the same worker_enabled boolean that drives it (see
# _add_sphinx_args and _worker_execution_requirements in sphinx_module.bzl):
# "--jobs auto" is present exactly when worker_enabled is False.

def _find_flag_value(argv, flag):
    for i, arg in enumerate(argv):
        if arg == flag and i + 1 < len(argv):
            return argv[i + 1]
    return None

def _worker_gating_html_test_impl(ctx):
    """Test that the HTML pass drops --jobs auto under allow_persistent_workers.

    See worker_enabled in sphinx_module.bzl's _add_sphinx_args. Also checks
    --doctree-dir is suffixed with the builder name.
    """
    env = analysistest.begin(ctx)

    actions = analysistest.target_actions(env)
    html_build_actions = [a for a in actions if a.mnemonic == "SphinxHtmlBuild"]
    asserts.equals(env, 1, len(html_build_actions), "Expected exactly one SphinxHtmlBuild action")
    argv = html_build_actions[0].argv

    asserts.false(
        env,
        "--jobs" in argv,
        "HTML pass with allow_persistent_workers=True must not pass --jobs " +
        "auto (parallel read inside a long-lived worker process is untested); got argv: %s" % argv,
    )

    doctree_dir = _find_flag_value(argv, "--doctree-dir")
    asserts.true(
        env,
        doctree_dir != None and doctree_dir.endswith("_html_doctrees"),
        "--doctree-dir must be suffixed with the builder name ('_html_doctrees'); got: %s" % doctree_dir,
    )

    return analysistest.end(env)

worker_gating_html_test = analysistest.make(_worker_gating_html_test_impl)

def _worker_gating_needs_test_impl(ctx):
    """Test that the needs pass keeps --jobs auto even under allow_persistent_workers.

    Its source dir is the real checkout, not a generated tree, so Worker's
    per-request write into srcdir (_bazel_worker_request_info.json) would
    otherwise land there. See worker_enabled in sphinx_module.bzl's
    _score_needs_impl.
    """
    env = analysistest.begin(ctx)

    actions = analysistest.target_actions(env)
    needs_build_actions = [a for a in actions if a.mnemonic == "SphinxNeedsBuild"]
    asserts.equals(env, 1, len(needs_build_actions), "Expected exactly one SphinxNeedsBuild action")
    argv = needs_build_actions[0].argv

    asserts.equals(
        env,
        "auto",
        _find_flag_value(argv, "--jobs"),
        "needs pass must always pass --jobs auto, regardless of " +
        "allow_persistent_workers, since it never runs as a worker; got argv: %s" % argv,
    )

    doctree_dir = _find_flag_value(argv, "--doctree-dir")
    asserts.true(
        env,
        doctree_dir != None and doctree_dir.endswith("_needs_doctrees"),
        "--doctree-dir must be suffixed with the builder name ('_needs_doctrees'); got: %s" % doctree_dir,
    )

    return analysistest.end(env)

worker_gating_needs_test = analysistest.make(_worker_gating_needs_test_impl)

# ============================================================================
# Test Suite
# ============================================================================

def sphinx_module_providers_test_suite(name):
    """Create a test suite for sphinx_module providers and build phases.

    Tests cover:
    - Transitive needs.json collection
    - Dependency handling (HTML merging, needs collection)

    Args:
        name: Name of the test suite
    """

    native.test_suite(
        name = name,
        tests = [
            # Provider field tests
            ":sphinx_module_info_fields_test",
            ":transitive_modules_dedup_test",
            ":score_needs_info_fields_test",
            ":score_needs_transitive_collection_test",

            # Provider tests
            ":score_needs_with_deps_test",

            # Two-phase build tests
            ":two_phase_needs_first_test",
            ":two_phase_html_second_test",

            # Config tests
            ":config_auto_generation_test",
            ":config_explicit_usage_test",

            # Dependency tests
            ":deps_html_merging_test",
            ":deps_needs_collection_test",

            # Toolchain resolution tests
            ":score_toolchain_override_test",

            # Persistent worker gating tests
            ":worker_gating_html_test",
            ":worker_gating_needs_test",
        ],
    )
