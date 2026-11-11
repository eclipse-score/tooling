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
"""Convert the LOBSTER report of a dependable element to a Sphinx-Needs ``needs.json``.

Reverse direction of ``lobster_sphinx_needs``: the executed test cases of the
report (``activity`` levels, e.g. from ``lobster-gtest``) become ``testcase``
Needs of the S-CORE metamodel, so a Sphinx-Needs documentation build can show
them next to the requirements they verify. Requirements are not exported; they
already exist in the documentation.

The ``testcase`` Needs use the ID scheme and fields of the score_docs_as_code
test.xml parser (``testcase__<suite>__<test>_<hash>``), so they replace the
test cases that parser would otherwise create from ``bazel-testlogs``.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

NEEDS_JSON_VERSION = "1.0"

_RESULTS = {"ok": "passed", "fail": "failed"}
_RE_FIELD = re.compile(r"^:(?P<key>[A-Za-z]+):\s*(?P<value>.*)$")


class ReportError(Exception):
    """Invalid LOBSTER report."""


def short_hash(text: str, length: int = 5) -> str:
    """Return the stable short hash of the score_docs_as_code test.xml parser."""
    b32 = base64.b32encode(hashlib.sha256(text.encode()).digest()).decode("utf-8")
    return "".join(filter(str.isalpha, b32))[:length].lower()


def _fields(text: str | None) -> dict[str, str]:
    """Parse the ``:Key: value`` lines lobster-gtest copies from the test properties."""
    fields = {}
    for line in (text or "").splitlines():
        match = _RE_FIELD.match(line.strip())
        if match:
            fields[match.group("key").lower()] = match.group("value").strip()
    return fields


def _split(value: str | None) -> list[str]:
    return [v.strip() for v in (value or "").split(",") if v.strip()]


def _tag_id(tag: str) -> str:
    """``req comp_req__x@2`` -> ``comp_req__x``."""
    return tag.split(" ", 1)[-1].split("@", 1)[0]


def testcase_need(item: dict[str, Any]) -> dict[str, Any]:
    """Convert one LOBSTER activity item into a ``testcase`` Need."""
    name = item["name"].replace(":", "__")
    location = item.get("location") or {}
    file = location.get("file") or ""
    line = location.get("line")
    fields = _fields(item.get("text"))
    partially = _split(fields.get("partiallyverifies"))
    fully = _split(fields.get("fullyverifies"))
    if not partially and not fully:
        # Traced via the "lobster-tracing" property only: no full/partial distinction.
        partially = [_tag_id(tag) for tag in item.get("ref_up", [])]
    return {
        "id": f"testcase__{name}_{short_hash((file or '<placeholder_file>') + name)}",
        "type": "testcase",
        "title": name,
        "tags": ["TEST"],
        "name": name,
        "file": file,
        "line": str(line) if line else "",
        "result": _RESULTS.get(item.get("status") or "", "skipped"),
        "result_text": "",
        "test_type": fields.get("testtype", ""),
        "derivation_technique": fields.get("derivationtechnique", ""),
        "partially_verifies": partially,
        "fully_verifies": fully,
        "repo_name": "",
    }


def convert(report: dict[str, Any], project: str) -> dict[str, Any]:
    """Return the ``needs.json`` content for a LOBSTER report."""
    if report.get("schema") != "lobster-report":
        raise ReportError(f"expected a lobster-report, got schema {report.get('schema')!r}")
    needs: dict[str, dict[str, Any]] = {}
    for level in report.get("levels", []):
        if level.get("kind") != "activity":
            continue
        for item in level.get("items", []):
            need = testcase_need(item)
            needs[need["id"]] = need
    needs = dict(sorted(needs.items()))
    return {
        "current_version": NEEDS_JSON_VERSION,
        "project": project,
        "versions": {NEEDS_JSON_VERSION: {"needs": needs, "needs_amount": len(needs)}},
    }


def main(argv: list[str] | None = None) -> int:
    """Entry point of the converter."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("report", nargs="?", help="LOBSTER report; omitted for an element without report")
    parser.add_argument("--out", required=True, help="output needs.json")
    parser.add_argument("--project", default="", help="project name written to needs.json")
    args = parser.parse_args(argv)

    report = {"schema": "lobster-report", "levels": []}
    if args.report:
        report = json.loads(Path(args.report).read_text(encoding="utf-8"))
    try:
        needs_json = convert(report, args.project)
    except ReportError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1
    Path(args.out).write_text(json.dumps(needs_json, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
