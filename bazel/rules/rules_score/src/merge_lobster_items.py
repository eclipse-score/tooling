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
"""Merge lobster files into one, collapsing duplicate items.

One item reaches a dependable_element along several paths, e.g. an AoU through
a direct dependency and through a dependency that forwards it, or an AoU
listed by several safety analyses. Each path is a separate file carrying an
item with the same tag, which lobster rejects as a duplicate definition. Items
with the same tag and the same definition (location aside) are kept once; items
with the same tag but a different definition, such as two versions of one AoU,
are an error.
"""

from __future__ import annotations

import argparse
import logging
from pathlib import Path

from lobster.common.errors import LOBSTER_Error, Message_Handler
from lobster.common.io import lobster_read, lobster_write
from lobster.common.items import Requirement

GENERATOR = "merge_lobster_items"

# Not part of an item's definition: where it was found, and report-time state.
_NON_DEFINITION_FIELDS = frozenset({"location", "messages", "ref_up", "ref_down", "tracing_status"})

logger = logging.getLogger(__name__)

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}


def _content(item: Requirement) -> dict:
    return {key: value for key, value in item.to_json().items() if key not in _NON_DEFINITION_FIELDS}


def merge_items(lobster_paths: list[str]) -> list[Requirement]:
    """Merge the items of all files; the first file defining a tag wins.

    Args:
        lobster_paths: Paths to the .lobster files, origin files first.

    Returns:
        Items in order of first appearance, one per tag.

    Raises:
        SystemExit: If a file cannot be read, or two files define the same tag
            with a different definition.
    """
    mh = Message_Handler()
    merged: dict[str, Requirement] = {}
    origin: dict[str, str] = {}
    for path in lobster_paths:
        items: dict[str, Requirement] = {}
        try:
            lobster_read(mh, path, "merged", items)
        except (OSError, LOBSTER_Error) as e:
            raise SystemExit(f"Failed to parse lobster file {path}: {e}") from e
        for key, item in items.items():
            known = merged.get(key)
            if known is None:
                merged[key] = item
                origin[key] = path
                continue
            known_content, content = _content(known), _content(item)
            if known_content != content:
                differing = sorted(
                    f for f in known_content.keys() | content.keys() if known_content.get(f) != content.get(f)
                )
                raise SystemExit(
                    f"Conflicting definitions of '{key}' in {origin[key]} and {path}: {', '.join(differing)} differ."
                )
            logger.info("Dropped duplicate of '%s' from %s", key, path)
    return list(merged.values())


def main() -> None:
    """Entry point for the lobster item merge tool."""
    parser = argparse.ArgumentParser(description="Merge lobster files, collapsing duplicate items.")
    parser.add_argument(
        "--input-lobster",
        nargs="+",
        required=True,
        help="Lobster files to merge; on a duplicate tag the first file wins.",
    )
    parser.add_argument("--output", required=True, help="Output .lobster file path.")
    parser.add_argument(
        "--log-level",
        choices=["error", "warn", "info", "debug"],
        default="warn",
        dest="log_level",
        help="Log level for tool output (default: warn).",
    )
    args = parser.parse_args()
    logging.basicConfig(level=_LEVEL_MAP[args.log_level], format="%(levelname)s: %(message)s")

    merged = merge_items(args.input_lobster)

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        lobster_write(f, Requirement, GENERATOR, merged)
    logger.info("Wrote %d item(s) to %s", len(merged), output_path)


if __name__ == "__main__":
    main()
