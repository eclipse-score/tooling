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
"""Shared CLI helpers used by both ``test_runner`` and ``update_runner``.

Both entry points read the same set of environment variables, scan the
gtest.lobster / req-lobster manifest to compute a fresh :class:`LockFile`, and
warn about requirements with zero linked test cases. Keeping that pipeline in
one place avoids the two runners drifting apart.
"""

from __future__ import annotations

import logging
import os
import sys
from pathlib import Path

from test_case_coverage.compute_lock import LockFile, RequirementMeta, compute_lock
from test_case_coverage.read_gtest_lobster import (
    read_req_metadata_from_lobster_files,
    resolve_path,
    scan_gtest_lobster,
)
from test_case_coverage.test_spec import load_test_spec_manifest

logger = logging.getLogger(__name__)


def require_env(name: str) -> str:
    """Return the value of environment variable *name*, or exit(1) if unset/empty."""
    value = os.environ.get(name)
    if not value:
        logger.error("%s is required", name)
        sys.exit(1)
    return value


def optional_spec_manifest() -> Path | None:
    """Return the resolved ``TEST_CASE_COVERAGE_SPEC_MANIFEST`` path, or None if unset."""
    raw = os.environ.get("TEST_CASE_COVERAGE_SPEC_MANIFEST")
    return resolve_path(raw) if raw else None


def scan_and_compute(
    lobster_manifest: Path,
    gtest_lobster_path: Path,
    package: str,
    label: str,
    spec_manifest: Path | None = None,
) -> tuple[list[RequirementMeta], LockFile]:
    """Extract requirement metadata, scan gtest.lobster, and compute the lock.

    *spec_manifest* lists the ``.test_spec.json`` files of the
    ``test_specification`` targets that test cases may reference.

    Emits a ``WARNING`` to stderr for every requirement with zero linked test
    cases found in the scanned XML files.

    Returns:
        (req_metadata, computed_lock)
    """
    req_metadata = read_req_metadata_from_lobster_files(lobster_manifest)
    req_ids = [m.id for m in req_metadata]

    by_req = scan_gtest_lobster(gtest_lobster_path, req_ids, package=package, label=label)

    for req_id, records in by_req.items():
        if not records:
            logger.warning(
                "[%s] Requirement %r has no linked test cases in the scanned XML files.",
                label,
                req_id,
            )

    specs = load_test_spec_manifest(spec_manifest)
    for records in by_req.values():
        for record in records:
            if record.spec_ref and (record.given or record.when or record.then):
                logger.warning(
                    "[%s] %s references test-spec %r; its given/when/then is ignored.",
                    label,
                    record.uid,
                    record.spec_ref,
                )

    computed = compute_lock(req_metadata, by_req, specs)
    return req_metadata, computed
