#!/usr/bin/env python3
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
#
# Carry over from Eclipse S-Core including slight modifications:
# https://github.com/eclipse-score/docs-as-code/tree/v0.4.0/src/extensions/score_source_code_linker

"""
Lobster source code linker -- scans source files for tracing tags and produces
a ``.lobster`` file in ``lobster-imp-trace`` format.

Tracing tags in source files follow the pattern::

    <comment_sign> <tracing_attribute>: <id>

The comment sign is automatically derived from the file extension.
Tracing attributes (e.g. ``req-traceability``, ``req-Id``) are configurable
via the ``--tag`` argument.
"""

import argparse
import logging
import sys
from pathlib import Path
from typing import Optional, Sequence, Union

from lobster.common.items import Implementation, Tracing_Tag
from lobster.common.location import File_Reference
from lobster.common.io import lobster_write

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Comment-sign mapping (file extension → single-line comment prefix)
# ---------------------------------------------------------------------------

COMMENT_SIGNS: dict[str, str] = {
    # C-family
    ".c": "//",
    ".cc": "//",
    ".cpp": "//",
    ".cxx": "//",
    ".h": "//",
    ".hh": "//",
    ".hpp": "//",
    ".hxx": "//",
    # Rust
    ".rs": "//",
    # Python
    ".py": "#",
    # Starlark / Bazel
    ".bzl": "#",
    # TRLC
    ".trlc": "#",
    ".rsl": "#",
}

LANGUAGE_MAP: dict[str, str] = {
    ".c": "C/C++",
    ".cc": "C/C++",
    ".cpp": "C/C++",
    ".cxx": "C/C++",
    ".h": "C/C++",
    ".hh": "C/C++",
    ".hpp": "C/C++",
    ".hxx": "C/C++",
    ".rs": "Rust",
    ".py": "Python",
    ".bzl": "Starlark",
    ".trlc": "TRLC",
    ".rsl": "TRLC",
}

LOBSTER_GENERATOR = "lobster_linker"


# ---------------------------------------------------------------------------
# Public helpers
# ---------------------------------------------------------------------------


def get_comment_sign(file_path: str) -> Union[str, None]:
    """Derive the single-line comment sign from a file's extension.

    Args:
        file_path: Path to the source file.

    Returns:
        The comment prefix string, or ``None`` if the extension is unknown.
    """
    ext = Path(file_path).suffix.lower()
    return COMMENT_SIGNS.get(ext)


def get_language(file_path: str) -> str:
    """Derive the language name from a file's extension.

    Args:
        file_path: Path to the source file.

    Returns:
        A human-readable language string (e.g. ``"Rust"``, ``"Python"``).
        Falls back to the uppercased extension if not in LANGUAGE_MAP.
    """
    ext = Path(file_path).suffix.lower()
    return LANGUAGE_MAP.get(ext, ext.lstrip(".").upper() or "Unknown")


def build_tag_patterns(tracing_tags: list[str], comment_sign: str) -> list[str]:
    """Construct the full tag patterns to search for in source lines.

    Each pattern is: ``<comment_sign> <tracing_tag>:``

    Args:
        tracing_tags: List of tracing attribute names
                      (e.g. ``["req-traceability", "req-Id"]``).
        comment_sign: The comment prefix for the file type.

    Returns:
        List of tag pattern strings.
    """
    return [f"{comment_sign} {tag}:" for tag in tracing_tags]


def extract_lobster_items(
    source_file: str,
    tracing_tags: list[str],
    namespace: str = "source",
) -> list[Implementation]:
    """Scan a source file for tracing tags and produce lobster trace items.

    For every matching line the function creates a lobster ``Implementation``
    item with a ``refs`` entry pointing at ``req <id>`` so that the lobster
    report can link implementations back to requirements.

    Tags are matched only when the pattern appears at the start of the
    (stripped) line, preventing false positives from mid-line occurrences.

    Args:
        source_file: Path to the source file to scan.
        tracing_tags: List of tracing attribute names to look for.
        namespace: Namespace prefix for lobster tags (default: ``"source"``).

    Returns:
        List of ``Implementation`` objects in ``lobster-imp-trace`` format.
    """
    comment_sign = get_comment_sign(source_file)
    if comment_sign is None:
        logger.warning("Unknown file extension for '%s', skipping.", source_file)
        return []
    tag_patterns = build_tag_patterns(tracing_tags, comment_sign)
    items: list[Implementation] = []
    try:
        with open(source_file, encoding="utf-8", errors="replace") as fh:
            for line_number, line in enumerate(fh, start=1):
                stripped = line.strip()
                for pattern in tag_patterns:
                    if stripped.startswith(pattern):
                        after_pattern = stripped[len(pattern) :].strip()
                        req_id = (
                            after_pattern.replace("'", "")
                            .replace('"', "")
                            .replace(",", "")
                            .strip()
                        )
                        if not req_id:
                            continue
                        item = Implementation(
                            tag=Tracing_Tag(namespace, req_id),
                            location=File_Reference(source_file, line_number),
                            language=get_language(source_file),
                            kind="Implementation",
                            name=req_id,
                        )
                        item.add_tracing_target(Tracing_Tag("req", req_id))
                        items.append(item)
                        break  # One match per line is enough
    except OSError:
        logger.exception("Cannot read file '%s'", source_file)
        raise
    return items


# ---------------------------------------------------------------------------
# CLI entry-point
# ---------------------------------------------------------------------------


def main(args: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        description="Scan source files for tracing tags and produce a .lobster file.",
    )
    parser.add_argument(
        "-o",
        "--output",
        required=True,
        help="Output .lobster file path.",
    )
    parser.add_argument(
        "--tag",
        action="append",
        dest="tags",
        required=True,
        help=(
            "Tracing tag attribute to scan for "
            "(can be specified multiple times, e.g. --tag req-Id --tag req-traceability)."
        ),
    )
    parser.add_argument(
        "--namespace",
        default="source",
        help="Namespace prefix for lobster tags (default: 'source').",
    )
    parser.add_argument(
        "inputs",
        nargs="*",
        help="File-list files (each line contains a source file path to scan).",
    )

    options = parser.parse_args(args)

    all_items: list[Implementation] = []
    for input_list_file in options.inputs:
        try:
            with open(input_list_file) as fh:
                for source_file_line in fh:
                    source_file = source_file_line.strip()
                    if not source_file:
                        continue
                    try:
                        items = extract_lobster_items(
                            source_file,
                            tracing_tags=options.tags,
                            namespace=options.namespace,
                        )
                        all_items.extend(items)
                    except OSError as exc:
                        logger.error("Skipping '%s': %s", source_file, exc)
        except OSError as exc:
            logger.error("Cannot read file list '%s': %s", input_list_file, exc)
            return 1

    with open(options.output, "w", encoding="utf-8") as fh:
        lobster_write(fh, Implementation, LOBSTER_GENERATOR, all_items)

    print(f"lobster-bazel: wrote {len(all_items)} items to {options.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
