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
"""Derive root-cause assessments from Sphinx-Needs FMEA needs.

A ``comp_saf_fmea`` need rated ``sufficient: yes`` is the assessment of all
root causes its fault tree generates: one LOBSTER item
``req <package>.<need ID>.assessment`` tracing to these root causes and to the
``mitigated_by`` measures. A ``[version==N]`` filter of a measure link becomes
the expected LOBSTER version, so a stale measure version fails the report.

Measures not among the items of ``--measures`` (e.g. AoUs of the platform) are
not traced; score_docs_as_code checks the ``mitigated_by`` links of the need.
"""

from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path

from lobster.common.errors import LOBSTER_Error, Message_Handler
from lobster.common.io import lobster_read, lobster_write
from lobster.common.items import Requirement, Tracing_Tag
from lobster_sphinx_needs import ConfigError, _location, load_needs, parse_link

logger = logging.getLogger(__name__)

GENERATOR = "fmea_needs_assessments"
FRAMEWORK = "Sphinx-Needs"
KIND = "RootCauseAssessment"

_LEVEL_MAP = {
    "error": logging.ERROR,
    "warn": logging.WARNING,
    "info": logging.INFO,
    "debug": logging.DEBUG,
}


def _read_items(paths: list[str]) -> dict[str, Requirement]:
    mh = Message_Handler()
    items: dict[str, Requirement] = {}
    for path in paths:
        try:
            lobster_read(mh, path, "input", items)
        except (OSError, LOBSTER_Error) as e:
            raise SystemExit(f"Failed to parse lobster file {path}: {e}") from e
    return items


def _links(need: dict, field_name: str) -> list[str]:
    value = need.get(field_name)
    if not value:
        return []
    values = value if isinstance(value, list) else str(value).split(",")
    return [str(v).strip() for v in values if str(v).strip()]


def root_causes_by_failure_mode(root_causes: dict[str, Requirement]) -> dict[str, list[Tracing_Tag]]:
    """Root-cause tags per failure-mode key (``req <package>.<name>``)."""
    by_fm: dict[str, list[Tracing_Tag]] = {}
    for item in root_causes.values():
        for ref in item.unresolved_references:
            by_fm.setdefault(ref.key(), []).append(item.tag)
    return by_fm


def derive_assessments(
    needs_files: list[str],
    package: str,
    root_causes: dict[str, Requirement],
    measures: dict[str, Requirement],
    need_type: str = "comp_saf_fmea",
) -> list[Requirement]:
    """One assessment item per sufficient FMEA need that has root causes."""
    rc_by_fm = root_causes_by_failure_mode(root_causes)
    assessments = []
    for path in needs_files:
        needs = load_needs(path)
        for need_id in sorted(needs):
            need = {"id": need_id, **needs[need_id]}
            if need.get("type") != need_type or need.get("status") == "invalid":
                continue
            rc_tags = rc_by_fm.get(f"req {package}.{need_id}", [])
            if not rc_tags:
                logger.info("%s: no root cause in the fault trees, no assessment", need_id)
                continue
            if need.get("sufficient") != "yes":
                logger.info("%s: not rated sufficient, its root causes stay unassessed", need_id)
                continue
            version = need.get("version")
            item = Requirement(
                tag=Tracing_Tag("req", f"{package}.{need_id}.assessment", None if version in (None, "") else version),
                location=_location(need, path),
                framework=FRAMEWORK,
                kind=KIND,
                name=f"{need_id}.assessment",
                text=str(need.get("content") or need.get("title") or ""),
                status=None,
            )
            for tag in sorted(rc_tags, key=lambda t: t.key()):
                item.add_tracing_target(tag)
            for link in _links(need, "mitigated_by"):
                tag = parse_link(link, "req")
                if tag.key() in measures:
                    item.add_tracing_target(tag)
                else:
                    logger.info("%s: measure %s is not a measure of the analysis, not traced", need_id, tag.key())
            assessments.append(item)
    return assessments


def main(argv: list[str] | None = None) -> int:
    """Entry point of the assessment generator."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("needs", nargs="+", help="needs.json files with the FMEA needs")
    parser.add_argument("--package", required=True, help="TRLC package of the FailureMode records of the needs")
    parser.add_argument("--root-causes", action="append", default=[], help="root-cause .lobster file (repeatable)")
    parser.add_argument("--measures", action="append", default=[], help="measure .lobster file (repeatable)")
    parser.add_argument("--out", required=True, help="output assessments .lobster file")
    parser.add_argument("--log-level", choices=sorted(_LEVEL_MAP), default="warn", dest="log_level")
    args = parser.parse_args(argv)
    logging.basicConfig(level=_LEVEL_MAP[args.log_level], format="%(levelname)s: %(message)s")

    try:
        items = derive_assessments(args.needs, args.package, _read_items(args.root_causes), _read_items(args.measures))
    except ConfigError as e:
        logger.error("%s", e)
        return 1
    with open(Path(args.out), "w", encoding="utf-8") as f:
        lobster_write(f, Requirement, GENERATOR, items)
    logger.info("Wrote %d assessment(s) to %s", len(items), args.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
