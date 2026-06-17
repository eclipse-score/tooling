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

# Default tags applied to every AI test. The AI analysis runs at test time and
# performs a non-hermetic network call to GitHub Copilot using credentials from
# the user's environment, so the test must escape Bazel's sandbox. "external"
# prevents result caching (the AI response is non-deterministic).
_AI_TEST_DEFAULT_TAGS = ["no-sandbox", "requires-network", "external"]

# Environment variables the test inherits from the invoking client environment.
# These are baked into the target via RunEnvironmentInfo so consumers do not
# need a --config=copilot / --test_env flag. HOME is essential: the test runner
# otherwise resets HOME to $TEST_TMPDIR (per the Bazel Test Encyclopedia),
# which hides the Copilot CLI's ~/.copilot/config.json credentials. The proxy
# variables are inherited so the call works behind a corporate proxy.
_AI_TEST_INHERITED_ENV = [
    "HOME",
    "COPILOT_GITHUB_TOKEN",
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "no_proxy",
]

def _shell_quote(value):
    if value == "":
        return "''"
    return "'" + value.replace("'", "'\"'\"'") + "'"

def _short_dir(f):
    """Return the runfiles-relative directory of a File (short_path dirname)."""
    sp = f.short_path
    if "/" in sp:
        return sp.rsplit("/", 1)[0]
    return "."

