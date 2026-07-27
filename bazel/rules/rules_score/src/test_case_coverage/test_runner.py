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
"""Test runner: verify test_case_coverage.lock.yaml is current and emit LOBSTER artifact.

Environment variables
---------------------
TEST_CASE_COVERAGE_LOBSTER_MANIFEST
    Path to a newline-delimited file listing lobster-req-trace JSON paths.
TEST_CASE_COVERAGE_GTEST_LOBSTER
    Path to the gtest.lobster file produced by subrule_lobster_gtest.
TEST_CASE_COVERAGE_LOCK_FILE
    Path to the committed lock YAML (short_path in runfiles).
TEST_CASE_COVERAGE_LABEL
    Bazel label of the test_case_coverage target.
TEST_CASE_COVERAGE_LOBSTER_OUTPUT
    Path where the .lobster artifact should be written.
"""

from __future__ import annotations

import logging
import os
import sys
from pathlib import Path

from test_case_coverage.check_lock import compare_lock_files, validate_specs
from test_case_coverage.compute_lock import load_lock_file
from test_case_coverage.lobster_generator import generate_lobster
from test_case_coverage.read_gtest_lobster import resolve_path
from test_case_coverage.runner_common import require_env, scan_and_compute

logger = logging.getLogger(__name__)


def main(argv: list[str] | None = None) -> None:
    import argparse

    logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s", force=True)

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--allow-check-failures",
        action="store_true",
        help="Return success even when lock checks fail (used for LOBSTER build action).",
    )
    args = parser.parse_args(argv)

    lobster_manifest = resolve_path(require_env("TEST_CASE_COVERAGE_LOBSTER_MANIFEST"))
    gtest_lobster_path = resolve_path(require_env("TEST_CASE_COVERAGE_GTEST_LOBSTER"))
    committed_lock_path = resolve_path(require_env("TEST_CASE_COVERAGE_LOCK_FILE"))
    label = os.environ.get("TEST_CASE_COVERAGE_LABEL", "<unknown>")
    package = os.environ.get("TEST_CASE_COVERAGE_PACKAGE", "")
    lobster_output_raw = os.environ.get("TEST_CASE_COVERAGE_LOBSTER_OUTPUT")

    # Steps 1-3: extract requirement metadata, scan gtest.lobster, compute lock
    _req_metadata, computed = scan_and_compute(
        lobster_manifest, gtest_lobster_path, package=package, label=label
    )

    # Step 3b: validate GWT specs (collect, don't exit yet — artifact must be written first)
    spec_ok, spec_issues = validate_specs(computed)
    if not spec_ok:
        logger.error("One or more test cases are missing GWT spec annotations.")
        for line in spec_issues:
            logger.error(line)

    # Step 4: load committed lock once (may not exist / may fail to parse) —
    # reused for both the LOBSTER artifact (Step 5) and the drift check
    # (Step 6) so the file is never read twice.
    try:
        committed = load_lock_file(committed_lock_path)
        lock_load_error: str | None = None
    except ValueError as exc:
        committed = None
        lock_load_error = str(exc)

    # Step 5: emit LOBSTER artifact — ALWAYS, before any sys.exit, so Bazel's
    # declared output is produced even when the lock check fails (D-6).
    if lobster_output_raw:
        generate_lobster(
            computed=computed,
            committed=committed,
            label=label,
            output_path=Path(lobster_output_raw),
        )

    # Step 6: compare against the already-loaded committed lock
    if lock_load_error is not None:
        lock_ok, diff_lines = False, [lock_load_error]
    else:
        lock_ok, diff_lines = compare_lock_files(committed, computed)
    if not lock_ok:
        logger.error("Coverage lock file is out of date.")
        for line in diff_lines:
            logger.error(line)
        logger.error("Run `bazel run %s.update` to refresh the lock file.", label)

    # Step 7: exit if any check failed (after artifact is written)
    if not args.allow_check_failures:
        if not spec_ok or not lock_ok:
            sys.exit(1)


if __name__ == "__main__":
    main()
