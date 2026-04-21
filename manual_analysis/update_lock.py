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
"""Writes a deterministic sha256 lock file for a set of source files.

Environment variables can be used instead of CLI flags:
- MANUAL_ANALYSIS_FILES_MANIFEST: absolute path to manifest TSV
- MANUAL_ANALYSIS_RULES_MANIFEST: absolute path to manifest TSV
- MANUAL_ANALYSIS_LOCK_FILE: workspace-relative output lock file path

Files manifest format (TSV, one entry per line):
    <display_path>\t<runtime_path>

Rules manifest format (TSV, one entry per line):
    <display_path>\t<canonical-attributes>

Lock file output format (one entry per line):
    <sha256hex> <workspace-relative-path|rule label>
"""

import argparse
import hashlib
import os
import sys
from pathlib import Path

from manual_analysis.common import resolve_path


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _sha256_string(value: str) -> str:
    """Compute SHA256 hash of a string value."""
    h = hashlib.sha256()
    h.update(value.encode("utf-8"))
    return h.hexdigest()


def _write_lock(rows: list[tuple[str, str]], lock_path: Path) -> None:
    """Sort rows by display path and write them to *lock_path*."""
    rows.sort(key=lambda item: item[0])
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("w", encoding="utf-8") as fh:
        for display, digest in rows:
            fh.write(f"{digest} {display}\n")


def _read_manifest_tsv(manifest_path: Path) -> list[tuple[str, str]]:
    files: list[tuple[str, str]] = []
    for line in manifest_path.read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        display, runtime = line.split("\t", 1)
        files.append((display, runtime))
    return files


def main(argv: list[str] | None = None) -> None:
    """CLI entry point: read a TSV manifest and write the lock file."""
    parser = argparse.ArgumentParser(
        description="Recompute the manual-analysis lock file from a file manifest.",
    )
    parser.add_argument(
        "--files-manifest",
        default=os.environ.get("MANUAL_ANALYSIS_FILES_MANIFEST"),
        help="Path to manifest TSV. Can also be provided via MANUAL_ANALYSIS_FILES_MANIFEST.",
    )
    parser.add_argument(
        "--rules-manifest",
        default=os.environ.get("MANUAL_ANALYSIS_RULES_MANIFEST"),
        help="Path to manifest TSV. Can also be provided via MANUAL_ANALYSIS_RULES_MANIFEST.",
    )
    parser.add_argument(
        "--lock-file",
        default=os.environ.get("MANUAL_ANALYSIS_LOCK_FILE"),
        help="Workspace-relative lock file path. Can also be provided via MANUAL_ANALYSIS_LOCK_FILE.",
    )
    parser.add_argument(
        "--output",
        default=os.environ.get("MANUAL_ANALYSIS_OUTPUT"),
        help="Absolute output path (bypasses workspace resolution). "
        "Can also be provided via MANUAL_ANALYSIS_OUTPUT.",
    )
    args = parser.parse_args(argv)

    if not args.files_manifest:
        parser.error(
            "--files-manifest is required (or set MANUAL_ANALYSIS_FILES_MANIFEST)"
        )
    if not args.rules_manifest:
        parser.error(
            "--rules-manifest is required (or set MANUAL_ANALYSIS_RULES_MANIFEST)"
        )
    if not args.lock_file and not args.output:
        parser.error("--lock-file or --output is required")

    files_manifest_path = resolve_path(args.files_manifest)
    if not files_manifest_path.exists():
        print(
            f"ERROR: files manifest not found: {files_manifest_path}", file=sys.stderr
        )
        sys.exit(1)

    rules_manifest_path = resolve_path(args.rules_manifest)
    if not rules_manifest_path.exists():
        print(
            f"ERROR: rules manifest not found: {rules_manifest_path}", file=sys.stderr
        )
        sys.exit(1)

    if args.output:
        lock_path = Path(args.output)
    else:
        lock_path = resolve_path(args.lock_file)

    # Process files: compute SHA256 of file contents
    files = _read_manifest_tsv(files_manifest_path)
    rows = [(display, _sha256(resolve_path(runtime))) for display, runtime in files]

    # Process rules: compute SHA256 of canonical forms
    rules = _read_manifest_tsv(rules_manifest_path)
    rows += [
        (display, _sha256_string(canonical_form)) for display, canonical_form in rules
    ]

    _write_lock(rows, lock_path)


if __name__ == "__main__":
    main()
