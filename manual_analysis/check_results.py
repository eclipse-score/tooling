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
"""Checks whether the final manual-analysis assertion is positive."""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

from common import resolve_path


def _load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(
        description="Check whether the final assertion in manual-analysis results passed.",
    )
    parser.add_argument(
        "--results-file",
        default=os.environ.get("MANUAL_ANALYSIS_RESULTS_FILE"),
        help="Path to manual analysis results JSON file.",
    )
    args = parser.parse_args(argv)

    if not args.results_file:
        parser.error("--results-file is required (or set MANUAL_ANALYSIS_RESULTS_FILE)")

    results_path = resolve_path(args.results_file)
    if not results_path.exists():
        print(f"ERROR: results file not found: {results_path}", file=sys.stderr)
        sys.exit(1)

    payload = _load(results_path)
    results = payload.get("results") if isinstance(payload, dict) else None
    if not isinstance(results, list) or not results:
        print("ERROR: Manual analysis results are missing or empty.", file=sys.stderr)
        sys.exit(1)

    last = results[-1]
    if not isinstance(last, dict) or last.get("type") != "assertion":
        print("ERROR: Manual analysis does not end with an assertion.", file=sys.stderr)
        sys.exit(1)

    if last.get("passed") is True:
        return

    print("ERROR: Final manual analysis assertion is not positive.", file=sys.stderr)
    sys.exit(1)


if __name__ == "__main__":
    main()
