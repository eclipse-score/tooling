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
"""Helpers for L3 artifact tests: pick a single output of a target and diff it against a golden file."""

load("@aspect_bazel_lib//lib:write_source_files.bzl", "write_source_files")
load("@score_tooling//bazel/rules/rules_score:providers.bzl", "ComponentInfo")

def _single_file_result(ctx, files, description, candidates):
    if len(files) != 1:
        fail("{}: expected one {}, got: {}".format(
            ctx.attr.src.label,
            description,
            [f.short_path for f in candidates],
        ))
    return [DefaultInfo(
        files = depset(files),
        runfiles = ctx.runfiles(files = files),
    )]

def _component_architecture_lobster_impl(ctx):
    src = ctx.attr.src

    # `architecture` also holds the files of nested components.
    candidates = src[ComponentInfo].architecture.to_list()
    files = [f for f in candidates if f.owner == src.label]
    return _single_file_result(ctx, files, "architecture lobster owned by the component", candidates)

component_architecture_lobster_rule = rule(
    implementation = _component_architecture_lobster_impl,
    doc = "Exposes the `<name>_architecture.lobster` file of a `component`.",
    attrs = {
        "src": attr.label(
            mandatory = True,
            providers = [ComponentInfo],
            doc = "The component target.",
        ),
    },
)

def _matches_suffix(file, suffix):
    return file.short_path == suffix or file.short_path.endswith("/" + suffix)

def _output_file_impl(ctx):
    src = ctx.attr.src
    group = ctx.attr.output_group
    if group:
        groups = src[OutputGroupInfo] if OutputGroupInfo in src else struct()
        if not hasattr(groups, group):
            fail("{}: no output group '{}', available: {}".format(
                src.label,
                group,
                [g for g in dir(groups) if not g.startswith("_")],
            ))
        candidates = getattr(groups, group).to_list()
    else:
        candidates = src[DefaultInfo].files.to_list()

    files = [f for f in candidates if _matches_suffix(f, ctx.attr.suffix)]
    return _single_file_result(
        ctx,
        files,
        "file ending in '{}' in output group '{}'".format(ctx.attr.suffix, group or "default"),
        candidates,
    )

output_file_rule = rule(
    implementation = _output_file_impl,
    doc = "Exposes the single file of the default outputs or an output group of `src` whose path ends in `suffix`.",
    attrs = {
        "output_group": attr.string(
            doc = "Output group to search; empty means the default outputs.",
        ),
        "src": attr.label(
            mandatory = True,
            doc = "The target producing the file.",
        ),
        "suffix": attr.string(
            mandatory = True,
            doc = "Trailing path segments of the output to pick, for example `log.txt` or `dir/index.rst`.",
        ),
    },
)

def component_architecture_lobster(name, src, **kwargs):
    """Exposes the `<name>_architecture.lobster` file of the `component` `src`."""
    kwargs.setdefault("testonly", True)
    component_architecture_lobster_rule(
        name = name,
        src = src,
        **kwargs
    )

def output_file(name, src, suffix, output_group = "", **kwargs):
    """Exposes the single file ending in `suffix` of the default outputs or of `output_group` of `src`."""
    kwargs.setdefault("testonly", True)
    output_file_rule(
        name = name,
        src = src,
        suffix = suffix,
        output_group = output_group,
        **kwargs
    )

def golden_test(name, src, golden, **kwargs):
    """Declares the golden check for the single-file target `src` against the source file `golden`.

    `bazel run :<name>` updates the golden, `bazel test :<name>_test` diffs it.
    """
    kwargs.setdefault("testonly", True)
    write_source_files(
        name = name,
        files = {golden: src},
        **kwargs
    )
