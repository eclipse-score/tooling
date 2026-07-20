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
"""Read test records from lobster-gtest JSON artifacts.

Provides all lobster-file reading utilities used by both runners:

* :func:`read_gtest_lobster` — parse a single gtest.lobster file
* :func:`scan_gtest_lobster` — group test records by requirement ID
* :func:`read_req_metadata_from_lobster_files` — extract CompReq metadata from a manifest
* :func:`resolve_path` — resolve Bazel-relative path in action/runfiles contexts
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field, replace
from pathlib import Path

from lobster.common.errors import LOBSTER_Error, Message_Handler
from lobster.common.io import lobster_read
from lobster.common.items import Activity, Requirement

from req_coverage.compute_lock import RequirementMeta


@dataclass
class TestRecord:
    """Metadata extracted from a single gtest.lobster item."""

    uid: str  # "Suite:TestName" (gtest tag without "gtest " prefix)
    lobster_traces: list[str] = field(default_factory=list)  # requirement IDs
    given: str = ""  # :Given: text from RecordProperty annotation
    when: str = ""  # :When: text
    then: str = ""  # :Then: text


def _parse_gwt(text: str) -> tuple[str, str, str]:
    """Parse a ``:Given:/:When:/:Then:`` text field into its three components.

    lobster-gtest capitalises RecordProperty keys, so the expected format is::

        :Given: value
        :When: value
        :Then: value
    """
    given = when = then = ""
    for line in text.splitlines():
        if line.startswith(":Given: "):
            given = line[len(":Given: ") :]
        elif line.startswith(":When: "):
            when = line[len(":When: ") :]
        elif line.startswith(":Then: "):
            then = line[len(":Then: ") :]
    return given, when, then


def read_gtest_lobster(lobster_path: Path) -> list[TestRecord]:
    """Parse a single gtest.lobster JSON file and return test records.

    Only items with at least one ``req`` reference are returned; items that
    carry no tracing annotation are silently skipped.

    Args:
        lobster_path: Path to a ``lobster-act-trace`` JSON file produced by
                      ``lobster-gtest``.

    Raises:
        ValueError: if the file cannot be read or parsed.
    """
    mh = Message_Handler()
    items: dict = {}
    try:
        lobster_read(mh, str(lobster_path), "act", items)
    except (OSError, LOBSTER_Error) as exc:
        raise ValueError(f"Cannot parse gtest.lobster {lobster_path}: {exc}") from exc

    records: list[TestRecord] = []
    for item in items.values():
        if not isinstance(item, Activity):
            continue
        if item.tag.namespace != "gtest":
            continue

        refs = [
            ref.tag
            for ref in item.unresolved_references
            if ref.namespace == "req"
        ]
        if not refs:
            continue

        gwt = _parse_gwt(item.text or "")
        records.append(
            TestRecord(
                uid=item.tag.tag,
                lobster_traces=refs,
                given=gwt[0],
                when=gwt[1],
                then=gwt[2],
            )
        )

    return records


# ---------------------------------------------------------------------------
# Path resolution (action-sandbox vs runfiles contexts)
# ---------------------------------------------------------------------------


def resolve_path(raw: str) -> Path:
    """Resolve a Bazel-relative path to an absolute filesystem path.

    Resolution order:

    1. Absolute path that exists — returned as-is.
    2. Relative path that exists from CWD — works in Bazel action sandboxes
       where CWD is the execroot and short paths are available.
    3. ``$RUNFILES_DIR/<raw>`` — used by ``bazel run`` executables whose CWD
       is ``$BUILD_WORKSPACE_DIRECTORY``, not the execroot.
    4. Raw path returned unchanged as a last resort (caller will get a clear
       ``FileNotFoundError`` rather than a silent wrong-path write).
    """
    candidate = Path(raw)
    if candidate.is_absolute() and candidate.exists():
        return candidate
    if not candidate.is_absolute() and candidate.exists():
        return candidate.resolve()
    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir and not candidate.is_absolute():
        via_runfiles = Path(runfiles_dir) / raw
        if via_runfiles.exists():
            return via_runfiles
    return candidate


# ---------------------------------------------------------------------------
# Req-ID extraction from lobster-req-trace manifests
# ---------------------------------------------------------------------------

# Only these TRLC requirement kinds belong in the coverage lock file.
# Feature requirements and assumed-system requirements are excluded — they
# are traceability targets, not directly testable component-level items.
_COMP_REQ_KINDS: frozenset[str] = frozenset({"CompReq"})


def read_req_metadata_from_lobster_files(
    lobster_manifest_path: Path,
) -> list[RequirementMeta]:
    """Parse a single-column manifest of lobster paths and return CompReq metadata.

    Each line is a path to a ``lobster-req-trace`` JSON file.  Only items
    with ``kind == "CompReq"`` are included; FeatReq and AssumedSystemReq
    items are silently skipped.

    Returns a deduplicated list sorted by requirement ID.  Each entry carries:

    * ``id`` — requirement identifier (tag without ``@version``)
    * ``version`` — version string from the ``@version`` tag suffix (e.g. ``"1"``)
    * ``description`` — requirement text from the ``text`` field
    """
    metadata: list[RequirementMeta] = []
    seen: set[str] = set()

    for line in lobster_manifest_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        lobster_path = resolve_path(line)
        mh = Message_Handler()
        items: dict = {}
        try:
            lobster_read(mh, str(lobster_path), "req", items)
        except (OSError, LOBSTER_Error) as exc:
            raise ValueError(f"Cannot read lobster file {lobster_path}: {exc}") from exc

        for item in items.values():
            if not isinstance(item, Requirement):
                continue
            if item.kind not in _COMP_REQ_KINDS:
                continue  # skip FeatReq, AssumedSystemReq, etc.
            if item.tag.namespace != "req":
                continue
            req_id = item.tag.tag
            if req_id in seen:
                continue
            seen.add(req_id)
            metadata.append(
                RequirementMeta(
                    id=req_id,
                    version=str(item.tag.version) if item.tag.version is not None else "",
                    description=str(item.text or ""),
                )
            )

    return sorted(metadata, key=lambda m: m.id)


# ---------------------------------------------------------------------------
# Gtest scan — group records by requirement ID
# ---------------------------------------------------------------------------


def scan_gtest_lobster(
    gtest_lobster_path: Path,
    req_ids: list[str],
    package: str = "",
) -> dict[str, list[TestRecord]]:
    """Read a gtest.lobster file and group test records by requirement ID.

    Only records whose ``lobster_traces`` intersect with *req_ids* are kept.
    A record that covers multiple requirements appears in all their lists.

    The ``package`` (e.g. ``//score/message_passing``) is prepended to each
    uid to make it globally unique across components.  Pass
    ``"//" + ctx.label.package`` from the Bazel rule via the
    ``REQ_COVERAGE_PACKAGE`` environment variable.
    """
    all_records = read_gtest_lobster(gtest_lobster_path)
    req_id_set = set(req_ids)
    by_req: dict[str, list[TestRecord]] = {rid: [] for rid in req_ids}
    for record in all_records:
        if package:
            record = replace(record, uid=f"{package}/{record.uid}")
        for trace in record.lobster_traces:
            if trace in req_id_set:
                by_req[trace].append(record)
    return by_req
