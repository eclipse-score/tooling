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
"""LOBSTER glue of the fault-tree analysis.

``known-failure-modes`` lists the failure modes that the lobster files of the
``failure_modes`` targets define, for ``puml_cli`` to check the ``$FailureMode``
calls against.

``findings`` turns the findings of ``fta_index.json`` into lobster items.  Every
item carries its message, so lobster reports it as not traced and the
traceability report fails on it.
"""

from __future__ import annotations

import argparse
import logging
import re
from pathlib import Path

from fta_index import FtaIndex
from lobster.common.errors import LOBSTER_Error, Message_Handler
from lobster.common.io import lobster_read, lobster_write
from lobster.common.items import Requirement, Tracing_Tag
from lobster.common.location import File_Reference

GENERATOR = "fta_lobster"
FRAMEWORK = "puml_fta"
NAMESPACE = "fta"

logger = logging.getLogger(__name__)

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}


def known_failure_modes(lobster_paths: list[str]) -> list[str]:
    """Names (``<Package>.<Name>``) of the items of *lobster_paths*, sorted and unique.

    Raises:
        SystemExit: If a file cannot be read.
    """
    mh = Message_Handler()
    names: set[str] = set()
    for path in lobster_paths:
        items: dict[str, Requirement] = {}
        try:
            lobster_read(mh, path, "failure_modes", items)
        except (OSError, LOBSTER_Error) as e:
            raise SystemExit(f"Failed to parse lobster file {path}: {e}") from e
        names.update(item.tag.tag for item in items.values())
    return sorted(names)


def finding_items(index: FtaIndex, package: str) -> list[Requirement]:
    """One lobster item per finding, tagged ``fta <package>.<diagram>_<line>_<kind>``.

    The ``fta`` namespace keeps the tags apart from the root cause items.
    The location is the diagram source; a finding without a line has none.
    """
    used: set[str] = set()
    items = []
    for finding in index.findings:
        stem = finding.diagram.removesuffix(".puml")
        base = re.sub(r"[^0-9A-Za-z]", "_", f"{stem}_{finding.line}_{finding.kind}")
        name, counter = base, 1
        while name in used:
            counter += 1
            name = f"{base}_{counter}"
        used.add(name)
        path = index.diagrams.get(finding.diagram, finding.diagram)
        item = Requirement(
            tag=Tracing_Tag(NAMESPACE, f"{package}.{name}"),
            location=File_Reference(path, finding.line or None),
            framework=FRAMEWORK,
            kind=finding.kind,
            name=name,
            text=finding.message,
        )
        item.error(finding.message)
        items.append(item)
    return items


def _write_lobster(path: str, items: list[Requirement]) -> None:
    output = Path(path)
    output.parent.mkdir(parents=True, exist_ok=True)
    with open(output, "w", encoding="utf-8") as fh:
        lobster_write(fh, Requirement, GENERATOR, items)


def main() -> None:
    """Entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--log-level",
        choices=sorted(_LEVEL_MAP),
        default="warn",
        dest="log_level",
        help="Log level for tool output (default: warn).",
    )
    commands = parser.add_subparsers(dest="command", required=True)

    known = commands.add_parser("known-failure-modes", help="List the failure modes the lobster files define.")
    known.add_argument("--output", required=True, help="Output text file, one failure mode name per line.")
    known.add_argument("lobster", nargs="*", help="Lobster files of the failure_modes targets.")

    findings = commands.add_parser("findings", help="Write the findings of fta_index.json as lobster items.")
    findings.add_argument("--index", required=True, help="fta_index.json written by puml_cli.")
    findings.add_argument("--package", required=True, help="TRLC package of the fault trees; scopes the item tags.")
    findings.add_argument("--output", required=True, help="Output .lobster file.")

    args = parser.parse_args()
    logging.basicConfig(level=_LEVEL_MAP[args.log_level], format="%(levelname)s: %(message)s")

    if args.command == "known-failure-modes":
        names = known_failure_modes(args.lobster)
        Path(args.output).parent.mkdir(parents=True, exist_ok=True)
        Path(args.output).write_text("".join(f"{name}\n" for name in names), encoding="utf-8")
        logger.info("Wrote %d failure mode(s) to %s", len(names), args.output)
    else:
        try:
            index = FtaIndex.load(args.index)
        except ValueError as e:
            raise SystemExit(str(e)) from e
        items = finding_items(index, args.package)
        _write_lobster(args.output, items)
        logger.info("Wrote %d finding(s) to %s", len(items), args.output)


if __name__ == "__main__":
    main()
