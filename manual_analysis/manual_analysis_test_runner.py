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
"""Unified manual-analysis test runner.

Performs lock/results checks and can emit a LOBSTER artifact in one flow.
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

from manual_analysis.common import resolve_path
from manual_analysis.check_lock import evaluate_lock_files
from manual_analysis.check_results import evaluate_results_file
from manual_analysis.lobster_generator import write_lobster_file
from manual_analysis.yaml_schema import load_analysis


def _required_path(name: str) -> Path:
    raw = os.environ.get(name)
    if not raw:
        raise ValueError(f"{name} is required")
    return resolve_path(raw)


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Check lock and results for a manual analysis and optionally emit "
            "a LOBSTER artifact."
        ),
    )
    parser.add_argument(
        "--allow-check-failures",
        action="store_true",
        help=(
            "Return success even when lock/results checks fail. "
            "Useful for artifact-generation actions."
        ),
    )
    args = parser.parse_args(argv)

    try:
        computed_lock = _required_path("MANUAL_ANALYSIS_COMPUTED_LOCK")
        committed_lock = _required_path("MANUAL_ANALYSIS_COMMITTED_LOCK")
        results_file = _required_path("MANUAL_ANALYSIS_RESULTS_FILE")
        analysis_yaml = _required_path("MANUAL_ANALYSIS_YAML")
    except ValueError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        sys.exit(1)

    analysis_label = os.environ.get("MANUAL_ANALYSIS_LABEL")
    if not analysis_label:
        print("ERROR: MANUAL_ANALYSIS_LABEL is required", file=sys.stderr)
        sys.exit(1)

    lock_ok, lock_error = evaluate_lock_files(computed_lock, committed_lock)
    results_ok, results_error = evaluate_results_file(results_file)

    try:
        _steps, requirements = load_analysis(analysis_yaml)
    except Exception as error:  # pragma: no cover - defensive wrapper
        # Requirements parsing errors must fail and skip artifact emission.
        print(f"ERROR: Failed to parse analysis requirements: {error}", file=sys.stderr)
        sys.exit(1)

    lobster_output_raw = os.environ.get("MANUAL_ANALYSIS_LOBSTER_OUTPUT")
    if lobster_output_raw:
        write_lobster_file(
            requirements=requirements,
            analysis_passed=lock_ok and results_ok,
            results_file_path=str(results_file),
            analysis_label=analysis_label,
            output_path=Path(lobster_output_raw),
        )

    if lock_error is not None:
        print(f"ERROR: {lock_error}", file=sys.stderr)
    if results_error is not None:
        print(f"ERROR: {results_error}", file=sys.stderr)

    checks_ok = lock_ok and results_ok
    if checks_ok or args.allow_check_failures:
        return
    sys.exit(1)


if __name__ == "__main__":
    main()
