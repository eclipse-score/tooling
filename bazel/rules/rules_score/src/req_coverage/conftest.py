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

"""Shared pytest helpers for req_coverage tests."""

from __future__ import annotations

from pathlib import Path

from lobster.common.io import lobster_write
from lobster.common.items import Activity, Requirement, Tracing_Tag
from lobster.common.location import Void_Reference

collect_ignore = ["test_runner.py", "update_runner.py"]

_DEFAULT_SPEC = ":Given: g\n:When: w\n:Then: t"


def write_req_lobster(
    path: Path,
    req_ids: list[str],
    *,
    version: str = "1",
    kind: str = "CompReq",
    text: str = "",
) -> None:
    """Write a minimal lobster-req-trace JSON file to *path*.

    Args:
        req_ids: Requirement identifiers.
        version: Version suffix appended to every tag (``@version``).
        kind:    TRLC requirement kind written to the ``kind`` field.
                 Defaults to ``"CompReq"``; use ``"FeatReq"`` to write
                 feature requirements that should be filtered out.
        text:    Description text written to the ``text`` field.
    """
    items = [
        Requirement(
            tag=Tracing_Tag("req", r, version=version),
            location=Void_Reference(),
            framework="TRLC",
            kind=kind,
            name=r,
            text=text or None,
        )
        for r in req_ids
    ]
    with path.open("w", encoding="utf-8") as fd:
        lobster_write(fd, Requirement, "t", items)


def write_gtest_lobster(
    path: Path,
    uid: str,
    req_id: str,
    spec: str = _DEFAULT_SPEC,
) -> None:
    """Write a minimal lobster-act-trace JSON file to *path*."""
    item = Activity(
        tag=Tracing_Tag("gtest", uid),
        location=Void_Reference(),
        framework="GoogleTest",
        kind="test",
        text=spec or None,
        status="ok",
    )
    item.add_tracing_target(Tracing_Tag("req", req_id))
    with path.open("w", encoding="utf-8") as fd:
        lobster_write(fd, Activity, "lobster_gtest", [item])
