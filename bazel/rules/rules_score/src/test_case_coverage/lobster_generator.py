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
"""Emit a lobster-act-trace artifact using the lobster library.

Each *requirement* in the lock file becomes one ``Activity`` item, grouping
all test cases that cover that requirement.  The ``status`` field carries the
coverage verdict:

  - ``"ok"``   — every test case for this requirement matches the committed lock
  - ``"fail"`` — at least one test case is new, removed, or its GWT has changed

The artifact is always written, even when verification fails, so that the
LOBSTER dashboard remains current during CI failures (D-6).

Reuses ``Activity``, ``Tracing_Tag``, ``Void_Reference``, and
``lobster_write`` from the lobster library (no manual JSON construction).
"""

from __future__ import annotations

from pathlib import Path

from lobster.common.io import lobster_write
from lobster.common.items import Activity, Tracing_Tag
from lobster.common.location import Void_Reference

from test_case_coverage.compute_lock import LockFile, TestCase

GENERATOR = "test_case_coverage"


def _committed_index(
    committed: LockFile | None,
) -> dict[str, tuple[str, dict[str, tuple[str, str, str]]]]:
    """Build {req_id: (version, {uid: (given, when, then)})} from committed lock."""
    if committed is None:
        return {}
    return {
        req.id: (
            req.version,
            {tc.uid: (tc.given, tc.when, tc.then) for tc in req.test_cases},
        )
        for req in committed.requirements
    }


def _make_activity(
    req_id: str,
    req_version: str,
    test_cases: list[TestCase],
    committed_version: str,
    committed_map: dict[str, tuple[str, str, str]],
    label: str,
) -> Activity:
    """Build one Activity representing coverage of *req_id* by all its test cases.

    ``covered`` is True only when the committed version matches, every test
    case matches the committed GWT, and no committed test has been removed.
    Text concatenates the GWT block of each test case, separated by a blank line.

    The item's own location is always void: this Activity aggregates
    multiple test cases (with potentially different source locations), so
    there is no single canonical source to point at. Each test case instead
    gets a native cross-reference to its underlying gtest lobster item
    (which carries its own real location), so the rendered report links
    straight through to the actual source of every individual test case.
    """
    tag = Tracing_Tag("test_case_coverage", req_id)

    blocks = []
    for tc in test_cases:
        parts = [f":{k.capitalize()}: {v}" for k, v in (("given", tc.given), ("when", tc.when), ("then", tc.then)) if v]
        if parts:
            # Prefix each block with the test case uid as a bold heading so that
            # multiple test cases for one requirement stay visually distinct in
            # the rendered report, instead of blurring into one GWT list.
            blocks.append(f"**{tc.uid}**\n\n" + "\n".join(parts))
    # Separate multiple test case blocks with a horizontal rule; a single block
    # needs no separator, so the join is a no-op in that case.
    separator = "\n\n.. raw:: html\n\n   <hr>\n\n"
    text = separator.join(blocks) if blocks else None

    computed_uids = {tc.uid for tc in test_cases}
    covered = (
        # version must match the committed approval
        req_version == committed_version
        # every computed test matches the committed GWT
        and all(committed_map.get(tc.uid) == (tc.given, tc.when, tc.then) for tc in test_cases)
        # AND no committed test has been removed from computed
        and set(committed_map.keys()) <= computed_uids
    )

    item = Activity(
        tag=tag,
        location=Void_Reference(),
        framework=GENERATOR,
        kind="test",
        text=text,
        status="ok" if covered else "fail",
    )
    item.add_tracing_target(Tracing_Tag("req", req_id))
    for tc in test_cases:
        if tc.gtest_tag:
            item.add_tracing_target(Tracing_Tag("gtest", tc.gtest_tag))

    if not covered:
        item.messages.append(f"test case coverage not confirmed for {label}")
        if req_version != committed_version:
            item.messages.append(f"Requirement Version updated: {committed_version!r} → {req_version!r}")

    return item


def generate_lobster(
    computed: LockFile,
    committed: LockFile | None,
    label: str,
    output_path: Path,
) -> None:
    """Write a lobster-act-trace file for *computed*, validated against *committed*.

    Args:
        computed:     Freshly computed lock (from current test XML scan).
        committed:    Committed lock file content, or None if missing.
        label:        Bazel label of the test_case_coverage target (for context).
        output_path:  Where to write the .lobster JSON file.
    """
    committed_index = _committed_index(committed)
    items: list[Activity] = []

    for req in computed.requirements:
        if not req.test_cases:
            continue
        committed_version, committed_map = committed_index.get(req.id, ("", {}))
        items.append(
            _make_activity(
                req_id=req.id,
                req_version=req.version,
                test_cases=req.test_cases,
                committed_version=committed_version,
                committed_map=committed_map,
                label=label,
            )
        )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8") as fd:
        lobster_write(fd, Activity, GENERATOR, items)