def _run_ai_analysis(ctx, analysis_files, all_input_files, dep_dirs, req_files = None, artefact_type = "requirements"):
    """Common implementation for all AI artefact analysis test rules.

    The AI analysis runs **at test time** (not as a build action): the test
    executable launches the orchestrator from the runfiles tree, so the
    non-hermetic network call and its credentials live in the test phase. The
    required environment is inherited automatically via RunEnvironmentInfo, and
    reports are written to the test's undeclared-outputs directory.

    Args:
        ctx: Rule context.
        analysis_files: Files to analyze (direct inputs).
        all_input_files: All files needed at runtime (incl. deps for resolution).
        dep_dirs: Dict of runfiles-relative dependency directories (link resolution).
        req_files: Optional list of individual files to register with TRLC instead
            of scanning the entire input directory. When set, only these files are
            parsed and they also define the grading scope, so requirements spread
            across several directories are all graded. Passing them explicitly
            also avoids picking up unreferenced files that may fail TRLC validation.
        artefact_type: "requirements" (TRLC) or "architecture" (raw PlantUML).

    Returns:
        List of providers (DefaultInfo).
    """
    if not analysis_files:
        fail("No artefact files found for analysis")

    # Collect guideline / context / project-guideline files.
    guideline_files = ctx.files.guidelines
    context_files = ctx.files.context
    project_guideline_files = ctx.files._project_guidelines

    # Build the orchestrator argument list. The orchestrator runs from the
    # runfiles root at test time, so every file is referenced by its
    # workspace-relative short_path.
    args = ["--artefact-type", artefact_type]

    for dep_dir in dep_dirs.keys():
        args += ["--deps", dep_dir]

    # When individual req files are provided, pass them explicitly. The
    # extractor then registers only those files (ignoring other files in the
    # same directory that may fail TRLC validation) and grades exactly that
    # set, so requirements located in different directories are all covered.
    if req_files:
        for f in req_files:
            args += ["--req-file", f.short_path]

    # For architecture, pass the raw PlantUML source files explicitly (the
    # orchestrator reads them as text).
    if artefact_type == "architecture":
        for f in analysis_files:
            args += ["--puml-file", f.short_path]

    # Background context (markdown / plantuml) injected as read-only material.
    for f in context_files:
        args += ["--context-file", f.short_path]

    # Project-specific guidelines (graded) layered on top of the general and
    # type guidelines. Sourced from a label_flag so a consumer repo can set
    # them once in .bazelrc instead of on every target.
    for f in project_guideline_files:
        args += ["--project-guidelines", f.short_path]

    # Pass each guideline file explicitly rather than a directory: the default
    # guideline sets span multiple directories (e.g. general.md plus a
    # requirements/ or architecture/ subdirectory) and the orchestrator scans a
    # guidelines directory non-recursively, so a single derived directory would
    # silently drop guidelines from the other directories.
    for f in guideline_files:
        args += ["--guidelines-file", f.short_path]

    if ctx.attr.model:
        args += ["--model", ctx.attr.model]

    if ctx.attr.batch_size > 0:
        args += ["--batch-size", str(ctx.attr.batch_size)]

    # NOTE: --cache is intentionally NOT passed. The --cache flag is only
    # available for direct CLI invocations (python orchestrator.py --cache).

    # Files that must be present in the test runfiles.
    runtime_files = (
        all_input_files + guideline_files + context_files + project_guideline_files
    )

    # Optional custom AI backend supplied by the consumer repo.
    if ctx.attr._custom_ai_model:
        custom_ai_model_files = ctx.attr._custom_ai_model[DefaultInfo].files.to_list()
        if custom_ai_model_files:
            runtime_files = runtime_files + custom_ai_model_files
            args += ["--custom-ai-model", custom_ai_model_files[0].short_path]

    # Generate the test launcher. Per the Bazel Test Encyclopedia, a test runs
    # with its working directory set to $TEST_SRCDIR/$TEST_WORKSPACE (the
    # runfiles root), so the workspace-relative artefact paths baked above
    # resolve directly and no runfiles probing is required. The orchestrator
    # writes its reports into $TEST_UNDECLARED_OUTPUTS_DIR itself, so the
    # launcher only has to exec it with the computed arguments.
    name = ctx.attr.name
    launcher_content = (
        "#!/usr/bin/env bash\n" +
        "set -euo pipefail\n\n" +
        "# A test starts in $TEST_SRCDIR/$TEST_WORKSPACE (the runfiles root), so\n" +
        "# the workspace-relative paths baked below resolve as-is. The\n" +
        "# orchestrator writes its reports into the test's undeclared-outputs\n" +
        "# directory automatically (Bazel zips it into\n" +
        "# bazel-testlogs/.../test.outputs/outputs.zip).\n" +
        "exec \"./" + ctx.executable._orchestrator.short_path + "\" \\\n" +
        "  " + " ".join([_shell_quote(a) for a in args]) + " \\\n" +
        "  --verbose \\\n" +
        "  --score-threshold " + _shell_quote(ctx.attr.score_threshold) + "\n"
    )

    launcher = ctx.actions.declare_file("{}_ai_test.sh".format(name))
    ctx.actions.write(
        output = launcher,
        content = launcher_content,
        is_executable = True,
    )

    # Everything the orchestrator needs at test time travels in the runfiles:
    # the orchestrator binary (+ its own runfiles) and all artefact / guideline
    # inputs.
    runfiles = ctx.runfiles(
        files = [ctx.executable._orchestrator] + runtime_files,
    ).merge(ctx.attr._orchestrator[DefaultInfo].default_runfiles)

    return [
        DefaultInfo(
            executable = launcher,
            runfiles = runfiles,
        ),
        # Bake the required environment-variable inheritance into the target so
        # `bazel test //...` works without a --config=copilot / --test_env flag.
        RunEnvironmentInfo(inherited_environment = _AI_TEST_INHERITED_ENV),
    ]

# Attributes shared by all AI test rules
_COMMON_AI_TEST_ATTRS = {
    "model": attr.string(
        doc = "AI model name to use for analysis.",
        default = "claude-sonnet-4.6",
    ),
    "score_threshold": attr.string(
        doc = "Minimum average score required to pass the test (0-10).",
        default = "0.0",
    ),
    "batch_size": attr.int(
        doc = "Number of artefacts to process per batch (0 = all at once).",
        default = 0,
    ),
    "context": attr.label(
        doc = "Optional filegroup of background-context files (.md / .puml) " +
              "passed to the AI as read-only reference material.",
        allow_files = [".md", ".puml"],
        default = None,
    ),
    "_custom_ai_model": attr.label(
        doc = "Custom ai_model.py file (optional, provided by consumer repo).",
        default = None,
        allow_single_file = [".py"],
    ),
    "_orchestrator": attr.label(
        doc = "Orchestrator binary (runs at test time, hence target config).",
        default = "//validation/ai_checker:orchestrator",
        executable = True,
        cfg = "target",
    ),
}

