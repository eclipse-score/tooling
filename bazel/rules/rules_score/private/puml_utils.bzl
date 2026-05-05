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

def make_puml_rst_wrappers(ctx, puml_files, output_dir, template, strip_prefix = "", filename_prefix = ""):
    """Generate a thin RST wrapper page for each PlantUML diagram file.

    The wrapper embeds the diagram via ``.. uml::`` so it appears as a
    proper toctree entry while keeping the source ``.puml`` file separate.

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

    Returns:
        List of declared ``.rst`` output Files, one per input diagram.
    """
    wrappers = []
    for f in puml_files:
        if f.extension not in ("puml", "plantuml"):
            continue
        stem = f.basename[:-(len(f.extension) + 1)]
        if strip_prefix and stem.startswith(strip_prefix):
            stem = stem[len(strip_prefix):]
        title = stem.replace("_", " ").title()
        wrapper = ctx.actions.declare_file(
            "{}/{}{}.rst".format(output_dir, filename_prefix, stem),
        )
        ctx.actions.expand_template(
            template = template,
            output = wrapper,
            substitutions = {
                "{title}": title,
                "{underline}": "=" * len(title),
                "{basename}": f.basename,
            },
        )
        wrappers.append(wrapper)
    return wrappers
