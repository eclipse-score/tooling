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
"""Bazel rules for AI-powered artefact testing.

Provides rules for analyzing TRLC requirements and architectural designs
against engineering guidelines using an AI checker.
"""

load("@trlc//:trlc.bzl", "TrlcProviderInfo")
load("//bazel/rules/rules_score:providers.bzl", "ArchitecturalDesignInfo")

# ============================================================================
# Shared implementation
# ============================================================================

def _run_ai_analysis(ctx, analysis_files, all_input_files, input_dirs, dep_dirs):
    """Common implementation for all AI artefact analysis test rules.

    Args:
        ctx: Rule context.
        analysis_files: Files to analyze (direct inputs).
        all_input_files: All files needed as action inputs (incl. deps for resolution).
        input_dirs: Dict of directories containing analysis files.
        dep_dirs: Dict of dependency directories (for link resolution).

    Returns:
        List of providers (DefaultInfo).
    """
    if not analysis_files:
        fail("No artefact files found for analysis")

    # Declare outputs
    html_report = ctx.actions.declare_file("{}_analysis.html".format(ctx.attr.name))
    json_report = ctx.actions.declare_file("{}_analysis.json".format(ctx.attr.name))
    guidelines_output_dir = ctx.actions.declare_directory("guidelines")
    debug_log = ctx.actions.declare_file("{}_debug.log".format(ctx.attr.name))

    # Collect guideline files from the filegroup
    guideline_files = ctx.files.guidelines

    # Determine input and guidelines directories
    input_dir = analysis_files[0].dirname
    guidelines_dir = guideline_files[0].dirname if guideline_files else None

    # Build arguments for the orchestrator
    args = ctx.actions.args()
    args.add("--input", input_dir)

    for dep_dir in dep_dirs.keys():
        args.add("--deps", dep_dir)

    for extra_dir in input_dirs.keys():
        if extra_dir != input_dir:
            args.add("--deps", extra_dir)

    args.add("--output", json_report.path)
    args.add("--html", html_report.path)
    args.add("--guidelines-output", guidelines_output_dir.path)

    if guidelines_dir:
        args.add("--guidelines", guidelines_dir)

    if ctx.attr.model:
        args.add("--model", ctx.attr.model)

    if ctx.attr._batch_size > 0:
        args.add("--batch-size", str(ctx.attr._batch_size))

    # NOTE: --cache is intentionally NOT passed.  Bazel actions are
    # already cached by Bazel's action cache; an additional Python-level
    # cache would break hermeticity.  The --cache flag is only available
    # for direct CLI invocations (python orchestrator.py --cache <dir>).

    # Prepare action inputs (include custom ai_model if provided)
    action_inputs = all_input_files + guideline_files
    if ctx.attr._custom_ai_model:
        custom_ai_model_file = ctx.attr._custom_ai_model[DefaultInfo].files.to_list()
        if custom_ai_model_file:
            action_inputs.extend(custom_ai_model_file)
            args.add("--custom-ai-model", custom_ai_model_file[0].path)

    # Add debug log output for Bazel output_groups
    args.add("--debug-log", debug_log.path)
    args.add("--verbose")

    ctx.actions.run(
        executable = ctx.executable._orchestrator,
        inputs = depset(direct = action_inputs),
        outputs = [json_report, html_report, guidelines_output_dir, debug_log],
        arguments = [args],
        progress_message = "Analyzing artefacts with AI for {}".format(ctx.attr.name),
        # NOTE: no-sandbox is required because the GitHub Copilot CLI needs
        # outbound network access (api.github.com) and the user's $HOME
        # directory (for stored OAuth credentials), both of which Bazel's
        # sandbox blocks.  This is an inherent trade-off of using an external
        # AI service from a Bazel action.  Hermeticity is partially preserved
        # by Bazel's own action-cache keying on declared inputs.
        execution_requirements = {"no-sandbox": "1"},
        use_default_shell_env = True,
    )

    # Test executable — validates the JSON report score against the threshold
    test_executable = ctx.actions.declare_file("{}_test_executable".format(ctx.attr.name))

    command = """#!/bin/bash
set -e
set -o pipefail

json_path="{json}"

if [ ! -f "$json_path" ] || [ ! -s "$json_path" ]; then
    echo "ERROR: JSON report was not generated or is empty"
    exit 1
fi

average=$(python3 -c "
import json, pathlib, sys
data = json.loads(pathlib.Path(sys.argv[1]).read_text())
scores = [a['score'] for a in data.get('analyses', [])]
print(f'{{sum(scores)/len(scores):.2f}}' if scores else '0')
" "$json_path")

threshold="{threshold}"

if (( $(echo "$average >= $threshold" | bc -l) )); then
    echo "AI analysis complete. Average score: $average (threshold: $threshold)"
    exit 0
else
    echo "ERROR: Average score $average is below threshold $threshold"
    exit 1
fi
""".format(json = json_report.short_path, threshold = ctx.attr.score_threshold)

    ctx.actions.write(
        output = test_executable,
        content = command,
        is_executable = True,
    )

    return [
        DefaultInfo(
            runfiles = ctx.runfiles(
                files = [json_report, html_report, guidelines_output_dir],
            ),
            files = depset([json_report, html_report, guidelines_output_dir]),
            executable = test_executable,
        ),
        OutputGroupInfo(
            debug = depset([debug_log]),
        ),
    ]

