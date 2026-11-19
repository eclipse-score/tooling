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
"""Point the lobster items of a generated TRLC file at the diagram they come from.

lobster-trlc locates each item at its record in the generated file (a
``bazel-out`` path no repository holds), so report links for it cannot resolve.
``puml_cli`` writes the origin of every record as a ``// <diagram>:<line>``
comment above it; this tool locates each item of the generated file at that
diagram and line. Other items are kept as they are.
"""

from __future__ import annotations

import argparse
import logging
import os
import re
from pathlib import Path

from lobster.common.errors import LOBSTER_Error, Message_Handler
from lobster.common.io import lobster_read, lobster_write
from lobster.common.items import Requirement
from lobster.common.location import File_Reference

GENERATOR = "relocate_lobster_items"

# The comment `puml_utils::render_trlc_file` writes above every record.
_ORIGIN_COMMENT = re.compile(r"//\s*(?P<diagram>.+):(?P<line>\d+)")

logger = logging.getLogger(__name__)

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}


def read_origins(generated_trlc: str) -> dict[int, tuple[str, int]]:
    """Read the origin comments of a generated TRLC file.

    Args:
        generated_trlc: Path of the generated TRLC file.

    Returns:
        The ``(diagram, line)`` origin of each record, keyed by the line the
        record starts on: the line after its comment.
    """
    origins: dict[int, tuple[str, int]] = {}
    with open(generated_trlc, encoding="utf-8") as f:
        for number, text in enumerate(f, start=1):
            match = _ORIGIN_COMMENT.fullmatch(text.strip())
            if match:
                origins[number + 1] = (match["diagram"], int(match["line"]))
    return origins


def relocate_items(lobster_path: str, generated_trlc: str, sources: dict[str, str]) -> list[Requirement]:
    """Point the items located in *generated_trlc* at their origin.

    Args:
        lobster_path: Path to the .lobster file lobster-trlc wrote.
        generated_trlc: Path of the generated TRLC file, as the items locate it.
        sources: Path of each diagram *generated_trlc* was generated from, by
            the name the origin comments give the diagram.

    Returns:
        All items in their original order, the ones located in
        *generated_trlc* now located at the line of their source diagram.

    Raises:
        SystemExit: If the file cannot be parsed, or an item of the generated
            file has no origin comment or originates from an unknown diagram.
    """
    mh = Message_Handler()
    items: dict[str, Requirement] = {}
    try:
        lobster_read(mh, lobster_path, "relocated", items)
    except (OSError, LOBSTER_Error) as e:
        raise SystemExit(f"Failed to parse lobster file {lobster_path}: {e}") from e

    generated = os.path.normpath(generated_trlc)
    origins = read_origins(generated_trlc)
    for item in items.values():
        location = item.location
        if not isinstance(location, File_Reference) or os.path.normpath(location.filename) != generated:
            continue
        origin = origins.get(location.line)
        if origin is None:
            raise SystemExit(
                f"'{item.tag}' is at {location.to_string()}, where no record with an origin comment starts."
            )
        diagram, line = origin
        if diagram not in sources:
            raise SystemExit(
                f"'{item.tag}' originates from '{diagram}', which is none of the sources: {', '.join(sorted(sources))}"
            )
        # puml_cli writes 0 for a line it does not know.
        item.location = File_Reference(sources[diagram], line or None)
        logger.debug("Relocated '%s' to %s", item.tag, item.location.to_string())
    return list(items.values())


def _source(argument: str) -> tuple[str, str]:
    name, separator, path = argument.partition("=")
    if not (name and separator and path):
        raise argparse.ArgumentTypeError(f"expected NAME=PATH, got '{argument}'")
    return name, path


def main() -> None:
    """Entry point for the lobster item relocation tool."""
    parser = argparse.ArgumentParser(
        description="Point the lobster items of a generated TRLC file at the diagram they come from."
    )
    parser.add_argument("--input-lobster", required=True, help="The .lobster file lobster-trlc wrote.")
    parser.add_argument(
        "--generated-trlc",
        required=True,
        help="The generated TRLC file, as the items of the .lobster file locate it.",
    )
    parser.add_argument(
        "--source",
        action="append",
        type=_source,
        default=[],
        metavar="NAME=PATH",
        help="A diagram the TRLC file was generated from: the name the origin comments give it, and its path.",
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

    items = relocate_items(args.input_lobster, args.generated_trlc, dict(args.source))

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        lobster_write(f, Requirement, GENERATOR, items)
    logger.info("Wrote %d item(s) to %s", len(items), output_path)


if __name__ == "__main__":
    main()
