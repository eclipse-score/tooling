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

"""Shared helper for generating RST wrapper pages for PlantUML diagram files."""

load("@bazel_skylib//lib:paths.bzl", "paths")

def _relative_source_path(file, package):
    prefix = package + "/" if package else ""
    if file.short_path.startswith(prefix):
        return file.short_path[len(prefix):]
    return file.basename

def _directory_title(directory):
    if not directory:
        return "Architectural Design"
    return directory.split("/")[-1].replace("_", " ").title()

def make_puml_rst_navigation(ctx, puml_files, output_dir, template, strip_prefix = "", filename_prefix = "", stems = None):
    """Generate PlantUML wrapper pages and indexes matching source directories.

    The wrapper embeds the diagram via ``.. uml::`` so it appears as a
    proper toctree entry while keeping the source ``.puml`` file separate.

    When disambiguated stems are provided (for collision handling), the stems
    are used in place of plain basenames while preserving the source directory
    structure for navigation and sidebar visibility.

    Args:
        ctx:             Rule context.
        puml_files:      Iterable of File objects whose extension is ``puml`` or
                         ``plantuml``.
        output_dir:      String prefix for declared output files
                         (e.g. ``ctx.label.name``).
        template:        The ``puml_diagram.template.rst`` File (from
                         ``ctx.file._puml_rst_template``).
        strip_prefix:    Optional filename stem prefix to strip before deriving
                         the human-readable title (e.g. ``"fta_"``).
        filename_prefix: Optional prefix prepended to the output RST filename
                         stem (e.g. ``"detail_"``).
        stems:           Optional dict from File.path to a precomputed unique
                         stem (see architectural_design.bzl's
                         _disambiguated_stems), used instead of the plain
                         basename stem for both the output filename and the
                         embedded ``.. uml::`` reference -- needed when the
                         diagram was colocated under a disambiguated name to
                         avoid colliding with a same-named diagram elsewhere.

    Returns:
        Struct containing ``wrappers``, ``indexes``, and ``root_index``.
    """
    wrappers = []
    diagrams_by_directory = {}
    directories = {"": True}
    for f in puml_files:
        if f.extension not in ("puml", "plantuml"):
            continue
        relative_path = _relative_source_path(f, ctx.label.package)
        relative_directory = paths.dirname(relative_path)
        if relative_directory == ".":
            relative_directory = ""

        # Use disambiguated stem for generated filenames, but keep the title
        # based on the source basename so the sidebar does not show a path.
        source_stem = paths.basename(relative_path)[:-(len(f.extension) + 1)]
        stem = stems[f.path] if stems else source_stem
        title = source_stem
        if strip_prefix and title.startswith(strip_prefix):
            title = title[len(strip_prefix):]
        title = title.replace("_", " ").title()
        wrapper_relative_path = paths.join(relative_directory, filename_prefix + stem + ".rst")
        wrapper = ctx.actions.declare_file(
            "{}/{}".format(output_dir, wrapper_relative_path),
        )

        # For the embedded diagram filename, use disambiguated stem if available
        basename = "{}.{}".format(stems[f.path], f.extension) if stems else f.basename
        ctx.actions.expand_template(
            template = template,
            output = wrapper,
            substitutions = {
                "{title}": title,
                "{underline}": "=" * len(title),
                "{basename}": basename,
            },
        )
        wrappers.append(wrapper)
        diagrams_by_directory.setdefault(relative_directory, []).append(stem)
        directory_parts = relative_directory.split("/") if relative_directory else []
        for part_count in range(1, len(directory_parts) + 1):
            directories["/".join(directory_parts[:part_count])] = True

    indexes = []
    for directory in sorted(directories.keys()):
        entries = []
        for stem in sorted(diagrams_by_directory.get(directory, [])):
            entries.append(stem)
        directory_prefix = directory + "/" if directory else ""
        for child in sorted(directories.keys()):
            child_prefix = directory_prefix
            if child.startswith(child_prefix) and child != directory:
                remainder = child[len(child_prefix):]
                if "/" not in remainder:
                    entries.append(remainder + "/index")
        index = ctx.actions.declare_file(
            "{}/{}".format(output_dir, paths.join(directory, "index.rst")),
        )
        ctx.actions.write(
            output = index,
            content = "{}\n{}\n\n.. toctree::\n   :maxdepth: 1\n\n{}\n".format(
                _directory_title(directory),
                "-" * len(_directory_title(directory)),
                "\n".join(["   " + entry for entry in entries]),
            ),
        )
        indexes.append(index)

    return struct(
        wrappers = wrappers,
        indexes = indexes,
        root_index = indexes[0] if indexes else None,
    )
