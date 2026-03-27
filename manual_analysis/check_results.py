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
import json
from pathlib import Path
from typing import Any


def _load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def evaluate_results_file(results_path: Path) -> tuple[bool, str | None]:
    if not results_path.exists():
        return False, f"results file not found: {results_path}"

    try:
        payload = _load(results_path)
    except OSError as exc:
        return False, f"Could not read results file: {exc}"
    except json.JSONDecodeError as exc:
        return False, f"Results file is not valid JSON: {exc}"

    if not isinstance(payload, dict):
        return False, "Manual analysis results are missing or empty."

    results = payload.get("results")
    if not isinstance(results, list) or not results:
        return False, "Manual analysis results are missing or empty."

    last = results[-1]
    if not isinstance(last, dict) or last.get("type") != "assertion":
        return False, "Manual analysis does not end with an assertion."

    if last.get("passed") is True:
        return True, None
    return False, "Final manual analysis assertion is not positive."
