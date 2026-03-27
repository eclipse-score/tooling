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
"""CLI for the interactive manual-analysis runner."""

from __future__ import annotations

import argparse
import os
import sys

from manual_analysis.common import resolve_path
from manual_analysis.interactive_runner_flow import run_analysis
from manual_analysis.interactive_runner_prefill import _PrefillState
from manual_analysis.interactive_runner_runtime import (
    _install_signal_handlers,
    _interrupt_exit_code,
)
from manual_analysis.interactive_runner_steps import AnalysisFailedError
from manual_analysis.interactive_runner_ui_split import _SplitPaneUI
from manual_analysis.yaml_schema import load_analysis


def _create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run a manual analysis interactively")
    parser.add_argument(
        "--analysis",
        default=os.environ.get("MANUAL_ANALYSIS_YAML"),
        help="Path to analysis YAML file",
    )
    parser.add_argument(
        "--results-file",
        default=os.environ.get("MANUAL_ANALYSIS_RESULTS_FILE"),
        help="Path to JSON file where captured results are stored",
    )
    parser.add_argument(
        "--no-prefill-from-last-run",
        action="store_true",
        help="Do not prefill inputs from the previous results file before executing steps.",
    )
    return parser


def main(argv: list[str] | None = None) -> None:
    _install_signal_handlers()
    parser = _create_parser()
    args = parser.parse_args(argv)

    if not args.analysis:
        parser.error("--analysis is required (or set MANUAL_ANALYSIS_YAML)")
    if not args.results_file:
        parser.error("--results-file is required (or set MANUAL_ANALYSIS_RESULTS_FILE)")

    analysis_path = resolve_path(args.analysis)
    if not analysis_path.exists():
        parser.error(f"Analysis file does not exist: {analysis_path}")

    results_path = resolve_path(args.results_file)
    steps, _ = load_analysis(analysis_path)
    prefill = (
        _PrefillState.load(results_path) if not args.no_prefill_from_last_run else None
    )
    ui = _SplitPaneUI()

    try:
        run_analysis(
            steps,
            ui,
            analysis_path=analysis_path,
            results_path=results_path,
            prefill=prefill,
        )
    except AnalysisFailedError as err:
        print(str(err), file=sys.stderr)
        sys.exit(1)
    except KeyboardInterrupt:
        print("Manual analysis interrupted by signal/user.", file=sys.stderr)
        sys.exit(_interrupt_exit_code())
    except EOFError:
        print("Manual analysis input stream closed/interrupted.", file=sys.stderr)
        sys.exit(_interrupt_exit_code())