# ============================================================================
# TRLC Requirements AI Test
# ============================================================================

def _trlc_requirements_ai_test_impl(ctx):
    """Extract TRLC artefacts from providers and delegate to shared analysis."""
    analysis_files = []
    all_files = []
    dep_dirs = {}

    for req in ctx.attr.reqs:
        trlc_provider = req[TrlcProviderInfo]

        direct_reqs = trlc_provider.reqs.to_list()
        analysis_files.extend(direct_reqs)

        dep_reqs = trlc_provider.deps.to_list()
        spec_files = trlc_provider.spec.to_list()
        all_files.extend(direct_reqs + dep_reqs + spec_files)
        for f in dep_reqs + spec_files:
            dep_dirs[_short_dir(f)] = True

    return _run_ai_analysis(ctx, analysis_files, all_files, dep_dirs, req_files = analysis_files)

_trlc_requirements_ai_test = rule(
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
        "_project_guidelines": attr.label(
            doc = "Project-specific guideline files (graded), settable once via " +
                  "the //validation/ai_checker:project_guidelines flag.",
            default = "//validation/ai_checker:project_guidelines",
            allow_files = True,
        ),
    }),
    test = True,
    toolchains = [],
    fragments = ["platform"],
)

def trlc_requirements_ai_test(name, **kwargs):
    """AI review of TRLC requirements (runs the analysis at test time).

    The AI call is non-hermetic (network + credentials), so default tags mark
    the test as un-sandboxed and network-dependent. Any caller-supplied tags
    are merged on top.
    """
    tags = kwargs.pop("tags", [])
    _trlc_requirements_ai_test(
        name = name,
        tags = _AI_TEST_DEFAULT_TAGS + [t for t in tags if t not in _AI_TEST_DEFAULT_TAGS],
        **kwargs
    )

# ============================================================================
# Architecture AI Test
# ============================================================================

def _architecture_ai_test_impl(ctx):
    """Extract architecture artefacts from providers and delegate to shared analysis.

    Architecture review reads the raw PlantUML *source* (not the parsed
    FlatBuffers binaries in ArchitecturalDesignInfo.static/dynamic). The design
    target's DefaultInfo carries the raw .puml source files.
    """
    analysis_files = []

    # The "designs" attr requires ArchitecturalDesignInfo, so only architectural
    # designs are accepted; the AI reads the raw .puml sources from DefaultInfo.
    for design in ctx.attr.designs:
        for f in design[DefaultInfo].files.to_list():
            if f.extension == "puml":
                analysis_files.append(f)

    return _run_ai_analysis(
        ctx,
        analysis_files,
        analysis_files,
        {},
        artefact_type = "architecture",
    )

_architecture_ai_test = rule(
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
        "_project_guidelines": attr.label(
            doc = "Project-specific guideline files (graded), settable once via " +
                  "the //validation/ai_checker:project_architecture_guidelines flag.",
            default = "//validation/ai_checker:project_architecture_guidelines",
            allow_files = True,
        ),
    }),
    test = True,
    toolchains = [],
    fragments = ["platform"],
)

def architecture_ai_test(name, **kwargs):
    """AI review of PlantUML architecture (runs the analysis at test time).

    The AI call is non-hermetic (network + credentials), so default tags mark
    the test as un-sandboxed and network-dependent. Any caller-supplied tags
    are merged on top.
    """
    tags = kwargs.pop("tags", [])
    _architecture_ai_test(
        name = name,
        tags = _AI_TEST_DEFAULT_TAGS + [t for t in tags if t not in _AI_TEST_DEFAULT_TAGS],
        **kwargs
    )
