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
"""Bazel rule to create a manual analysis
A manual analysis always consists of a context on which the analysis applies and
the steps of the analysis.
This rule adds a lockfile that makes sure that a manual analysis is redone when
the context changes.
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

_INTERACTIVE_RUNNER_ATTR = {
    "_interactive_runner": attr.label(
        doc = "Standalone interactive manual analysis runner executable.",
        default = "//manual_analysis:interactive_runner",
        executable = True,
        cfg = "exec",
    ),
}

_CHECK_LOCK_ATTR = {
    "_check_lock": attr.label(
        doc = "Standalone check_lock executable.",
        default = "//manual_analysis:check_lock",
        executable = True,
        cfg = "exec",
    ),
}

_CHECK_RESULTS_ATTR = {
    "_check_results": attr.label(
        doc = "Standalone check_results executable.",
        default = "//manual_analysis:check_results",
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
    runfiles = runfiles.merge(ctx.attr._interactive_runner[DefaultInfo].default_runfiles)
    runfiles = runfiles.merge(ctx.attr._update_lock[DefaultInfo].default_runfiles)

    launcher = ctx.actions.declare_file("{}.sh".format(ctx.label.name))
    ctx.actions.write(
        output = launcher,
        is_executable = True,
        content = "\n".join([
            "#!/usr/bin/env bash",
            "set -euo pipefail",
            'RUNFILES_DIR="${RUNFILES_DIR:-$0.runfiles}"',
            '"${{RUNFILES_DIR}}/_main/{}" "$@"'.format(ctx.executable._interactive_runner.short_path),
            '"${{RUNFILES_DIR}}/_main/{}"'.format(ctx.executable._update_lock.short_path),
            "",
        ]),
    )

    return [
        DefaultInfo(
            executable = launcher,
            files = depset([lock_file]),
            runfiles = runfiles,
        ),
        RunEnvironmentInfo(environment = {
            "MANUAL_ANALYSIS_FILES_MANIFEST": manifests[0].path,
            "MANUAL_ANALYSIS_RULES_MANIFEST": manifests[1].path,
            "MANUAL_ANALYSIS_LOCK_FILE": lock_file.short_path,
            "MANUAL_ANALYSIS_YAML": analysis_file.path,
            "MANUAL_ANALYSIS_RESULTS_FILE": results_file.path,
        }),
    ]

manual_analysis_update = rule(
    doc = """
    Updates the lockfile to signify that the manual analysis is not up-to-date with the context
    """,
    implementation = _manual_analysis_update_impl,
    attrs = dict(_COMMON_ATTRS, **dict(_UPDATE_LOCK_ATTR, **_INTERACTIVE_RUNNER_ATTR)),
    executable = True,
)

# =============================================================================
# Rule: manual_analysis_test
# =============================================================================

def _manual_analysis_test_impl(ctx):
    input_files, input_rules = _collect_inputs(ctx)
    lock_file = ctx.attr.lock_file[DefaultInfo].files.to_list()[0]
    results_file = ctx.attr.results_file[DefaultInfo].files.to_list()[0]
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

    test_exe = ctx.actions.declare_file("{}_check_manual_analysis".format(ctx.label.name))
    ctx.actions.write(
        output = test_exe,
        is_executable = True,
        content = "\n".join([
            "#!/usr/bin/env bash",
            "set -euo pipefail",
            'RUNFILES_DIR="${RUNFILES_DIR:-$0.runfiles}"',
            '"${{RUNFILES_DIR}}/_main/{}"'.format(ctx.executable._check_lock.short_path),
            '"${{RUNFILES_DIR}}/_main/{}"'.format(ctx.executable._check_results.short_path),
            "",
        ]),
    )

    runfiles = ctx.runfiles(files = [computed_lock, lock_file, results_file])
    runfiles = runfiles.merge(ctx.attr._check_lock[DefaultInfo].default_runfiles)
    runfiles = runfiles.merge(ctx.attr._check_results[DefaultInfo].default_runfiles)

    return [
        DefaultInfo(
            executable = test_exe,
            runfiles = runfiles,
        ),
        RunEnvironmentInfo(environment = {
            "MANUAL_ANALYSIS_COMPUTED_LOCK": computed_lock.short_path,
            "MANUAL_ANALYSIS_COMMITTED_LOCK": lock_file.short_path,
            "MANUAL_ANALYSIS_RESULTS_FILE": results_file.short_path,
        }),
    ]

manual_analysis_test = rule(
    doc = """
    Tests that a manual analysis is up-to-date with the context.
    """,
    implementation = _manual_analysis_test_impl,
    attrs = dict(
        _COMMON_ATTRS,
        **dict(
            _UPDATE_LOCK_ATTR,
            **dict(_CHECK_LOCK_ATTR, **_CHECK_RESULTS_ATTR)
        )
    ),
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
