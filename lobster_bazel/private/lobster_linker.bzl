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

# ---------------------------------------------------------------------------
# Aspect – transitively collect source files from deps
# ---------------------------------------------------------------------------

_CollectedFilesInfo = provider(
    doc = "Internal provider for collecting all source files.",
    fields = {
        "files": "depset of source files",
    },
)

def _extract_files_from_attr(attr, attr_name):
    """Extracts source files from a given attribute, if the attribute exists."""
    return [
        f
        for src in getattr(attr, attr_name, [])
        for f in src.files.to_list()
        if not f.path.startswith("external")
    ]

def _get_transitive_deps(attr, attr_name):
    """Extracts previously collected transitive dependencies."""
    return [
        dep[_CollectedFilesInfo].files
        for dep in getattr(attr, attr_name, [])
        if _CollectedFilesInfo in dep
    ]

def _collect_source_files_aspect_impl(_target, ctx):
    """Aspect implementation to collect source files from rules and dependencies."""
    return [
        _CollectedFilesInfo(
            files = depset(
                _extract_files_from_attr(ctx.rule.attr, "srcs") +
                _extract_files_from_attr(ctx.rule.attr, "hdrs"),
                transitive = _get_transitive_deps(ctx.rule.attr, "deps"),
            ),
        ),
    ]

_collect_source_files_aspect = aspect(
    implementation = _collect_source_files_aspect_impl,
    attr_aspects = ["deps"],
    doc = "Aspect that collects source files from a rule and its dependencies.",
)

# ---------------------------------------------------------------------------
# Subrule implementation
# ---------------------------------------------------------------------------

def _lobster_linker_subrule_impl(ctx, files, tracing_tags, namespace, _parser):
    """Subrule implementation: takes file list, tags and namespace, emits .lobster."""

    # 1. Write the file list to a temporary text file -------------------
    sources_file = ctx.actions.declare_file("%s_sources.txt" % ctx.label.name)

    ctx.actions.write(sources_file, "\n".join([f.path for f in files]))

    # 2. Declare output .lobster file -----------------------------------
    lobster_file = ctx.actions.declare_file("%s.lobster" % ctx.label.name)

    # 3. Build command-line arguments -----------------------------------
    args = ctx.actions.args()
    args.add(sources_file)
    args.add("--output", lobster_file)
    for tag in tracing_tags:
        args.add("--tag", tag)
    if namespace:
        args.add("--namespace", namespace)

    # 4. Run the parser -------------------------------------------------
    ctx.actions.run(
        arguments = [args],
        executable = _parser,
        inputs = depset(
            [sources_file] + files,
        ),
        outputs = [lobster_file],
        mnemonic = "LobsterLinker",
        progress_message = "Scanning source files for tracing tags: %s" % ctx.label,
    )

    return [
        lobster_file,
        LobsterProvider(lobster_input = {lobster_file.basename: lobster_file.path}),
    ]

subrule_lobster_linker = subrule(
    implementation = _lobster_linker_subrule_impl,
    attrs = {
        "_parser": attr.label(
            default = Label("//lobster_bazel:lobster-bazel"),
            executable = True,
            cfg = "exec",
        ),
    },
)

# ---------------------------------------------------------------------------
# Rule implementation
# ---------------------------------------------------------------------------

def _lobster_linker_impl(ctx):
    """Implementation of the lobster_linker rule."""
    all_files = depset(
        direct = ctx.files.srcs,
        transitive = _get_transitive_deps(ctx.attr, "srcs"),
    ).to_list()

    # gToDo: what is going on here?
    lobster_file, lobster_provider = subrule_lobster_linker(
        all_files,
        ctx.attr.tracing_tags,
        ctx.attr.namespace,
    )

    return [
        DefaultInfo(
            files = depset([lobster_file]),
            runfiles = ctx.runfiles([lobster_file]),
        ),
        lobster_provider,
    ]

# ---------------------------------------------------------------------------
# Rule definition
# ---------------------------------------------------------------------------

lobster_linker = rule(
    implementation = _lobster_linker_impl,
    attrs = {
        "srcs": attr.label_list(
            aspects = [_collect_source_files_aspect],
            allow_files = True,
            doc = "Source file targets (filegroups, libraries, etc.) to scan for tracing tags.",
        ),
        "tracing_tags": attr.string_list(
            default = ["lobster-trace"],
            doc = "List of tracing tag attribute names to search for in source files. The tool will construct the full pattern as: <comment_sign> <tag>: <id> where the comment sign is auto-derived from the file extension. Defaults to ['lobster-trace'].",
        ),
        "namespace": attr.string(
            default = "source",
            doc = "Namespace prefix for lobster tags (default: 'source').",
        ),
    },
    subrules = [subrule_lobster_linker],
    provides = [
        DefaultInfo,
        LobsterProvider,
    ],
    doc = "Scans source files for tracing tags and produces a .lobster file. Provides a LobsterProvider for integration with lobster_test traceability reports.",
)
