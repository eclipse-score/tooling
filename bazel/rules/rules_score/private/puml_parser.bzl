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
Shared `puml_cli` invocation helper for `architectural_design.bzl` and
`unit_design.bzl`.

One `puml_cli` invocation resolves all its `.puml` files as one group per
diagram kind.

- `disambiguated_stems`: a unique output stem per input file.
- `run_puml_parser`: one `puml_cli` action for a whole file list.
"""

load("@bazel_skylib//lib:paths.bzl", "paths")
load("//bazel/rules/rules_score/private:puml_utils.bzl", "relative_source_path")
load("//bazel/rules/rules_score/private:verbosity.bzl", "get_log_level")

def disambiguated_stems(ctx, files):
    """Computes a unique output stem (no directory, no extension) per .puml file.

    A unique basename keeps its plain stem. Colliding basenames are prefixed
    with the package-relative directory (`/` becomes `_`). A residual stem
    collision fails the build naming both files.

    Args:
        ctx: Rule context.
        files: Iterable of File objects (non-.puml entries ignored).
    Returns:
        Dict from File.path to a unique stem string.
    """
    puml_files = [f for f in files if f.extension == "puml"]
    basename_counts = {}
    for f in puml_files:
        basename_counts[f.basename] = basename_counts.get(f.basename, 0) + 1

    stems = {}
    stem_sources = {}
    for f in puml_files:
        stem = f.basename.rsplit(".", 1)[0]
        if basename_counts[f.basename] > 1:
            dir_part = paths.dirname(relative_source_path(f, ctx.label.package, ctx.label.workspace_name))
            stem = "{}__{}".format(dir_part.replace("/", "_"), stem) if dir_part else stem

        previous = stem_sources.get(stem)
        if previous != None:
            fail((
                "{}: '{}' and '{}' both disambiguate to the output stem '{}'; " +
                "rename one of these files so their stems differ"
            ).format(ctx.label, previous, f.short_path, stem))
        stem_sources[stem] = f.short_path
        stems[f.path] = stem
    return stems

def run_puml_parser(ctx, files, stems, diagram_type = None, emit_lobster = False, progress_label = None):
    """Runs `puml_cli` once for a whole `.puml` file list, so the files resolve
    as one group.

    `--output-stem` is each file's disambiguated stem, so the output names
    match the declared outputs. `--source-name` is each file's `short_path`:
    the stable, workspace-relative `source` embedded in the outputs, which the
    `clickable_plantuml` Sphinx extension matches against its own doctree
    paths.

    Args:
        ctx: Rule context.
        files: List of File objects to parse (non-.puml entries ignored).
        stems: Dict from File.path to unique output stem (see `disambiguated_stems`).
        diagram_type: Optional CLI `--diagram-type` value (e.g. "component");
            `None` leaves detection to the parser. With a value, a diagram of
            another kind fails the action.
        emit_lobster: When `True`, also produce a `.lobster` per file.
        progress_label: Optional group description for the progress message;
            defaults to `str(ctx.label)`.
    Returns:
        Tuple of (fbs_outputs, lobster_outputs, idmap_outputs), one entry per
        `.puml` file in `files`, in order. `lobster_outputs` is empty unless
        `emit_lobster`.
    """
    puml_files = [f for f in files if f.extension == "puml"]
    if not puml_files:
        return [], [], []

    fbs_outputs = []
    lobster_outputs = []
    idmap_outputs = []
    arguments = []
    for f in puml_files:
        stem = stems[f.path]
        fbs_outputs.append(ctx.actions.declare_file("{}/{}.fbs.bin".format(ctx.label.name, stem)))
        if emit_lobster:
            lobster_outputs.append(ctx.actions.declare_file("{}/{}.lobster".format(ctx.label.name, stem)))
        idmap_outputs.append(ctx.actions.declare_file("{}/{}.idmap.json".format(ctx.label.name, stem)))
        arguments.extend([
            "--file",
            f.path,
            "--source-name",
            f.short_path,
            "--output-stem",
            stem,
        ])

    if diagram_type:
        arguments.extend(["--diagram-type", diagram_type])

    arguments.extend([
        "--fbs-output-dir",
        fbs_outputs[0].dirname,
        "--idmap-output-dir",
        idmap_outputs[0].dirname,
        "--log-level",
        get_log_level(ctx),
    ])
    if emit_lobster:
        arguments.extend(["--lobster-output-dir", lobster_outputs[0].dirname])

    ctx.actions.run(
        inputs = puml_files,
        outputs = fbs_outputs + lobster_outputs + idmap_outputs,
        executable = ctx.executable._puml_parser,
        arguments = arguments,
        progress_message = "Parsing PlantUML diagram group: %s" % (progress_label or str(ctx.label)),
    )

    return fbs_outputs, lobster_outputs, idmap_outputs
