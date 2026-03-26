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
"""Generate LOBSTER-act-trace JSON for manual verification results."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def generate_lobster_json(
    requirements: list[str],
    analysis_passed: bool,
    results_file_path: str,
    analysis_label: str,
) -> str:
    """Generate LOBSTER-act-trace JSON for manual verification results.

    Creates a LOBSTER traceability file mapping each requirement to the
    verification outcome. Manual analysis is treated as a verification activity
    over implementation context.

    Args:
        requirements: List of requirement identifiers (strings)
        analysis_passed: True if the assertion passed, False otherwise
        results_file_path: Path to the manual analysis results file (for location info)
        analysis_label: Bazel label of the manual analysis target

    Returns:
        JSON string in lobster-act-trace format (version 3)
    """

    sorted_requirements = [f"req {requirement}" for requirement in sorted(requirements)]
    analysis_status = "ok" if analysis_passed else "fail"
    items: list[dict[str, Any]] = [
        {
            "tag": f"manualanalysis {analysis_label}",
            "location": {
                "kind": "file",
                "file": results_file_path,
                "line": None,
                "column": None,
            },
            "name": f"Manual verification: {analysis_label}",
            "refs": sorted_requirements,
            "just_up": [],
            "just_down": [],
            "just_global": [],
            "framework": "manual_analysis",
            "kind": "Manual Analysis Run",
            "status": analysis_status,
        }
    ]

    # Create LOBSTER-act-trace document
    lobster_doc = {
        "schema": "lobster-act-trace",
        "version": 3,
        "generator": "manual_analysis",
        "data": items,
    }
    
    return json.dumps(lobster_doc, indent=2) + "\n"


def write_lobster_file(
    requirements: list[str],
    analysis_passed: bool,
    results_file_path: str,
    analysis_label: str,
    output_path: Path,
) -> None:
    """Generate and write LOBSTER JSON file for manual verification.

    Args:
        requirements: List of requirement identifiers
        analysis_passed: True if the assertion passed
        results_file_path: Path to the manual analysis results file
        analysis_label: Bazel label of the manual analysis target
        output_path: Path where the .lobster file should be written
    """
    json_content = generate_lobster_json(
        requirements,
        analysis_passed,
        results_file_path,
        analysis_label,
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json_content, encoding="utf-8")

