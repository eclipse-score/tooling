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

load("@lobster//:lobster.bzl", "LobsterProvider")

"""Bazel rule to create a manual verification analysis.

A manual analysis consists of context (the implementation under review) and
verification steps. It is verification evidence that checks whether the
implementation gathered by the context satisfies the referenced requirements.
This rule adds a lockfile that ensures manual verification is redone when the
context changes.
"""

# =============================================================================
# Context provider
# =============================================================================

ManualAnalysisContextInfo = provider(
    doc = "Context inputs for manual analysis lock-file computation.",
    fields = {
        "files": "depset of files that define the context for the manual analysis",
        "rules": "depset of serialized <label>\\t<canonical_form> entries for relevant attributes",
    },
)

# =============================================================================
# Shared helpers
# =============================================================================

def _collect_inputs(ctx):
    """Collect all input files from context providers and analysis."""
    context_depsets = []
    rules = []
    for target in ctx.attr.contexts:
        info = target[ManualAnalysisContextInfo]
        context_depsets.append(info.files)
        rules.append(info.rules)
    analysis_files = ctx.attr.analysis[DefaultInfo].files
    results_files = ctx.attr.results_file[DefaultInfo].files
    return [depset(transitive = context_depsets + [analysis_files, results_files]).to_list(), rules]

def _make_manifests(ctx, input_files, input_rules):
    """Write two TSV manifests for *input_files* and *input_rules* and return the declared Files."""
    file_manifest = ctx.actions.declare_file("{}_file_manifest.tsv".format(ctx.label.name))
    ctx.actions.write(
        output = file_manifest,
        content = "\n".join(["{}\t{}".format(f.short_path, f.path) for f in input_files]) + "\n",
    )
    rule_manifest = ctx.actions.declare_file("{}_rule_manifest.tsv".format(ctx.label.name))

    # Flatten list of depsets into a single sorted list of serialized rule entries.
    flattened_rules = []
    for rule_set in input_rules:
        flattened_rules.extend(rule_set.to_list())
    flattened_rules = sorted(flattened_rules)
    ctx.actions.write(
        output = rule_manifest,
        content = "\n".join(flattened_rules) + "\n",
    )
    return [file_manifest, rule_manifest]

_UPDATE_LOCK_ATTR = {
    "_update_lock": attr.label(
        doc = "Standalone update_lock executable.",
        default = "//manual_analysis:update_lock",
        executable = True,
        cfg = "exec",
    ),
}

_UPDATE_RUNNER_ATTR = {
    "_manual_analysis_update_runner": attr.label(
        doc = "Unified update executable for interactive run and lock refresh.",
        default = "//manual_analysis:manual_analysis_update_runner",
        executable = True,
        cfg = "exec",
    ),
}

_TEST_RUNNER_ATTR = {
    "_manual_analysis_test_runner": attr.label(
        doc = "Unified test executable for lock/results checks and LOBSTER generation.",
        default = "//manual_analysis:manual_analysis_test_runner",
        executable = True,
        cfg = "exec",
    ),
}

_COMMON_ATTRS = {
    "contexts": attr.label_list(
        doc = "Context provider targets for the analysis",
        mandatory = True,
        providers = [ManualAnalysisContextInfo],
    ),
    "analysis": attr.label(
        doc = "Filegroup containing the analysis itself",
        allow_files = True,
        mandatory = True,
    ),
    "lock_file": attr.label(
        doc = "Lock file to safeguard the actuality of the analysis",
        mandatory = True,
        allow_single_file = True,
    ),
    "results_file": attr.label(
        doc = "Results file capturing interactive manual-analysis inputs",
        mandatory = True,
        allow_single_file = True,
    ),
}

# =============================================================================
# Rule: manual_analysis_update
# =============================================================================

