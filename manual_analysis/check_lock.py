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
"""Checks whether committed and computed manual-analysis lock files match."""

import argparse
import os
import sys
from pathlib import Path

from common import resolve_path


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(
        description="Check whether the computed lock matches the committed lock.",
    )
    parser.add_argument(
        "--computed",
        default=os.environ.get("MANUAL_ANALYSIS_COMPUTED_LOCK"),
        help="Path to newly computed lock file.",
    )
    parser.add_argument(
        "--committed",
        default=os.environ.get("MANUAL_ANALYSIS_COMMITTED_LOCK"),
        help="Path to committed lock file.",
    )
    args = parser.parse_args(argv)

    if not args.computed:
        parser.error("--computed is required (or set MANUAL_ANALYSIS_COMPUTED_LOCK)")
    if not args.committed:
        parser.error("--committed is required (or set MANUAL_ANALYSIS_COMMITTED_LOCK)")

    computed = resolve_path(args.computed)
    committed = resolve_path(args.committed)

    if not computed.exists():
        print(f"ERROR: computed lock file not found: {computed}", file=sys.stderr)
        sys.exit(1)
    if not committed.exists():
        print(f"ERROR: committed lock file not found: {committed}", file=sys.stderr)
        sys.exit(1)

    if _read(computed) == _read(committed):
        return

    print("ERROR: Manual analysis lock file is out of date.", file=sys.stderr)
    sys.exit(1)


if __name__ == "__main__":
    main()
