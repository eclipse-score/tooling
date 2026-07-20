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
"""Test runner: verify coverage.lock.yaml is current and emit LOBSTER artifact.

Environment variables
---------------------
REQ_COVERAGE_LOBSTER_MANIFEST
    Path to a newline-delimited file listing lobster-req-trace JSON paths.
REQ_COVERAGE_GTEST_LOBSTER
    Path to the gtest.lobster file produced by subrule_lobster_gtest.
REQ_COVERAGE_LOCK_FILE
    Path to the committed lock YAML (short_path in runfiles).
REQ_COVERAGE_LABEL
    Bazel label of the req_coverage target.
REQ_COVERAGE_LOBSTER_OUTPUT
    Path where the .lobster artifact should be written.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

from req_coverage.check_lock import evaluate_lock, validate_specs
from req_coverage.compute_lock import compute_lock, load_lock_file
from req_coverage.lobster_generator import generate_lobster
from req_coverage.read_gtest_lobster import (
    read_req_metadata_from_lobster_files,
    resolve_path,
    scan_gtest_lobster,
)


def _require_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        print(f"ERROR: {name} is required", file=sys.stderr)
        sys.exit(1)
    return value


def main(argv: list[str] | None = None) -> None:
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--allow-check-failures",
        action="store_true",
        help="Return success even when lock checks fail (used for LOBSTER build action).",
    )
    args = parser.parse_args(argv)

    lobster_manifest = resolve_path(_require_env("REQ_COVERAGE_LOBSTER_MANIFEST"))
    gtest_lobster_path = resolve_path(_require_env("REQ_COVERAGE_GTEST_LOBSTER"))
    committed_lock_path = resolve_path(_require_env("REQ_COVERAGE_LOCK_FILE"))
    label = os.environ.get("REQ_COVERAGE_LABEL", "<unknown>")
    package = os.environ.get("REQ_COVERAGE_PACKAGE", "")
    lobster_output_raw = os.environ.get("REQ_COVERAGE_LOBSTER_OUTPUT")

    # Step 1: extract requirement metadata (CompReq only)
    req_metadata = read_req_metadata_from_lobster_files(lobster_manifest)

    req_ids = [m.id for m in req_metadata]

    # Step 2: scan gtest.lobster for test records
    by_req = scan_gtest_lobster(gtest_lobster_path, req_ids, package=package)

    # Warn about requirements with no linked tests
    for req_id, records in by_req.items():
        if not records:
            print(
                f"WARNING: [{label}] Requirement {req_id!r} has no linked test cases "
                "in the scanned XML files.",
                file=sys.stderr,
            )

    # Step 3: compute lock
    computed = compute_lock(req_metadata, by_req)

    # Step 3b: validate GWT specs (collect, don't exit yet — artifact must be written first)
    spec_ok, spec_issues = validate_specs(computed)
    if not spec_ok:
        print(
            "ERROR: One or more test cases are missing GWT spec annotations.",
            file=sys.stderr,
        )
        for line in spec_issues:
            print(line, file=sys.stderr)

    # Step 4: load committed lock (may not exist)
    try:
        committed = load_lock_file(committed_lock_path)
    except ValueError:
        committed = None

    # Step 5: emit LOBSTER artifact — ALWAYS, before any sys.exit, so Bazel's
    # declared output is produced even when the lock check fails (D-6).
    if lobster_output_raw:
        generate_lobster(
            computed=computed,
            committed=committed,
            label=label,
            output_path=Path(lobster_output_raw),
        )

    # Step 6: compare
    lock_ok, diff_lines = evaluate_lock(committed_lock_path, computed)
    if not lock_ok:
        print("ERROR: Coverage lock file is out of date.", file=sys.stderr)
        for line in diff_lines:
            print(line, file=sys.stderr)
        print(
            f"\nRun `bazel run {label}.update` to refresh the lock file.",
            file=sys.stderr,
        )

    # Step 7: exit if any check failed (after artifact is written)
    if not args.allow_check_failures:
        if not spec_ok or not lock_ok:
            sys.exit(1)


if __name__ == "__main__":
    main()