def _manual_analysis_update_impl(ctx):
    input_files, input_rules = _collect_inputs(ctx)
    workspace_lock_file = ctx.attr.lock_file[DefaultInfo].files.to_list()[0]
    workspace_results_file = ctx.attr.results_file[DefaultInfo].files.to_list()[0]
    manifests = _make_manifests(ctx, input_files, input_rules)
    analysis_file = ctx.attr.analysis[DefaultInfo].files.to_list()[0]

    lock_file = ctx.actions.declare_file("{}_lockfile.txt".format(ctx.label.name[:-7]))
    ctx.actions.symlink(
        output = lock_file,
        target_file = workspace_lock_file,
    )

    results_file = ctx.actions.declare_file("{}_results.json".format(ctx.label.name[:-7]))
    ctx.actions.symlink(
        output = results_file,
        target_file = workspace_results_file,
    )

    runfiles = ctx.runfiles(files = input_files + manifests + [lock_file, results_file])
    runfiles = runfiles.merge(ctx.attr._manual_analysis_update_runner[DefaultInfo].default_runfiles)

    executable = ctx.actions.declare_file(
        "{}_manual_analysis_update_runner".format(ctx.label.name),
    )
    ctx.actions.symlink(
        output = executable,
        target_file = ctx.executable._manual_analysis_update_runner,
        is_executable = True,
    )

    return [
        DefaultInfo(
            executable = executable,
            files = depset([lock_file]),
            runfiles = runfiles,
        ),
        RunEnvironmentInfo(environment = {
            "MANUAL_ANALYSIS_FILES_MANIFEST": manifests[0].short_path,
            "MANUAL_ANALYSIS_RULES_MANIFEST": manifests[1].short_path,
            "MANUAL_ANALYSIS_LOCK_FILE": lock_file.short_path,
            "MANUAL_ANALYSIS_YAML": analysis_file.short_path,
            "MANUAL_ANALYSIS_RESULTS_FILE": results_file.short_path,
        }),
    ]

manual_analysis_update = rule(
    doc = """
    Updates the lockfile to signify that the manual analysis is not up-to-date with the context
    """,
    implementation = _manual_analysis_update_impl,
    attrs = dict(_COMMON_ATTRS, **_UPDATE_RUNNER_ATTR),
    executable = True,
)

# =============================================================================
# Rule: manual_analysis_test
# =============================================================================

