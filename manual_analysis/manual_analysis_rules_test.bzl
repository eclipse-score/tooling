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
"""Analysis tests for manual_analysis Bazel rules."""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")
load(
    "//manual_analysis:manual_analysis.bzl",
    "ManualAnalysisContextInfo",
)

def _has_short_path(files, suffix):
    for file_obj in files:
        if file_obj.short_path.endswith(suffix):
            return True
    return False

def _has_rule_label(rules, expected):
    for rule in rules:
        label = rule.split("\t", 1)[0]
        if label == expected or label.endswith(expected):
            return True
    return False

def _get_rule_canonical_form(rules, expected):
    for rule in rules:
        label, canonical_form = rule.split("\t", 1)
        if label == expected or label.endswith(expected):
            return canonical_form
    return None

def _context_from_filegroup_provider_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        ManualAnalysisContextInfo in target_under_test,
        "Target should provide ManualAnalysisContextInfo",
    )

    info = target_under_test[ManualAnalysisContextInfo]
    files = info.files.to_list()
    rules = info.rules.to_list()
    asserts.true(
        env,
        _has_short_path(files, "manual_analysis/example/context_a.txt"),
        "Context provider should contain context_a.txt",
    )
    asserts.true(
        env,
        _has_short_path(files, "manual_analysis/example/context_b.txt"),
        "Context provider should contain context_b.txt",
    )
    asserts.true(
        env,
        len(rules) == 0,
        "Filegroup context should not emit hashed rule metadata",
    )

    return analysistest.end(env)

context_from_filegroup_provider_test = analysistest.make(_context_from_filegroup_provider_test_impl)

def _context_from_cc_library_provider_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        ManualAnalysisContextInfo in target_under_test,
        "Target should provide ManualAnalysisContextInfo",
    )

    info = target_under_test[ManualAnalysisContextInfo]
    files = info.files.to_list()
    rules = info.rules.to_list()
    asserts.true(
        env,
        _has_short_path(files, "manual_analysis/example/ma_cc_root.cc"),
        "CC context should include root source file",
    )
    asserts.true(
        env,
        _has_short_path(files, "manual_analysis/example/ma_cc_dep.h"),
        "CC context should include transitive header file",
    )

    asserts.true(
        env,
        len(rules) >= 2,
        "CC context should include hashed metadata for direct and transitive libraries",
    )
    asserts.true(
        env,
        _has_rule_label(rules, "//manual_analysis/example:ma_cc_root"),
        "CC context should include canonical form for ma_cc_root",
    )
    asserts.true(
        env,
        _has_rule_label(rules, "//manual_analysis/example:ma_cc_dep"),
        "CC context should include canonical form for ma_cc_dep",
    )
    root_canonical_form = _get_rule_canonical_form(rules, "//manual_analysis/example:ma_cc_root")
    asserts.true(
        env,
        root_canonical_form != None,
        "CC context should expose the root canonical form",
    )
    asserts.true(
        env,
        root_canonical_form.startswith("{"),
        "CC context should serialize rule metadata as JSON",
    )
    asserts.true(
        env,
        '"name":"ma_cc_root"' in root_canonical_form,
        "CC context JSON should include the rule name",
    )
    asserts.true(
        env,
        '"alwayslink":false' in root_canonical_form,
        "CC context JSON should encode defaulted boolean attrs deterministically",
    )

    return analysistest.end(env)

context_from_cc_library_provider_test = analysistest.make(_context_from_cc_library_provider_test_impl)

def _manual_analysis_update_env_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        RunEnvironmentInfo in target_under_test,
        "manual_analysis_update target should provide RunEnvironmentInfo",
    )

    run_env = target_under_test[RunEnvironmentInfo].environment
    asserts.true(env, "MANUAL_ANALYSIS_FILES_MANIFEST" in run_env, "Missing files manifest env")
    asserts.true(env, "MANUAL_ANALYSIS_RULES_MANIFEST" in run_env, "Missing rules manifest env")
    asserts.true(env, "MANUAL_ANALYSIS_LOCK_FILE" in run_env, "Missing lock file env")
    asserts.true(env, "MANUAL_ANALYSIS_YAML" in run_env, "Missing analysis yaml env")
    asserts.true(env, "MANUAL_ANALYSIS_RESULTS_FILE" in run_env, "Missing results file env")

    default_files = target_under_test[DefaultInfo].files.to_list()
    asserts.true(
        env,
        len(default_files) == 1,
        "manual_analysis_update should expose one lock file output",
    )
    asserts.true(
        env,
        default_files[0].basename.endswith("_lockfile.txt"),
        "manual_analysis_update should expose generated lockfile symlink",
    )

    return analysistest.end(env)

manual_analysis_update_env_test = analysistest.make(_manual_analysis_update_env_test_impl)
manual_analysis_macro_update_env_test = manual_analysis_update_env_test

def _manual_analysis_test_env_test_impl(ctx):
    env = analysistest.begin(ctx)
    target_under_test = analysistest.target_under_test(env)

    asserts.true(
        env,
        RunEnvironmentInfo in target_under_test,
        "manual_analysis_test target should provide RunEnvironmentInfo",
    )

    run_env = target_under_test[RunEnvironmentInfo].environment
    asserts.true(env, "MANUAL_ANALYSIS_COMPUTED_LOCK" in run_env, "Missing computed lock env")
    asserts.true(env, "MANUAL_ANALYSIS_COMMITTED_LOCK" in run_env, "Missing committed lock env")
    asserts.true(env, "MANUAL_ANALYSIS_YAML" in run_env, "Missing analysis yaml env")
    asserts.true(env, "MANUAL_ANALYSIS_RESULTS_FILE" in run_env, "Missing results file env")
    asserts.true(env, "MANUAL_ANALYSIS_LABEL" in run_env, "Missing analysis label env")

    default_info = target_under_test[DefaultInfo]
    executable = default_info.files_to_run.executable
    asserts.true(
        env,
        executable != None,
        "manual_analysis_test should produce a test executable",
    )
    asserts.true(
        env,
        executable.basename.endswith("manual_analysis_test_runner"),
        "manual_analysis_test should execute the unified Python test runner",
    )

    default_runfiles = default_info.default_runfiles.files.to_list()
    asserts.true(
        env,
        _has_short_path(default_runfiles, "manual_analysis/example/manual_analysis.lobster"),
        "manual_analysis_test should include the generated lobster artifact in runfiles",
    )

    return analysistest.end(env)

manual_analysis_test_env_test = analysistest.make(_manual_analysis_test_env_test_impl)
manual_analysis_macro_test_env_test = manual_analysis_test_env_test

def manual_analysis_rules_test_suite(name):
    native.test_suite(
        name = name,
        tests = [
            ":context_from_filegroup_provider_test",
            ":context_from_cc_library_provider_test",
            ":manual_analysis_macro_update_env_test",
            ":manual_analysis_macro_test_env_test",
        ],
    )