# Attributes shared by all AI test rules
_COMMON_AI_TEST_ATTRS = {
    "model": attr.string(
        doc = "AI model name to use for analysis.",
        default = "anthropic/claude-sonnet-4-5",
    ),
    "score_threshold": attr.string(
        doc = "Minimum average score required to pass the test (0-10).",
        default = "0.0",
    ),
    "_batch_size": attr.int(
        doc = "Number of artefacts to process per batch (0 = all at once).",
        default = 0,
    ),
    "_custom_ai_model": attr.label(
        doc = "Custom ai_model.py file (optional, provided by consumer repo).",
        default = None,
        allow_single_file = [".py"],
    ),
    "_orchestrator": attr.label(
        doc = "Orchestrator binary.",
        default = "//validation/ai_checker:orchestrator",
        executable = True,
        cfg = "exec",
    ),
}

# ============================================================================
# TRLC Requirements AI Test
# ============================================================================

def _trlc_requirements_ai_test_impl(ctx):
    """Extract TRLC artefacts from providers and delegate to shared analysis."""
    analysis_files = []
    all_files = []
    input_dirs = {}
    dep_dirs = {}

    for req in ctx.attr.reqs:
        trlc_provider = req[TrlcProviderInfo]

        direct_reqs = trlc_provider.reqs.to_list()
        analysis_files.extend(direct_reqs)
        for f in direct_reqs:
            input_dirs[f.dirname] = True

        dep_reqs = trlc_provider.deps.to_list()
        spec_files = trlc_provider.spec.to_list()
        all_files.extend(direct_reqs + dep_reqs + spec_files)
        for f in dep_reqs + spec_files:
            dep_dirs[f.dirname] = True

    return _run_ai_analysis(ctx, analysis_files, all_files, input_dirs, dep_dirs)

trlc_requirements_ai_test = rule(
    implementation = _trlc_requirements_ai_test_impl,
    attrs = dict(_COMMON_AI_TEST_ATTRS, **{
        "reqs": attr.label_list(
            doc = "Targets providing TrlcProviderInfo.",
            providers = [TrlcProviderInfo],
            mandatory = True,
        ),
        "guidelines": attr.label(
            doc = "Filegroup containing guideline markdown files.",
            default = "//validation/ai_checker:default_guidelines",
            allow_files = True,
        ),
    }),
    test = True,
    toolchains = [],
    fragments = ["platform"],
)

# ============================================================================
# Architecture AI Test
# ============================================================================

def _architecture_ai_test_impl(ctx):
    """Extract architecture artefacts from providers and delegate to shared analysis."""
    analysis_files = []
    input_dirs = {}

    for design in ctx.attr.designs:
        design_info = design[ArchitecturalDesignInfo]
        for f in design_info.static.to_list() + design_info.dynamic.to_list():
            analysis_files.append(f)
            input_dirs[f.dirname] = True

    return _run_ai_analysis(ctx, analysis_files, analysis_files, input_dirs, {})

architecture_ai_test = rule(
    implementation = _architecture_ai_test_impl,
    attrs = dict(_COMMON_AI_TEST_ATTRS, **{
        "designs": attr.label_list(
            doc = "Targets providing ArchitecturalDesignInfo.",
            providers = [ArchitecturalDesignInfo],
            mandatory = True,
        ),
        "guidelines": attr.label(
            doc = "Filegroup containing architecture guideline markdown files.",
            default = "//validation/ai_checker:default_architecture_guidelines",
            allow_files = True,
        ),
    }),
    test = True,
    toolchains = [],
    fragments = ["platform"],
)
