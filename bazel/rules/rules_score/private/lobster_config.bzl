# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
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
Utilities for filling out Lobster traceability report config templates.

Report configs are static .tpl files in tools/lobster/templates/; the only
dynamic part is the list of source file paths for each level.  Rules call
format_lobster_sources() to build the source-line block, then pass it as a
substitution value to ctx.actions.expand_template().
"""

def format_lobster_sources(files):
    """Format a list of File objects as lobster source lines.

    Args:
        files: List of File objects to include as sources.

    Returns:
        String containing one ``  source: "...";`` line per file, suitable
        for substituting into a lobster config template placeholder.
    """
    return "\n".join(['  source: "{}";'.format(f.path) for f in files])

def format_lobster_block(kind, name, files, trace_to = [], emit_empty = False, requires = []):
    """Build a complete, optional LOBSTER config block.

    By default returns "" (omitting the level entirely) when `files` is empty,
    instead of emitting an empty block. An empty level that other levels
    `trace to:` would make LOBSTER report every item at those other levels as
    missing a reference, so callers must only pass names in `trace_to` for
    levels that are themselves non-empty (i.e. also present in the same config).

    Set `emit_empty = True` to declare the level even when it has no sources.
    This keeps the level's `trace to:` policy edge active so that LOBSTER
    reports a "missing down reference" for every item at the target level that
    this (now source-less) level fails to cover — used in release mode to make
    a missing verification level (e.g. no unit tests, no root causes) fail the
    traceability check instead of silently disappearing from the policy.

    By default, when *multiple* levels each declare `trace to: <this level>`,
    LOBSTER requires ALL of them to independently cover every item here (an
    AND across sources) -- see the `requires` config directive in LOBSTER's
    documentation. Pass `requires` to relax this to an OR: each element is a
    list of level names, rendered as a single `requires: "A" or "B";` line
    (multiple elements become separate, independently-AND'd `requires:`
    lines). Only include level names that are themselves present (non-empty
    or emit_empty) in the same config, and that already declare a `trace to:`
    line pointing at this level (`requires` narrows an existing mandatory
    `trace to:` edge into an alternative, it does not create one).

    Args:
        kind: LOBSTER block kind, e.g. "requirements", "activity", "implementation".
        name: Level name, used as the quoted block header.
        files: List of File objects to include as sources; the block is
            omitted entirely (returns "") when this list is empty, unless
            `emit_empty` is True.
        trace_to: List of level names this level traces to. Only include
            names of levels that are themselves present (non-empty) in the
            same config.
        emit_empty: When True, emit the block header and `trace to:` lines even
            if `files` is empty (no `source:` lines are produced). Defaults to
            False.
        requires: List of OR-groups (each a list of level names) that jointly
            replace the default AND-of-sources coverage check for this level
            with "any one of these is sufficient". Defaults to [] (no
            override; the default AND-of-sources behaviour applies).

    Returns:
        The full block text (kind, header, sources, trace-to lines), or ""
        when `files` is empty and `emit_empty` is False.
    """
    if not files and not emit_empty:
        return ""
    trace_lines = "".join(['  trace to: "{}";\n'.format(t) for t in trace_to])
    requires_lines = "".join([
        "  requires: {};\n".format(" or ".join(['"{}"'.format(n) for n in group]))
        for group in requires
        if group
    ])
    return '{kind} "{name}" {{\n{sources}\n{trace}{requires}}}'.format(
        kind = kind,
        name = name,
        sources = format_lobster_sources(files),
        trace = trace_lines,
        requires = requires_lines,
    )