def _manual_analysis_test_impl(ctx):
    input_files, input_rules = _collect_inputs(ctx)
    lock_file = ctx.attr.lock_file[DefaultInfo].files.to_list()[0]
    results_file = ctx.attr.results_file[DefaultInfo].files.to_list()[0]
    analysis_file = ctx.attr.analysis[DefaultInfo].files.to_list()[0]
    manifests = _make_manifests(ctx, input_files, input_rules)

    computed_lock = ctx.actions.declare_file("{}_computed_lock.txt".format(ctx.label.name))
    ctx.actions.run(
        executable = ctx.executable._update_lock,
        inputs = depset(input_files + manifests),
        outputs = [computed_lock],
        env = {
            "MANUAL_ANALYSIS_FILES_MANIFEST": manifests[0].path,
            "MANUAL_ANALYSIS_RULES_MANIFEST": manifests[1].path,
            "MANUAL_ANALYSIS_OUTPUT": computed_lock.path,
        },
        mnemonic = "ManualAnalysisCompute",
        progress_message = "Computing analysis digest for {}".format(ctx.label),
    )

    lobster_file = ctx.actions.declare_file("{}.lobster".format(ctx.label.name))

    action_environment = {
        "MANUAL_ANALYSIS_COMPUTED_LOCK": computed_lock.path,
        "MANUAL_ANALYSIS_COMMITTED_LOCK": lock_file.path,
        "MANUAL_ANALYSIS_YAML": analysis_file.path,
        "MANUAL_ANALYSIS_RESULTS_FILE": results_file.path,
        "MANUAL_ANALYSIS_LOBSTER_OUTPUT": lobster_file.path,
        "MANUAL_ANALYSIS_LABEL": str(ctx.label),
    }

    test_environment = {
        "MANUAL_ANALYSIS_COMPUTED_LOCK": computed_lock.short_path,
        "MANUAL_ANALYSIS_COMMITTED_LOCK": lock_file.short_path,
        "MANUAL_ANALYSIS_YAML": analysis_file.short_path,
        "MANUAL_ANALYSIS_RESULTS_FILE": results_file.short_path,
        "MANUAL_ANALYSIS_LOBSTER_OUTPUT": "{}.lobster".format(ctx.label.name),
        "MANUAL_ANALYSIS_LABEL": str(ctx.label),
    }

    ctx.actions.run(
        executable = ctx.executable._manual_analysis_test_runner,
        inputs = [
            computed_lock,
            lock_file,
            analysis_file,
            results_file,
        ],
        arguments = ["--allow-check-failures"],
        outputs = [lobster_file],
        env = action_environment,
        mnemonic = "ManualAnalysisTestRunner",
        progress_message = "Checking manual analysis and generating LOBSTER for {}".format(ctx.label),
    )

    # Rule executables must be artifacts created by the same rule.
    test_executable = ctx.actions.declare_file(
        "{}_manual_analysis_test_runner".format(ctx.label.name),
    )
    ctx.actions.symlink(
        output = test_executable,
        target_file = ctx.executable._manual_analysis_test_runner,
        is_executable = True,
    )

    runfiles = ctx.runfiles(files = [computed_lock, lock_file, analysis_file, results_file, lobster_file])
    runfiles = runfiles.merge(ctx.attr._manual_analysis_test_runner[DefaultInfo].default_runfiles)

    return [
        DefaultInfo(
            executable = test_executable,
            runfiles = runfiles,
            files = depset([lobster_file, test_executable]),
        ),
        RunEnvironmentInfo(environment = test_environment),
        LobsterProvider(
            lobster_input = {lobster_file.basename: lobster_file.path},
        ),
    ]

manual_analysis_test = rule(
    doc = """
    Tests that a manual analysis is up-to-date with the context.
    """,
    implementation = _manual_analysis_test_impl,
    attrs = dict(_COMMON_ATTRS, **dict(_UPDATE_LOCK_ATTR, **_TEST_RUNNER_ATTR)),
    test = True,
)

def _manual_analysis_impl(name, visibility, contexts, analysis, lock_file, results_file, **kwargs):
    manual_analysis_update(
        name = name + ".update",
        contexts = contexts,
        analysis = analysis,
        lock_file = lock_file,
        results_file = results_file,
        visibility = visibility,
    )

    manual_analysis_test(
        name = name,
        contexts = contexts,
        analysis = analysis,
        lock_file = lock_file,
        results_file = results_file,
        visibility = visibility,
    )

manual_analysis = macro(
    doc = """
    Defines a manual analysis with context providers and an analysis file.
    A lock file is used to ensure that the manual analysis is in sync with the context.
    This macro creates two targets:
    - {name}.update
      Executable target to update the lock file after a manual analysis was redone
    - {name}
      Test target to verify that a manual analysis is still up to date

    This macro requires context provider targets that expose ManualAnalysisContextInfo.
    The lock file is computed from all files contained in those context providers
    and the analysis file itself.
    """,
    attrs = {
        "contexts": attr.label_list(
            doc = "Context providers for the analysis",
            mandatory = True,
            providers = [ManualAnalysisContextInfo],
        ),
        "analysis": attr.label(
            doc = "Filegroup containing the analysis itself",
            allow_files = True,
            mandatory = True,
        ),
        "lock_file": attr.label(
            doc = "Lock file to safeguard the actuality of the analysis",
            mandatory = True,
            allow_single_file = True,
            configurable = False,
        ),
        "results_file": attr.label(
            doc = "Results file to capture interactive manual-analysis outcomes",
            mandatory = True,
            allow_single_file = True,
            configurable = False,
        ),
    },
    implementation = _manual_analysis_impl,
)
