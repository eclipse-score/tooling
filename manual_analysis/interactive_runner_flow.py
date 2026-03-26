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
"""Analysis execution flow for interactive manual analyses."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Callable

from manual_analysis.interactive_runner_prefill import _PrefillState
from manual_analysis.interactive_runner_runtime import _workspace_root
from manual_analysis.interactive_runner_steps import (
    AnalysisFailedError,
    _execute_step,
)
from manual_analysis.interactive_runner_ui import _make_ui
from manual_analysis.yaml_schema import Step


def run_analysis(
    steps: list[Step],
    input_fn: Callable[[str], str] | None = None,
    *,
    analysis_path: Path,
    results_path: Path,
    prefill: _PrefillState | None = None,
) -> list[dict]:
    def _write_results() -> None:
        if results_path is None:
            return
        payload = {
            "analysis": str(analysis_path.resolve().relative_to(_workspace_root())),
            "results": results,
        }
        results_path.parent.mkdir(parents=True, exist_ok=True)
        results_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")

    ui = _make_ui(input_fn)
    results: list[dict] = []
    try:
        for step in steps:
            _execute_step(step, ui, results, prefill=prefill)
            _write_results()
    except (KeyboardInterrupt, EOFError):
        results.append(
            {
                "type": "assertion",
                "description": "Final assertion (partial run)",
                "answer": "No",
                "passed": False,
                "partial_run": True,
            }
        )
        _write_results()
        raise
    except AnalysisFailedError:
        _write_results()
        raise
    return results
